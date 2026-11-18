//! Word export (specification, "Word output rules"). A scale is written as a native .docx, so Word is not needed to create
//! it. The scale title is a heading; instructions, anchors and items have their own named styles, so a whole study can
//! be restyled in one step. The box grid and the semantic differential are Word tables whose header row repeats on a new
//! page; nothing is lined up with tabs or spaces. The anchor row is kept on the page of the first item, and no item row
//! splits across pages.

use prismsurvey_core::instructions::box_grid_key;
use prismsurvey_core::scale::{computed_range, recode_table, reversed_numbers};
use prismsurvey_core::*;
use std::io::Write;

pub mod html;
pub mod study;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Copy {
    /// Title, instructions, anchors and items, ready to paste into a questionnaire. No reverse marks or scoring.
    #[default]
    Respondent,
    /// Everything above plus (R) marks, the recode table, subscales, the scoring rule, the score range and sources.
    Researcher,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportOptions {
    pub copy: Copy,
    pub font: String,
    pub size_pt: u32,
    /// Print the scale title. When false and `neutral_label` is set, the label prints instead ("Section 3").
    pub show_title: bool,
    pub neutral_label: Option<String>,
    /// Added to every item number, when numbering runs on through a study.
    pub number_offset: u32,
    /// Where sources print, in place of the scale's own setting (a study decides for all its scales).
    pub sources: Option<SourcePlacement>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            copy: Copy::Respondent,
            font: "Times New Roman".into(),
            size_pt: 12,
            show_title: true,
            neutral_label: None,
            number_offset: 0,
            sources: None,
        }
    }
}

/// The finished file and anything the person should be told about it.
#[derive(Debug, Clone)]
pub struct Exported {
    pub bytes: Vec<u8>,
    pub notes: Vec<String>,
}

pub fn export_scale(scale: &Scale, opts: &ExportOptions) -> Exported {
    let mut notes = Vec::new();
    if opts.copy == Copy::Respondent
        && matches!(
            scale.licence.status,
            LicenceStatus::Unknown | LicenceStatus::LocalOnly
        )
        && scale.source.is_some()
    {
        notes.push(
            "Reproducing a published measure may need the rights holder's permission.".to_string(),
        );
    }
    let body = scale_body(scale, opts);
    Exported {
        bytes: package(&body, opts),
        notes,
    }
}

// ---------------------------------------------------------------- XML pieces

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            // Characters XML 1.0 forbids are dropped.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => o.push(c),
        }
    }
    o
}

/// A run of text. Tabs become Word tabs and line breaks Word breaks.
fn run(text: &str, bold: bool) -> String {
    let rpr = if bold { "<w:rPr><w:b/></w:rPr>" } else { "" };
    let mut out = String::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push_str(&format!("<w:r>{rpr}<w:br/></w:r>"));
        }
        for (k, part) in line.split('\t').enumerate() {
            if k > 0 {
                out.push_str(&format!("<w:r>{rpr}<w:tab/></w:r>"));
            }
            if !part.is_empty() {
                out.push_str(&format!(
                    "<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
                    esc(part)
                ));
            }
        }
    }
    out
}

pub(crate) struct P<'a> {
    style: &'a str,
    runs: String,
    keep_next: bool,
    align: Option<&'a str>,
}

impl<'a> P<'a> {
    fn new(style: &'a str) -> Self {
        P {
            style,
            runs: String::new(),
            keep_next: false,
            align: None,
        }
    }
    fn text(mut self, t: &str) -> Self {
        self.runs.push_str(&run(t, false));
        self
    }
    fn bold(mut self, t: &str) -> Self {
        self.runs.push_str(&run(t, true));
        self
    }
    fn keep_next(mut self) -> Self {
        self.keep_next = true;
        self
    }
    fn align(mut self, a: &'a str) -> Self {
        self.align = Some(a);
        self
    }
    fn xml(&self) -> String {
        let mut ppr = format!("<w:pStyle w:val=\"{}\"/>", self.style);
        if self.keep_next {
            ppr.push_str("<w:keepNext/>");
        }
        if let Some(a) = self.align {
            ppr.push_str(&format!("<w:jc w:val=\"{a}\"/>"));
        }
        format!("<w:p><w:pPr>{ppr}</w:pPr>{}</w:p>", self.runs)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Border {
    None,
    All,
}

struct TCell {
    width: u32,
    content: String,
    border: Border,
}

/// A table row. `header` rows repeat on each page; every row is kept whole.
fn row(cells: &[TCell], header: bool) -> String {
    let mut trpr = String::from("<w:cantSplit/>");
    if header {
        trpr.push_str("<w:tblHeader/>");
    }
    let mut out = format!("<w:tr><w:trPr>{trpr}</w:trPr>");
    for c in cells {
        let borders = match c.border {
            Border::All => "<w:tcBorders><w:top w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"000000\"/><w:left w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"000000\"/><w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"000000\"/><w:right w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"000000\"/></w:tcBorders>",
            Border::None => "",
        };
        out.push_str(&format!(
            "<w:tc><w:tcPr><w:tcW w:w=\"{}\" w:type=\"dxa\"/>{borders}<w:vAlign w:val=\"center\"/></w:tcPr>{}</w:tc>",
            c.width, c.content
        ));
    }
    out.push_str("</w:tr>");
    out
}

/// A row across the whole table holding an item's lead-in, kept with the item below it.
fn lead_row(text: &str, widths: &[u32]) -> String {
    let total: u32 = widths.iter().sum();
    format!(
        "<w:tr><w:trPr><w:cantSplit/></w:trPr><w:tc><w:tcPr><w:tcW w:w=\"{total}\" w:type=\"dxa\"/><w:gridSpan w:val=\"{}\"/></w:tcPr>{}</w:tc></w:tr>",
        widths.len(),
        P::new("PSInstructions").text(text).keep_next().xml()
    )
}

fn table(widths: &[u32], rows: &[String], style: &str) -> String {
    let grid: String = widths
        .iter()
        .map(|w| format!("<w:gridCol w:w=\"{w}\"/>"))
        .collect();
    let total: u32 = widths.iter().sum();
    format!(
        "<w:tbl><w:tblPr><w:tblStyle w:val=\"{style}\"/><w:tblW w:w=\"{total}\" w:type=\"dxa\"/><w:tblLayout w:type=\"fixed\"/><w:tblLook w:val=\"0420\" w:firstRow=\"1\" w:lastRow=\"0\" w:firstColumn=\"0\" w:lastColumn=\"0\" w:noHBand=\"1\" w:noVBand=\"1\"/></w:tblPr><w:tblGrid>{grid}</w:tblGrid>{}</w:tbl>",
        rows.concat()
    )
}

/// Text width of a page with one-inch margins, in twentieths of a point.
const TEXT_WIDTH: u32 = 9360;

fn cell_p(text: &str, align: &str, keep_next: bool, bold: bool) -> String {
    let p = P::new("PSTableText").align(align);
    let p = if bold { p.bold(text) } else { p.text(text) };
    let p = if keep_next { p.keep_next() } else { p };
    p.xml()
}

// ---------------------------------------------------------------- the scale

fn point_label(scale: &Scale, i: usize) -> String {
    scale
        .response
        .labels
        .get(i)
        .cloned()
        .flatten()
        .unwrap_or_default()
}

/// The body XML of one scale (paragraphs and tables), so a study can place several scales in one document.
pub fn scale_body(scale: &Scale, opts: &ExportOptions) -> String {
    let researcher = opts.copy == Copy::Researcher;
    let mut out = String::new();
    // Title.
    let heading = if opts.show_title {
        Some(scale.title.clone())
    } else {
        opts.neutral_label.clone()
    };
    if let Some(h) = heading.filter(|h| !h.trim().is_empty()) {
        out.push_str(&P::new("Heading1").text(&h).keep_next().xml());
    }
    // Instructions.
    if !scale.instructions.text.trim().is_empty() {
        out.push_str(
            &P::new("PSInstructions")
                .text(&scale.instructions.text)
                .keep_next()
                .xml(),
        );
    }
    let n = scale.response.points as usize;
    let values = &scale.response.values;
    let reversed = |it: &Item| researcher && it.reverse.is_reversed();
    let item_text = |it: &Item| {
        if reversed(it) {
            format!("{} (R)", it.text)
        } else {
            it.text.clone()
        }
    };
    match scale.format {
        Format::LikertBlankLine => {
            // The anchor row: labels over numbers, a borderless table kept with the first item.
            let w = TEXT_WIDTH / n as u32;
            let widths = vec![w; n];
            let labels: Vec<TCell> = (0..n)
                .map(|i| TCell {
                    width: w,
                    content: cell_p(&point_label(scale, i), "center", true, false),
                    border: Border::None,
                })
                .collect();
            let nums: Vec<TCell> = (0..n)
                .map(|i| TCell {
                    width: w,
                    content: cell_p(&values[i].to_string(), "center", true, false),
                    border: Border::None,
                })
                .collect();
            out.push_str(&table(
                &widths,
                &[row(&labels, false), row(&nums, false)],
                "PSScaleTable",
            ));
            for it in &scale.items {
                if let Some(l) = it.lead_in.as_deref().filter(|l| !l.trim().is_empty()) {
                    out.push_str(&P::new("PSInstructions").text(l).keep_next().xml());
                }
                out.push_str(
                    &P::new("PSItem")
                        .text(&format!(
                            "_____{}.\t{}",
                            it.number + opts.number_offset,
                            item_text(it)
                        ))
                        .xml(),
                );
            }
        }
        Format::LikertBoxGrid => {
            let key = box_grid_key(&scale.response);
            if !key.is_empty() {
                out.push_str(&P::new("PSInstructions").text(&key).keep_next().xml());
            }
            let box_w = 560u32;
            let num_w = 560u32;
            let text_w = TEXT_WIDTH
                .saturating_sub(num_w + box_w * n as u32)
                .max(2400);
            let mut widths = vec![num_w, text_w];
            widths.extend(std::iter::repeat_n(box_w, n));
            let mut header = vec![
                TCell {
                    width: num_w,
                    content: cell_p("", "left", true, false),
                    border: Border::None,
                },
                TCell {
                    width: text_w,
                    content: cell_p("", "left", true, false),
                    border: Border::None,
                },
            ];
            for (i, value) in values.iter().enumerate().take(n) {
                let a = scale
                    .response
                    .abbreviations
                    .get(i)
                    .cloned()
                    .filter(|a| !a.is_empty())
                    .unwrap_or_else(|| value.to_string());
                header.push(TCell {
                    width: box_w,
                    content: cell_p(&a, "center", true, true),
                    border: Border::None,
                });
            }
            let mut rows = vec![row(&header, true)];
            for it in &scale.items {
                if let Some(l) = it.lead_in.as_deref().filter(|l| !l.trim().is_empty()) {
                    rows.push(lead_row(l, &widths));
                }
                let mut cells = vec![
                    TCell {
                        width: num_w,
                        content: cell_p(
                            &format!("{}.", it.number + opts.number_offset),
                            "left",
                            false,
                            false,
                        ),
                        border: Border::None,
                    },
                    TCell {
                        width: text_w,
                        content: cell_p(&item_text(it), "left", false, false),
                        border: Border::None,
                    },
                ];
                for _ in 0..n {
                    cells.push(TCell {
                        width: box_w,
                        content: cell_p("", "center", false, false),
                        border: Border::All,
                    });
                }
                rows.push(row(&cells, false));
            }
            out.push_str(&table(&widths, &rows, "PSScaleTable"));
        }
        Format::SemanticDifferential => {
            let num_w = 560u32;
            let pt_w = 440u32;
            let adj_w = (TEXT_WIDTH - num_w - pt_w * n as u32) / 2;
            let mut widths = vec![num_w, adj_w];
            widths.extend(std::iter::repeat_n(pt_w, n));
            widths.push(adj_w);
            // The header row carries any point labels; it repeats on each page.
            let mut header = vec![
                TCell {
                    width: num_w,
                    content: cell_p("", "left", true, false),
                    border: Border::None,
                },
                TCell {
                    width: adj_w,
                    content: cell_p("", "right", true, false),
                    border: Border::None,
                },
            ];
            for i in 0..n {
                header.push(TCell {
                    width: pt_w,
                    content: cell_p(&point_label(scale, i), "center", true, false),
                    border: Border::None,
                });
            }
            header.push(TCell {
                width: adj_w,
                content: cell_p("", "left", true, false),
                border: Border::None,
            });
            let mut rows = vec![row(&header, true)];
            for it in &scale.items {
                if let Some(l) = it.lead_in.as_deref().filter(|l| !l.trim().is_empty()) {
                    rows.push(lead_row(l, &widths));
                }
                let left = it.left.clone().unwrap_or_default();
                let right = it.right.clone().unwrap_or_default();
                let num = if reversed(it) {
                    format!("{}. (R)", it.number + opts.number_offset)
                } else {
                    format!("{}.", it.number + opts.number_offset)
                };
                let mut cells = vec![
                    TCell {
                        width: num_w,
                        content: cell_p(&num, "left", false, false),
                        border: Border::None,
                    },
                    TCell {
                        width: adj_w,
                        content: cell_p(&left, "right", false, false),
                        border: Border::None,
                    },
                ];
                for v in values.iter().take(n) {
                    cells.push(TCell {
                        width: pt_w,
                        content: cell_p(&v.to_string(), "center", false, false),
                        border: Border::None,
                    });
                }
                cells.push(TCell {
                    width: adj_w,
                    content: cell_p(&right, "left", false, false),
                    border: Border::None,
                });
                rows.push(row(&cells, false));
            }
            out.push_str(&table(&widths, &rows, "PSScaleTable"));
        }
    }
    if researcher {
        out.push_str(&scoring_key(scale, opts.number_offset));
        out.push_str(&interpretation(scale));
        out.push_str(&variant_record(scale, opts.number_offset));
        out.push_str(&source_notes(scale));
    }
    if opts.sources.unwrap_or(scale.source_placement) == SourcePlacement::UnderScale {
        out.push_str(&sources(scale));
    }
    if researcher {
        out.push_str(&open_findings(scale));
    }
    out
}

/// How the source says to read the scores, as it gives them.
fn interpretation(scale: &Scale) -> String {
    let lines: Vec<&String> = scale
        .scoring
        .interpretation
        .iter()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    let mut out = P::new("PSSourceLabel")
        .bold("Reading the scores:")
        .keep_next()
        .xml();
    for l in lines {
        out.push_str(&P::new("PSScoring").text(l).xml());
    }
    out
}

/// For an adapted variant: what it is adapted from, the change of target, the caution that reliability and validity do
/// not carry over, and the item map with each original wording beside the new one (specification, "What the
/// researcher copy says").
fn variant_record(scale: &Scale, offset: u32) -> String {
    let Some(v) = &scale.variant_of else {
        return String::new();
    };
    let mut out = P::new("PSSourceLabel")
        .bold("Adapted variant:")
        .keep_next()
        .xml();
    let mut line = format!(
        "Adapted from {} (its saved version {}). The target is changed from \"{}\" to \"{}\"",
        v.parent_title.trim(),
        v.parent_version,
        v.old_target.trim(),
        v.new_target.trim()
    );
    if let Some(c) = v.context.as_deref().filter(|c| !c.trim().is_empty()) {
        line.push_str(&format!(" ({})", c.trim().trim_end_matches('.')));
    }
    line.push('.');
    out.push_str(&P::new("PSScoring").text(&line).xml());
    out.push_str(
        &P::new("PSScoring")
            .text("Reliability and validity reported for the original do not carry over to this adapted version; check them in the new sample.")
            .xml(),
    );
    let rows: Vec<String> = v
        .item_map
        .iter()
        .filter_map(|e| {
            let now = scale.items.iter().find(|i| i.number == e.number)?;
            let now_text = prismsurvey_core::variant::original_wording(now);
            let n = e.number + offset;
            if let Some(r) = &e.flagged {
                Some(format!(
                    "{n}. Not reworded, kept as the original item {}: {} ({r})",
                    e.original_number, e.original_text
                ))
            } else if now_text != e.original_text || e.number != e.original_number {
                Some(format!(
                    "{n}. Original item {}: {}",
                    e.original_number, e.original_text
                ))
            } else {
                None
            }
        })
        .collect();
    if !rows.is_empty() {
        out.push_str(
            &P::new("PSSourceLabel")
                .bold("Item map (the original's wording):")
                .keep_next()
                .xml(),
        );
        for r in rows {
            out.push_str(&P::new("PSScoring").text(&r).xml());
        }
    }
    out
}

/// The scale's source notes: findings kept as published, and corrections made to the published measure, so the key
/// says which (specification, "Source notes: kept as published"). Never in the respondent copy.
fn source_notes(scale: &Scale) -> String {
    if scale.source_notes.is_empty() {
        return String::new();
    }
    let mut out = P::new("PSSourceLabel")
        .bold("Source notes:")
        .keep_next()
        .xml();
    for n in &scale.source_notes {
        out.push_str(
            &P::new("PSScoring")
                .text(&prismsurvey_core::selfcheck::note_text(n))
                .xml(),
        );
    }
    out
}

/// The self-check findings still open (neither accepted nor dismissed), so they travel with the researcher copy.
fn open_findings(scale: &Scale) -> String {
    let open = prismsurvey_core::selfcheck::open_findings(scale);
    if open.is_empty() {
        return String::new();
    }
    let mut out = P::new("PSSourceLabel")
        .bold("Self-check findings still open:")
        .keep_next()
        .xml();
    for f in open {
        out.push_str(
            &P::new("PSScoring")
                .text(&format!("{}: {}", f.check_label, f.message))
                .xml(),
        );
    }
    out
}

fn list_numbers(n: &[u32]) -> String {
    match n.len() {
        0 => String::new(),
        1 => n[0].to_string(),
        _ => {
            let s: Vec<String> = n.iter().map(|x| x.to_string()).collect();
            format!("{}, and {}", s[..s.len() - 1].join(", "), s[s.len() - 1])
        }
    }
}

fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{}", x as i64)
    } else {
        format!("{x:.2}")
    }
}

/// The researcher copy's scoring section: recode list and table, subscales, the total and the score range.
fn scoring_key(scale: &Scale, offset: u32) -> String {
    let mut out = P::new("PSScoring").bold("Scoring").keep_next().xml();
    let rev: Vec<u32> = reversed_numbers(scale)
        .into_iter()
        .map(|n| n + offset)
        .collect();
    if offset > 0 {
        out.push_str(
            &P::new("PSScoring")
                .text("Item numbers below are the study's numbers.")
                .xml(),
        );
    }
    if rev.is_empty() {
        out.push_str(
            &P::new("PSScoring")
                .text("No items are reverse scored.")
                .xml(),
        );
    } else {
        out.push_str(
            &P::new("PSScoring")
                .text(&format!("Recode items {} as follows:", list_numbers(&rev)))
                .keep_next()
                .xml(),
        );
        let pairs = recode_table(scale);
        let rows: Vec<String> = pairs
            .iter()
            .enumerate()
            .map(|(k, (a, b))| {
                let last = k + 1 == pairs.len();
                row(
                    &[
                        TCell {
                            width: 600,
                            content: cell_p(&a.to_string(), "center", !last, false),
                            border: Border::None,
                        },
                        TCell {
                            width: 600,
                            content: cell_p("=", "center", !last, false),
                            border: Border::None,
                        },
                        TCell {
                            width: 600,
                            content: cell_p(&b.to_string(), "center", !last, false),
                            border: Border::None,
                        },
                    ],
                    false,
                )
            })
            .collect();
        out.push_str(&table(&[600, 600, 600], &rows, "PSScaleTable"));
    }
    let count = scale.items.len() as u32;
    let method = |m: ScoreMethod| match m {
        ScoreMethod::Sum => "Add",
        ScoreMethod::Mean => "Average",
    };
    for s in &scale.subscales {
        let items: Vec<u32> = s
            .items
            .resolve(count)
            .into_iter()
            .map(|n| n + offset)
            .collect();
        out.push_str(
            &P::new("PSScoring")
                .text(&format!(
                    "{}: {} items {}.",
                    s.name,
                    method(scale.scoring.subscale_method),
                    list_numbers(&items)
                ))
                .xml(),
        );
    }
    if scale.scoring.has_total {
        let name = scale
            .scoring
            .total_name
            .clone()
            .unwrap_or_else(|| "Total score".into());
        let how = match scale.scoring.total_from {
            TotalFrom::Items => format!(
                "{} all items, after recoding.",
                method(scale.scoring.total_method)
            ),
            TotalFrom::Subscales => format!(
                "{} the subscale scores.",
                method(scale.scoring.total_method)
            ),
        };
        out.push_str(
            &P::new("PSScoring")
                .text(&format!("{}: {how}", capitalize(&name)))
                .xml(),
        );
        if let Some((a, b)) = computed_range(scale) {
            out.push_str(
                &P::new("PSScoring")
                    .text(&format!(
                        "Scores should range between {} and {}.",
                        fmt_num(a),
                        fmt_num(b)
                    ))
                    .xml(),
            );
        }
    }
    out
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

pub fn sources(scale: &Scale) -> String {
    let mut out = String::new();
    if let Some(src) = &scale.source {
        out.push_str(&P::new("PSSourceLabel").bold("Source:").keep_next().xml());
        out.push_str(&P::new("PSSource").text(&src.apa).xml());
    }
    if let Some(src) = &scale.adapted_from {
        out.push_str(
            &P::new("PSSourceLabel")
                .bold("Adapted from:")
                .keep_next()
                .xml(),
        );
        out.push_str(&P::new("PSSource").text(&src.apa).xml());
    }
    out
}

// ---------------------------------------------------------------- the package

fn styles_xml(opts: &ExportOptions) -> String {
    let font = esc(&opts.font);
    let sz = opts.size_pt * 2;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="{font}" w:hAnsi="{font}" w:eastAsia="{font}" w:cs="{font}"/><w:sz w:val="{sz}"/><w:szCs w:val="{sz}"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="PSInstructions"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="240" w:after="240"/><w:jc w:val="center"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:bCs/></w:rPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSInstructions"><w:name w:val="Instructions"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:after="240"/></w:pPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSItem"><w:name w:val="Item"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepLines/><w:tabs><w:tab w:val="left" w:pos="1080"/></w:tabs><w:spacing w:before="120" w:after="120"/><w:ind w:left="1080" w:hanging="1080"/></w:pPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSTableText"><w:name w:val="Scale Table Text"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="40" w:after="40"/></w:pPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSScoring"><w:name w:val="Scoring"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:before="120" w:after="60"/></w:pPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSTransition"><w:name w:val="Transition"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="240"/></w:pPr><w:rPr><w:i/></w:rPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSDemographic"><w:name w:val="Demographic Question"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepLines/><w:spacing w:before="120" w:after="120"/></w:pPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSDemographicChoice"><w:name w:val="Demographic Choice"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepLines/><w:spacing w:before="0" w:after="40"/><w:ind w:left="360"/></w:pPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSSourceLabel"><w:name w:val="Source Label"/><w:basedOn w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="60"/></w:pPr></w:style>
<w:style w:type="paragraph" w:customStyle="1" w:styleId="PSSource"><w:name w:val="Source"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="120"/><w:ind w:left="720" w:hanging="720"/></w:pPr></w:style>
<w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:tblPr><w:tblInd w:w="0" w:type="dxa"/><w:tblCellMar><w:top w:w="0" w:type="dxa"/><w:left w:w="108" w:type="dxa"/><w:bottom w:w="0" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>
<w:style w:type="table" w:customStyle="1" w:styleId="PSScaleTable"><w:name w:val="Scale Table"/><w:basedOn w:val="TableNormal"/><w:tblPr><w:jc w:val="center"/></w:tblPr></w:style>
</w:styles>"#
    )
}

fn document_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#
    )
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/></Relationships>"#;

const SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:defaultTabStop w:val="720"/><w:characterSpacingControl w:val="doNotCompress"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#;

const APP: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Prism Survey</Application></Properties>"#;

fn core_xml() -> String {
    let now = prismsurvey_core::pscale::now_rfc3339();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dcterms:created xsi:type="dcterms:W3CDTF">{now}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">{now}</dcterms:modified></cp:coreProperties>"#
    )
}

/// Wraps body XML in a complete .docx package.
pub fn package(body: &str, opts: &ExportOptions) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let parts: [(&str, String); 7] = [
            ("[Content_Types].xml", CONTENT_TYPES.to_string()),
            ("_rels/.rels", ROOT_RELS.to_string()),
            ("word/_rels/document.xml.rels", DOC_RELS.to_string()),
            ("word/document.xml", document_xml(body)),
            ("word/styles.xml", styles_xml(opts)),
            ("word/settings.xml", SETTINGS.to_string()),
            ("docProps/core.xml", core_xml()),
        ];
        for (name, content) in parts.iter() {
            zip.start_file(*name, o).expect("zip entry");
            zip.write_all(content.as_bytes()).expect("zip write");
        }
        zip.start_file("docProps/app.xml", o).expect("zip entry");
        zip.write_all(APP.as_bytes()).expect("zip write");
        zip.finish().expect("zip finish");
    }
    buf.into_inner()
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;
    use prismsurvey_core::scale::new_blank;
    use prismsurvey_import::document::{read_docx, Block};

    pub fn sample(format: Format) -> Scale {
        let mut s = new_blank(format);
        s.title = "Pond Mood Scale".into();
        s.items.truncate(3);
        for (k, it) in s.items.iter_mut().enumerate() {
            it.text = format!("Statement number {} & more.", k + 1);
            if format == Format::SemanticDifferential {
                it.text.clear();
                it.left = Some(format!("calm{k}"));
                it.right = Some(format!("stormy{k}"));
            }
        }
        s.items[1].reverse = ReverseFlag {
            state: ReverseState::Confirmed,
            reason: None,
        };
        s.source = Some(Citation {
            apa: "Heron, G. (2020). Ponds. Journal of Water, 1, 1-2.".into(),
            doi: None,
            reference_manager_id: None,
        });
        s
    }

    pub fn paragraphs(bytes: &[u8]) -> Vec<String> {
        read_docx(bytes)
            .unwrap()
            .blocks
            .into_iter()
            .flat_map(|b| match b {
                Block::Para(p) => vec![p.text],
                Block::Table(rows) => rows
                    .into_iter()
                    .map(|r| r.into_iter().map(|c| c.text).collect::<Vec<_>>().join("|"))
                    .collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::{paragraphs, sample};
    use super::*;

    #[test]
    fn blank_line_respondent_copy_reads_back() {
        let out = export_scale(&sample(Format::LikertBlankLine), &ExportOptions::default());
        let p = paragraphs(&out.bytes);
        assert_eq!(p[0], "Pond Mood Scale");
        assert!(p.contains(&"Strongly Disagree|Disagree|Neutral|Agree|Strongly Agree".to_string()));
        assert!(p.contains(&"1|2|3|4|5".to_string()));
        assert!(
            p.contains(&"_____2.\tStatement number 2 & more.".to_string()),
            "{p:?}"
        );
        assert!(
            !p.iter().any(|x| x.contains("(R)")),
            "no reverse marks on the respondent copy"
        );
        assert!(p.iter().any(|x| x.starts_with("Heron, G.")));
        assert_eq!(
            out.notes.len(),
            1,
            "a permission note for a local-only published measure"
        );
    }

    #[test]
    fn researcher_copy_has_the_scoring_key() {
        let opts = ExportOptions {
            copy: Copy::Researcher,
            ..Default::default()
        };
        let p = paragraphs(&export_scale(&sample(Format::LikertBlankLine), &opts).bytes);
        assert!(p.iter().any(|x| x.ends_with("more. (R)")));
        assert!(p.contains(&"Recode items 2 as follows:".to_string()));
        assert!(p.contains(&"1|=|5".to_string()));
        assert!(
            p.contains(&"Scores should range between 3 and 15.".to_string()),
            "{p:?}"
        );
    }

    #[test]
    fn researcher_copy_lists_open_findings_only() {
        let mut s = sample(Format::LikertBlankLine);
        s.items[2].text = s.items[0].text.clone();
        let opts = ExportOptions {
            copy: Copy::Researcher,
            ..Default::default()
        };
        let p = paragraphs(&export_scale(&s, &opts).bytes);
        assert!(
            p.contains(&"Self-check findings still open:".to_string()),
            "{p:?}"
        );
        assert!(p
            .iter()
            .any(|x| x.starts_with("Duplicate items: Item 3 repeats item 1")));
        prismsurvey_core::selfcheck::decide(
            &mut s,
            "duplicate:1:3",
            Some(DecisionStatus::Dismissed),
            "On purpose.",
            "t",
        )
        .unwrap();
        let p = paragraphs(&export_scale(&s, &opts).bytes);
        assert!(
            !p.iter().any(|x| x.starts_with("Duplicate items")),
            "a dismissed finding does not print"
        );
        let p = paragraphs(&export_scale(&s, &ExportOptions::default()).bytes);
        assert!(
            !p.iter().any(|x| x.contains("Self-check")),
            "never in the respondent copy"
        );
    }

    #[test]
    fn lead_ins_print_above_their_items_and_bands_in_the_key() {
        for f in [Format::LikertBlankLine, Format::LikertBoxGrid] {
            let mut s = sample(f);
            s.items[1].lead_in = Some("Now answer as a coach would.".into());
            s.scoring.interpretation = vec!["3-5 Very calm".into()];
            let p = paragraphs(&export_scale(&s, &ExportOptions::default()).bytes);
            let at = p
                .iter()
                .position(|x| x.contains("Now answer as a coach would."))
                .expect("the lead-in prints");
            assert!(p[at + 1].contains("2."), "{f:?}: {p:?}");
            assert!(
                !p.iter().any(|x| x.contains("Very calm")),
                "bands are for the researcher copy"
            );
            let researcher = ExportOptions {
                copy: Copy::Researcher,
                ..Default::default()
            };
            let p = paragraphs(&export_scale(&s, &researcher).bytes);
            assert!(
                p.contains(&"Reading the scores:".to_string())
                    && p.contains(&"3-5 Very calm".to_string())
            );
        }
    }

    #[test]
    fn a_variant_names_its_original_in_the_researcher_copy_only() {
        let mut parent = sample(Format::LikertBlankLine);
        parent.title = "Tutor Scale".into();
        parent.items[0].text = "I trust my tutor.".into();
        let req = prismsurvey_core::variant::Request {
            old_target: "tutor".into(),
            new_target: "coach".into(),
            context: Some("a sports setting".into()),
            replacements: vec![],
        };
        let v = prismsurvey_core::variant::draft(&parent, &req)
            .unwrap()
            .scale;
        let researcher = ExportOptions {
            copy: Copy::Researcher,
            ..Default::default()
        };
        let p = paragraphs(&export_scale(&v, &researcher).bytes);
        assert!(p.contains(&"Adapted variant:".to_string()), "{p:?}");
        assert!(p.iter().any(|x| x.starts_with("Adapted from Tutor Scale (its saved version 0). The target is changed from \"tutor\" to \"coach\" (a sports setting).")));
        assert!(p.iter().any(|x| x
            .starts_with("Reliability and validity reported for the original do not carry over")));
        assert!(
            p.contains(&"1. Original item 1: I trust my tutor.".to_string()),
            "{p:?}"
        );
        let p = paragraphs(&export_scale(&v, &ExportOptions::default()).bytes);
        assert!(!p
            .iter()
            .any(|x| x.contains("Adapted variant") || x.contains("Original item")));
    }

    #[test]
    fn source_notes_print_in_the_researcher_copy_only() {
        let mut s = sample(Format::LikertBlankLine);
        s.items[2].text = s.items[0].text.clone();
        prismsurvey_core::selfcheck::keep_as_published(
            &mut s,
            "duplicate:1:3",
            "the original prints item 1 twice",
            Some("12"),
            "t",
        )
        .unwrap();
        let researcher = ExportOptions {
            copy: Copy::Researcher,
            ..Default::default()
        };
        let p = paragraphs(&export_scale(&s, &researcher).bytes);
        assert!(p.contains(&"Source notes:".to_string()), "{p:?}");
        assert!(p.contains(
            &"As published (page 12): the original prints item 1 twice. Kept as printed."
                .to_string()
        ));
        assert!(
            !p.iter().any(|x| x.starts_with("Duplicate items")),
            "a finding kept as published is not listed as open"
        );
        let p = paragraphs(&export_scale(&s, &ExportOptions::default()).bytes);
        assert!(!p
            .iter()
            .any(|x| x.contains("Source notes") || x.contains("As published")));
    }

    #[test]
    fn box_grid_is_a_table_with_a_repeating_header() {
        let out = export_scale(&sample(Format::LikertBoxGrid), &ExportOptions::default());
        let p = paragraphs(&out.bytes);
        assert!(p.contains(&"||SD|D|N|A|SA".to_string()), "{p:?}");
        assert!(p.contains(&"2.|Statement number 2 & more.|||||".to_string()));
        let xml = String::from_utf8_lossy(&unzip(&out.bytes, "word/document.xml")).into_owned();
        assert!(xml.contains("<w:tblHeader/>"));
        assert!(xml.contains("<w:cantSplit/>"));
        assert!(
            !xml.contains("<w:tab/>"),
            "nothing in a grid is aligned with tabs"
        );
    }

    #[test]
    fn semantic_differential_rows() {
        let p = paragraphs(
            &export_scale(
                &sample(Format::SemanticDifferential),
                &ExportOptions::default(),
            )
            .bytes,
        );
        assert!(
            p.contains(&"1.|calm0|1|2|3|4|5|6|7|stormy0".to_string()),
            "{p:?}"
        );
    }

    #[test]
    fn styles_are_real_and_named() {
        let out = export_scale(&sample(Format::LikertBlankLine), &ExportOptions::default());
        let styles = String::from_utf8_lossy(&unzip(&out.bytes, "word/styles.xml")).into_owned();
        for name in ["heading 1", "Instructions", "Item", "Times New Roman"] {
            assert!(styles.contains(name), "{name}");
        }
    }

    fn unzip(bytes: &[u8], name: &str) -> Vec<u8> {
        use std::io::Read;
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut v = Vec::new();
        z.by_name(name).unwrap().read_to_end(&mut v).unwrap();
        v
    }
}
