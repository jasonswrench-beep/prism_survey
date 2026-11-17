//! Parsing a stored scale document into a scale (specification, "Importing his stored scales"). The importer reads the
//! layout cues of the stored files directly: the "_____1." item lines, the anchor row, "Recode" and "Reverse Code"
//! lines, "Factor" lines, "Odd Items" and "Even Items", "Scores should range", the Source and Adapted from blocks, and
//! factor tables (with bold marking the kept items where the file keeps bold). Nothing is saved until the researcher
//! confirms it; what no rule understood is kept in `Scale::unparsed` for the confirm form.

use crate::document::{read_path, Block, Cell, Document, ReadError};
use prismsurvey_core::formats::{abbreviate, classify_labels, is_anchor_word, response_scale};
use prismsurvey_core::instructions;
use prismsurvey_core::scale::{new_blank, title_count};
use prismsurvey_core::*;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

/// One scale found in a file, with what the importer wants the researcher to look at.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub scale: Scale,
    pub file: String,
    /// The scale's position in the file when the file holds several (1 for the first).
    pub part: usize,
    /// True for a blank template ("TITLE", "NAME" and empty item lines).
    pub is_template: bool,
    /// Plain-language notes for the confirm form, most important first.
    pub notes: Vec<String>,
}

/// Imports every scale in a stored file (.txt, .docx or .doc).
pub fn import_path(path: &Path) -> Result<Vec<ImportReport>, ReadError> {
    let doc = read_path(path)?;
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(import_document(&doc, &file))
}

pub fn import_document(doc: &Document, file: &str) -> Vec<ImportReport> {
    let lines = flatten(doc);
    let parts = split_scales(&lines);
    let reports: Vec<ImportReport> = parts
        .into_iter()
        .map(|range| parse_scale(&lines[range], file))
        .collect();
    let reports = merge_repeated_blocks(reports);
    let count = reports.len();
    reports
        .into_iter()
        .enumerate()
        .map(|(i, mut r)| {
            r.part = i + 1;
            if count > 1 {
                r.notes.insert(
                    0,
                    format!("This file holds {count} scales; this is scale {}.", i + 1),
                );
            }
            r
        })
        .collect()
}

/// The item wording of a scale, for comparing blocks.
fn block_key(s: &Scale) -> Vec<String> {
    s.items
        .iter()
        .map(|i| {
            // Punctuation and case are left out, so a missing full stop does not split a block off.
            prismsurvey_core::variant::original_wording(i)
                .to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

/// A run of short lines naming the blocks, as a source lists them under its scoring ("You", "Sailor", ...).
fn block_names(unparsed: &[UnparsedLine], count: usize) -> Option<std::ops::Range<usize>> {
    let short = |t: &str| {
        let w = t.split_whitespace().count();
        (1..=5).contains(&w) && !t.trim_end().ends_with(['.', ':', '?', '!'])
    };
    (0..unparsed.len().saturating_sub(count - 1)).find_map(|i| {
        let run = i..i + count;
        let all = unparsed[run.clone()].iter().all(|u| short(&u.text));
        let bounded = unparsed.get(i + count).is_none_or(|u| !short(&u.text));
        (all && bounded).then_some(run)
    })
}

/// The same items asked again under new prompts (a perspective-taking exercise: "Now answer as a sailor would") make
/// one scale, numbered straight through, with a subscale for each block and each block's prompt printed above its
/// first item. Item numbering restarts in the source, which first splits the blocks into separate scales.
fn merge_repeated_blocks(reports: Vec<ImportReport>) -> Vec<ImportReport> {
    let mut out: Vec<ImportReport> = Vec::new();
    let mut i = 0;
    while i < reports.len() {
        let key = block_key(&reports[i].scale);
        let filled = key.iter().filter(|k| !k.is_empty()).count();
        let mut j = i + 1;
        while j < reports.len()
            && !reports[i].is_template
            && filled >= 2
            && filled == key.len()
            && block_key(&reports[j].scale) == key
            && reports[j].scale.format == reports[i].scale.format
        {
            j += 1;
        }
        if j - i < 2 {
            out.push(reports[i].clone());
            i += 1;
            continue;
        }
        let blocks = &reports[i..j];
        let per = key.len() as u32;
        let k = blocks.len();
        let mut merged = blocks[0].clone();
        let s = &mut merged.scale;
        s.items.clear();
        s.subscales.clear();
        s.unparsed.clear();
        s.provenance.clear();
        s.scoring.stated.clear();
        s.scoring.interpretation.clear();
        let mut notes: Vec<String> = Vec::new();
        for (b, r) in blocks.iter().enumerate() {
            for it in &r.scale.items {
                let mut it = it.clone();
                it.number += b as u32 * per;
                if it.number == b as u32 * per + 1 && b > 0 {
                    let prompt = r.scale.instructions.text.trim();
                    if !prompt.is_empty() {
                        it.lead_in = Some(prompt.to_string());
                    }
                }
                s.items.push(it);
            }
            s.unparsed.extend(r.scale.unparsed.iter().cloned());
            s.provenance.extend(r.scale.provenance.iter().cloned());
            s.scoring
                .stated
                .extend(r.scale.scoring.stated.iter().cloned());
            s.scoring
                .interpretation
                .extend(r.scale.scoring.interpretation.iter().cloned());
            if s.source.is_none() {
                s.source = r.scale.source.clone();
            }
            if s.adapted_from.is_none() {
                s.adapted_from = r.scale.adapted_from.clone();
            }
            for n in &r.notes {
                if !n.contains("line(s) were not understood") && !notes.contains(n) {
                    notes.push(n.clone());
                }
            }
        }
        let names: Vec<String> = match block_names(&s.unparsed, k) {
            Some(run) => {
                let names = s.unparsed[run.clone()]
                    .iter()
                    .map(|u| u.text.trim().to_string())
                    .collect();
                s.unparsed.drain(run);
                names
            }
            None => (1..=k).map(|n| format!("Perspective {n}")).collect(),
        };
        for (b, name) in names.into_iter().enumerate() {
            s.subscales.push(Subscale {
                name,
                items: ItemSelector::Block {
                    first: b as u32 * per + 1,
                    last: (b as u32 + 1) * per,
                },
                other_names: Vec::new(),
            });
        }
        // Each block is scored on its own and compared; a total across them means nothing.
        s.scoring.has_total = false;
        s.score_range.computed = prismsurvey_core::scale::computed_range(s);
        notes.insert(
            0,
            format!(
                "The same {per} items are asked {k} times, each under its own prompt, so they are kept as one scale of {} items with a subscale for each block. Each block's prompt prints above its first item. Check the subscale names.",
                per * k as u32
            ),
        );
        if !s.unparsed.is_empty() {
            notes.push(format!(
                "{} line(s) were not understood and are listed for you to place or drop.",
                s.unparsed.len()
            ));
        }
        merged.notes = notes;
        out.push(merged);
        i = j;
    }
    out
}

/// Moves the source's guide to reading the scores ("Scores between 3 and 7 indicate ...", "3-5 Very calm") from
/// the lines no rule understood into the scoring record: from the first such line to the end of the unparsed lines.
fn take_interpretation(unparsed: &mut Vec<UnparsedLine>) -> Vec<String> {
    let cue = |t: &str| score_band().is_match(t) || score_reading().is_match(t);
    let Some(start) = unparsed.iter().position(|u| cue(&u.text)) else {
        return Vec::new();
    };
    // One stray line is not a guide: ask for a band, or two reading lines.
    let rest = &unparsed[start..];
    let bands = rest
        .iter()
        .filter(|u| score_band().is_match(&u.text))
        .count();
    let readings = rest
        .iter()
        .filter(|u| score_reading().is_match(&u.text))
        .count();
    if bands == 0 && readings < 2 {
        return Vec::new();
    }
    unparsed
        .split_off(start)
        .into_iter()
        .map(|u| u.text)
        .collect()
}

// ---------------------------------------------------------------- lines

/// A block with its position (1-based, counting paragraphs and tables) in the file.
#[derive(Debug, Clone)]
enum Line<'a> {
    Para {
        n: u32,
        text: String,
        bold: Option<bool>,
    },
    Table {
        n: u32,
        rows: &'a [Vec<Cell>],
    },
}

impl Line<'_> {
    fn n(&self) -> u32 {
        match self {
            Line::Para { n, .. } | Line::Table { n, .. } => *n,
        }
    }
}

fn flatten(doc: &Document) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    for (i, b) in doc.blocks.iter().enumerate() {
        let n = i as u32 + 1;
        match b {
            Block::Para(p) => {
                // A paragraph with line breaks inside reads as separate lines.
                for part in p.text.split('\n') {
                    out.push(Line::Para {
                        n,
                        text: part.to_string(),
                        bold: p.bold,
                    });
                }
            }
            Block::Table(rows) if is_layout_table(rows) => {
                // A one-column table used as a box around text: read its lines as paragraphs.
                for row in rows {
                    for cell in row {
                        for part in cell.text.split('\n') {
                            out.push(Line::Para {
                                n,
                                text: part.to_string(),
                                bold: cell.bold,
                            });
                        }
                    }
                }
            }
            Block::Table(rows) => {
                // Text rows at the top of a grid (a title, instructions or a key set in a merged cell) are read as
                // paragraphs; the grid starts at the first row with two or more filled cells.
                let lead = if is_factor_table(rows) {
                    0
                } else {
                    rows.iter()
                        .take_while(|r| r.iter().filter(|c| !c.text.trim().is_empty()).count() <= 1)
                        .count()
                };
                for row in &rows[..lead] {
                    for cell in row {
                        for part in cell.text.split('\n') {
                            out.push(Line::Para {
                                n,
                                text: part.to_string(),
                                bold: cell.bold,
                            });
                        }
                    }
                }
                out.push(Line::Table {
                    n,
                    rows: &rows[lead..],
                });
            }
        }
    }
    out
}

/// A table with at most one filled cell in every row is a box around text, not a grid.
fn is_layout_table(rows: &[Vec<Cell>]) -> bool {
    !rows.is_empty()
        && rows
            .iter()
            .all(|r| r.iter().filter(|c| !c.text.trim().is_empty()).count() <= 1)
}

/// Spaces and tabs collapsed, ends trimmed.
fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------- patterns

macro_rules! re {
    ($name:ident, $pat:expr) => {
        fn $name() -> &'static Regex {
            static R: OnceLock<Regex> = OnceLock::new();
            R.get_or_init(|| Regex::new($pat).unwrap())
        }
    };
}

re!(blank_item, r"^\s*_{2,}\s*(\d{1,3})\s*(?:\.\s*|\s+)(.*)$");
re!(number_cell, r"^\s*(\d{1,3})\s*\.?\s*$");
re!(numbered_text_cell, r"^\s*(\d{1,3})\s*\.\s+(\S[\s\S]*)$");
re!(loading, r"^\s*[-\u{2212}]?\s*(?:0|1)?\.\d+\s*\*?\s*$");
re!(eq_pairs, r"(\d+)\s*=\s*([A-Za-z][A-Za-z \-']*[A-Za-z])");
re!(
    abbr_pairs,
    r"\b([A-Z][A-Za-z]{0,3})\s*=\s*([A-Z][A-Za-z \-']*[a-z])"
);
re!(recode_table_line, r"^\s*(-?\d+)\s*=\s*(-?\d+)\s*$");
re!(score_band, r"^\s*\d+\s*(?:-|\u{2013}|to)\s*\d+\s+[A-Za-z]");
re!(
    score_reading,
    r"(?i)\bscores?\b.*\bindicat|\bunderstand (?:your|the) (?:results|scores)\b|\bindicates the strength\b"
);
re!(
    recode_cue,
    r"(?i)\b(?:recode[ds]?|reverse[\s-]*(?:code[ds]?|scored?|scoring|keyed))\b"
);
re!(
    number_list,
    r"\d+(?:\s*(?:,\s*(?:&|and)?|&|and|-|–|to)\s*\d+)*"
);
re!(
    sd_line,
    r"^\s*(\d{1,3})\s*\.?(?:\t+|\s{2,})\s*(\S.*?)(?:\t+|\s{2,})\s*((?:\d{1,2}\s+){1,10}\d{1,2})(?:\t+|\s{2,})\s*(\S.*?)\s*$"
);
re!(
    odd_even_sum,
    r"(?i)^\s*(.+?)\s*[-–—:]+\s*add\s+(?:all\s+)?(?:the\s+)?(odd|even)(?:[- ]numbered)?\s+(?:questions|items)\b"
);
re!(
    citation_like,
    r"^[A-Z][A-Za-z'\-]+,\s+(?:[A-Z]\.\s*)+.*\((?:19|20)\d\d[a-z]?\)\."
);
re!(
    created_by,
    r"(?i)^\s*(?:created|developed|written|designed)\s+by\s+.+\b(?:19|20)\d\d\b"
);
re!(
    range_stated,
    r"(?i)\bscores?\s+should\s+(?:range\s+)?(?:be\s+)?(?:between|from)\s+(-?\d+(?:\.\d+)?)\s+(?:and|to)\s+(-?\d+(?:\.\d+)?)"
);
re!(
    list_line,
    r"^\s*([A-Za-z][A-Za-z /&'\-]{1,80}?)\s*:\s*((?:\d+\s*(?:,\s*)?(?:&|and)?\s*)+)\.?\s*$"
);
re!(
    odd_even,
    r"(?i)^\s*(odd|even)(?:[- ]numbered)?\s+items?\s+(?:measure|measures|are|=|:|assess|tap)?\s*(.+?)\.?\s*$"
);
re!(
    block_sum,
    r"(?i)\badd\s+(?:questions|items)\s+(\d+)\s*(?:-|–|to|through)\s*(\d+)\s+(?:for|to\s+get)\s+(?:your\s+)?(?:basic\s+)?(.+?)\s*$"
);
re!(
    total_subscales,
    r"(?i)\badd\s+(?:the\s+)?(?:\w+\s+)?subscales?\s+(?:scores?\s+)?together\s+to\s+(?:get|find|compute)\s+(?:your\s+|the\s+)?(.+?)\s*$"
);
re!(
    total_items,
    r"(?i)\badd\s+(?:all\s+)?(?:of\s+)?the\s+(?:numbers|scores|items)\s+together\s+to\s+(?:get|find|compute)\s+(?:your\s+|the\s+)?(.+?)\s*$"
);
re!(
    total_mention,
    r"(?i)\b(?:see|get|compute|find)\s+(?:your|the)\s+((?:composite|overall|total)\s+.+?\s+score)\b"
);
re!(mean_cue, r"(?i)\b(?:average|mean)\b");
re!(
    source_label,
    r"(?i)^\s*(sources?|reference|citation)\s*:\s*(.*)$"
);
re!(
    adapted_label,
    r"(?i)^\s*(?:adapted|modified|derived)\s+from\s*:?\s*(.*)$"
);
re!(year, r"\((?:19|20)\d\d[a-z]?\)|\b(?:19|20)\d\d\b");
re!(
    factor_label,
    r"(?i)^\s*factor\s+([ivx]+|\d+)\s*[-–:.]\s*(.+?)\s*$"
);
re!(points_phrase, r"(?i)\b(\d{1,2})[- ]point\b");
re!(
    placeholder_title,
    r"^(?:TITLE|NAME|SCALE NAME|Title|Name)\s*:?$"
);
re!(range_pair, r"(\d+)\s*(?:-|–|to)\s*(\d+)");
re!(digits, r"\d+");
re!(cell_split, r"\t+|\s{2,}");
re!(instr_prefix, r"(?i)^(instructions|directions)\s*:\s*");
re!(trailing_blanks, r"[\s_]{2,}$");
re!(any_digit, r"\d");
re!(with_following, r"(?i)\bwith\s+the\s+following.*$");
re!(scoring_prefix, r"(?i)^scoring\s*:?\s*");
re!(factor_word, r"(?i)\s+(factor|subscale|dimension)$");
re!(score_word, r"(?i)\s+(score|subscale score)$");
re!(doi, r"(?i)\b(10\.\d{4,9}/[^\s]+[^\s.,;])");

fn numbers_in(s: &str) -> Vec<u32> {
    // Ranges "1-5" expand; everything else is a plain list, repeats kept.
    let mut out = Vec::new();
    let range = range_pair();
    let mut rest = s.to_string();
    for c in range.captures_iter(s) {
        let (a, b): (u32, u32) = (c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0));
        if a < b && b - a < 200 {
            out.extend(a..=b);
            rest = rest.replacen(&c[0], " ", 1);
        }
    }
    if out.is_empty() {
        rest = s.to_string();
    }
    let single = digits();
    let mut singles: Vec<u32> = single
        .find_iter(&rest)
        .filter_map(|m| m.as_str().parse().ok())
        .collect();
    if out.is_empty() {
        return singles;
    }
    out.append(&mut singles);
    out
}

/// The anchor labels in a line split on tabs or runs of spaces, when every piece is an anchor word.
fn anchor_labels(text: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = cell_split()
        .split(text.trim())
        .map(tidy)
        .filter(|p| !p.is_empty())
        .collect();
    (parts.len() >= 2 && parts.iter().all(|p| is_anchor_word(p))).then_some(parts)
}

/// A row of numbers such as "1 2 3 4 5" or "1    2    3".
fn number_row(text: &str) -> Option<Vec<i32>> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    parts
        .iter()
        .map(|p| p.trim_end_matches('.').parse::<i32>().ok())
        .collect()
}

/// "1 = Never; 2 = Rarely; ..." as labels and values.
fn equals_anchors(text: &str) -> Option<(Vec<String>, Vec<i32>)> {
    let caps: Vec<_> = eq_pairs().captures_iter(text).collect();
    if caps.len() < 2 {
        return None;
    }
    let values: Vec<i32> = caps.iter().filter_map(|c| c[1].parse().ok()).collect();
    let labels: Vec<String> = caps.iter().map(|c| tidy(&c[2])).collect();
    // The numbers must climb by one, or this is a recode table or something else.
    let climbs = values.windows(2).all(|w| w[1] == w[0] + 1);
    climbs.then_some((labels, values))
}

/// "SD = Strongly Disagree, D = Disagree, ..." as abbreviations and labels.
fn abbreviation_key(text: &str) -> Option<(Vec<String>, Vec<String>)> {
    let caps: Vec<_> = abbr_pairs().captures_iter(text).collect();
    if caps.len() < 2 {
        return None;
    }
    let labels: Vec<String> = caps
        .iter()
        .map(|c| tidy(&c[2]).trim_start_matches("and ").to_string())
        .collect();
    if !labels.iter().all(|l| is_anchor_word(l)) {
        return None;
    }
    Some((caps.iter().map(|c| c[1].to_string()).collect(), labels))
}

fn cell_texts(row: &[Cell]) -> Vec<String> {
    row.iter().map(|c| tidy(&c.text)).collect()
}

fn is_loading(s: &str) -> bool {
    loading().is_match(s)
}

/// A development (factor) table: most rows hold item text and loading numbers.
fn is_factor_table(rows: &[Vec<Cell>]) -> bool {
    let with_loadings = rows
        .iter()
        .filter(|r| r.iter().filter(|c| is_loading(&c.text)).count() >= 1)
        .count();
    with_loadings >= 3 && with_loadings * 2 >= rows.len().saturating_sub(2)
}

// ---------------------------------------------------------------- splitting a file into scales

fn is_item_line(line: &Line) -> bool {
    match line {
        Line::Para { text, .. } => blank_item().is_match(text) || sd_text_item(text).is_some(),
        Line::Table { rows, .. } => table_items(rows).is_some(),
    }
}

/// A semantic differential pair typed as a line: "1.<tab>Animated<tab>1 2 3 4 5 6 7<tab>Withdrawn".
fn sd_text_item(text: &str) -> Option<(u32, String, String, Vec<i32>)> {
    let c = sd_line().captures(text)?;
    let points: Vec<i32> = c[3]
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    let climbs = points.len() >= 2 && points.windows(2).all(|w| w[1] == w[0] + 1);
    // Both ends must be words; a row of numbers under an anchor row is not a pair.
    let wordy = |s: &str| s.chars().any(|ch| ch.is_alphabetic());
    (climbs && wordy(&c[2]) && wordy(&c[4]))
        .then(|| (c[1].parse().unwrap_or(0), tidy(&c[2]), tidy(&c[4]), points))
}

/// Splits a file that holds several scales (a study file, or the study template) at each new title: a short bold line,
/// or a placeholder title, that comes after a scale's items and before new instructions or items.
fn split_scales(lines: &[Line]) -> Vec<std::ops::Range<usize>> {
    let mut starts = vec![0usize];
    let mut seen_items = false;
    let mut last_number = 0u32;
    let mut last_item_at = 0usize;
    for (i, line) in lines.iter().enumerate() {
        if is_item_line(line) {
            // Numbering that starts again at 1 is a new scale, even with no title of its own. Its instructions and
            // anchor row come with it: the split goes back over them to just after the last scoring or source line.
            let number = match line {
                Line::Para { text, .. } => blank_item()
                    .captures(text)
                    .and_then(|c| c[1].parse::<u32>().ok())
                    .or_else(|| sd_text_item(text).map(|x| x.0)),
                Line::Table { .. } => None,
            };
            if seen_items && number == Some(1) && last_number > 1 {
                let mut start = i;
                let mut kept = 0;
                while start > last_item_at + 1 {
                    let Line::Para { text, .. } = &lines[start - 1] else {
                        break;
                    };
                    let t = tidy(text);
                    let ends_previous = recode_cue().is_match(&t)
                        || source_label().is_match(&t)
                        || adapted_label().is_match(&t)
                        || year().is_match(&t)
                        || recode_table_line().is_match(&t)
                        || t.to_ascii_lowercase().starts_with("scor");
                    if ends_previous {
                        break;
                    }
                    let instruction_like = !t.is_empty()
                        && anchor_labels(text).is_none()
                        && number_row(&t).is_none()
                        && equals_anchors(&t).is_none();
                    if instruction_like {
                        kept += 1;
                        if kept > 3 {
                            break;
                        }
                    }
                    start -= 1;
                }
                if start > *starts.last().unwrap() {
                    starts.push(start);
                }
            }
            if let Some(n) = number {
                last_number = n;
            }
            last_item_at = i;
            seen_items = true;
            continue;
        }
        if !seen_items {
            continue;
        }
        let Line::Para { text, bold, .. } = line else {
            continue;
        };
        let t = tidy(text);
        let words = t.split_whitespace().count();
        let looks_like_title = (placeholder_title().is_match(&t)
            || (*bold == Some(true) && (1..=10).contains(&words)))
            && !t.ends_with(':')
            && t.chars().filter(|c| c.is_alphabetic()).count() >= 2
            && number_row(&t).is_none()
            && anchor_labels(text).is_none()
            && !recode_cue().is_match(&t)
            && !year().is_match(&t)
            && !t.to_ascii_lowercase().contains("factor")
            && !t.to_ascii_lowercase().starts_with("scor");
        // A plain (not bold) short line in title case can be a title too, when an anchor row or instructions follow it.
        let title_case = {
            let ws: Vec<&str> = t.split_whitespace().filter(|w| w.len() > 3).collect();
            !ws.is_empty()
                && ws
                    .iter()
                    .filter(|w| w.chars().next().is_some_and(char::is_uppercase))
                    .count()
                    * 2
                    >= ws.len()
        };
        let plain_title = !looks_like_title
            && (1..=8).contains(&words)
            && title_case
            && !t.ends_with(['.', ':', '?'])
            && t.chars().filter(|c| c.is_alphabetic()).count() >= 3
            && number_row(&t).is_none()
            && anchor_labels(text).is_none()
            && !recode_cue().is_match(&t)
            && !year().is_match(&t)
            && lines[i + 1..].iter().take(10).any(|l| match l {
                Line::Para { text, .. } => {
                    let lt = tidy(text).to_ascii_lowercase();
                    anchor_labels(text).is_some()
                        || equals_anchors(&tidy(text)).is_some()
                        || lt.starts_with("instructions")
                        || lt.starts_with("directions")
                }
                Line::Table { .. } => false,
            });
        if !looks_like_title && !plain_title {
            continue;
        }
        // A new scale follows if items come before any source or scoring line.
        let ahead = lines[i + 1..].iter().take(80);
        let mut found = false;
        for l in ahead {
            if is_item_line(l) {
                found = true;
                break;
            }
            if let Line::Para { text, .. } = l {
                if source_label().is_match(text) || recode_cue().is_match(text) {
                    break;
                }
            }
        }
        if found {
            starts.push(i);
            seen_items = false;
        }
    }
    let mut out = Vec::new();
    for (k, &s) in starts.iter().enumerate() {
        let e = starts.get(k + 1).copied().unwrap_or(lines.len());
        out.push(s..e);
    }
    out
}

// ---------------------------------------------------------------- items in tables

/// What a table of items holds.
struct TableItems {
    format: Format,
    items: Vec<Item>,
    /// Box grid header abbreviations, or the numbers printed in a semantic differential row.
    header: Vec<String>,
    points: Vec<i32>,
}

/// Reads a box grid (a header of abbreviations, then one row per item) or a semantic differential (number, left
/// adjective, the numbers, right adjective).
fn table_items(rows: &[Vec<Cell>]) -> Option<TableItems> {
    if is_factor_table(rows) {
        return None;
    }
    // Semantic differential rows: a run of consecutive numbers 1..n inside the row.
    let mut sd = Vec::new();
    let mut sd_points: Vec<i32> = Vec::new();
    for row in rows {
        let cells = cell_texts(row);
        let nums: Vec<(usize, i32)> = cells
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.parse::<i32>().ok().map(|v| (i, v)))
            .collect();
        if nums.len() >= 3
            && nums
                .windows(2)
                .all(|w| w[1].0 == w[0].0 + 1 && w[1].1 == w[0].1 + 1)
        {
            let first = nums[0].0;
            let last = nums[nums.len() - 1].0;
            let before: Vec<&String> = cells[..first].iter().collect();
            let after: Vec<&String> = cells[last + 1..].iter().collect();
            let number = before.iter().find_map(|c| {
                number_cell()
                    .captures(c)
                    .and_then(|m| m[1].parse::<u32>().ok())
            });
            let left = before
                .iter()
                .rev()
                .find(|c| number_cell().captures(c).is_none())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let right = after
                .iter()
                .find(|c| !c.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_default();
            sd_points = nums.iter().map(|n| n.1).collect();
            let n = number.unwrap_or(sd.len() as u32 + 1);
            let mut it = Item::new(n, "");
            it.left = Some(left);
            it.right = Some(right);
            sd.push(it);
        }
    }
    if sd.len() >= 2 {
        return Some(TableItems {
            format: Format::SemanticDifferential,
            items: sd,
            header: Vec::new(),
            points: sd_points,
        });
    }
    // Box grid: a header row whose cells after the item column are abbreviations or anchor words.
    for (h, row) in rows.iter().enumerate() {
        let cells = cell_texts(row);
        let filled: Vec<&String> = cells.iter().filter(|c| !c.is_empty()).collect();
        let abbr_like = |c: &&String| {
            c.len() <= 3 && c.chars().all(|ch| ch.is_ascii_uppercase()) || is_anchor_word(c)
        };
        // An anchor table (labels over a row of numbers) is not a box grid.
        let has_number_row = rows.iter().any(|r| {
            let f: Vec<String> = cell_texts(r)
                .into_iter()
                .filter(|c| !c.is_empty())
                .collect();
            f.len() >= 2 && f.iter().all(|c| c.parse::<i32>().is_ok())
        });
        if filled.len() >= 2
            && filled.iter().all(abbr_like)
            && !has_number_row
            && rows.len() > h + 1
        {
            let header: Vec<String> = filled.iter().map(|s| s.to_string()).collect();
            let mut items = Vec::new();
            for row in &rows[h + 1..] {
                let cells = cell_texts(row);
                // The item text is the longest cell; a number cell before it gives the item number.
                let text = cells
                    .iter()
                    .filter(|c| number_cell().captures(c).is_none())
                    .max_by_key(|c| c.len())
                    .cloned()
                    .unwrap_or_default();
                let number = cells.iter().find_map(|c| {
                    number_cell()
                        .captures(c)
                        .and_then(|m| m[1].parse::<u32>().ok())
                });
                let (number, text) = match numbered_text_cell().captures(&text) {
                    Some(m) => (m[1].parse().ok(), tidy(&m[2])),
                    None => (number, text),
                };
                items.push(Item::new(number.unwrap_or(items.len() as u32 + 1), text));
            }
            if items.is_empty() {
                return None;
            }
            let points = (1..=header.len() as i32).collect();
            return Some(TableItems {
                format: Format::LikertBoxGrid,
                items,
                header,
                points,
            });
        }
    }
    None
}

// ---------------------------------------------------------------- the parse

#[derive(Default)]
struct Found {
    labels: Option<(Vec<String>, u32)>,
    values: Option<Vec<i32>>,
    abbreviations: Option<Vec<String>>,
    points_phrase: Option<u8>,
}

fn prov(field: &str, file: &str, line: u32, confidence: f64, rule: &str) -> Provenance {
    Provenance {
        field: field.to_string(),
        file: file.to_string(),
        page: None,
        line: Some(line),
        region: None,
        confidence,
        rule: Some(rule.to_string()),
    }
}

fn parse_scale(lines: &[Line], file: &str) -> ImportReport {
    let mut scale = new_blank(Format::LikertBlankLine);
    scale.items.clear();
    scale.licence = Licence {
        status: LicenceStatus::LocalOnly,
        note: Some("Imported from a stored file; stays on this machine.".into()),
    };
    let mut notes: Vec<String> = Vec::new();
    let mut unparsed: Vec<UnparsedLine> = Vec::new();
    let mut found = Found::default();
    let mut is_template = false;

    // Title: the first line with text.
    let mut i = 0;
    while i < lines.len() {
        if let Line::Para { n, text, .. } = &lines[i] {
            let t = tidy(text);
            if !t.is_empty() {
                let lower = t.to_ascii_lowercase();
                let reads_like_instructions = lower.starts_with("instructions")
                    || lower.starts_with("directions")
                    || t.split_whitespace().count() > 12
                    || (t.ends_with('.') && t.split_whitespace().count() > 6);
                if placeholder_title().is_match(&t) {
                    is_template = true;
                    scale
                        .provenance
                        .push(prov("title", file, *n, 0.5, "placeholder title"));
                } else if reads_like_instructions {
                    // No title line: the file starts with its instructions. The file name stands in.
                    let stem = Path::new(file)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    scale.title = stem.clone();
                    scale.provenance.push(prov(
                        "title",
                        file,
                        *n,
                        0.4,
                        "file name (the file has no title line)",
                    ));
                    notes.push(format!(
                        "The file has no title line, so the file name \"{stem}\" is used."
                    ));
                    break;
                } else if !lower.starts_with("instructions") && !lower.starts_with("directions") {
                    scale.title = t.trim_end_matches(':').to_string();
                    scale
                        .provenance
                        .push(prov("title", file, *n, 0.9, "first line of the file"));
                } else {
                    break;
                }
                i += 1;
                break;
            }
        }
        i += 1;
    }

    // Items: blank-line paragraphs, or a table of items.
    let first_item = lines.iter().position(is_item_line);
    let Some(first_item) = first_item else {
        notes.push("No item lines were found. Check that this file holds a scale.".into());
        for l in &lines[i.min(lines.len())..] {
            if let Line::Para { n, text, .. } = l {
                if !tidy(text).is_empty() {
                    unparsed.push(UnparsedLine {
                        line: *n,
                        text: tidy(text),
                    });
                }
            }
        }
        scale.unparsed = unparsed;
        return ImportReport {
            scale,
            file: file.to_string(),
            part: 1,
            is_template,
            notes,
        };
    };

    // Before the items: instructions and the anchor row.
    let mut instr: Vec<(u32, String)> = Vec::new();
    let mut k = i;
    while k < first_item {
        match &lines[k] {
            Line::Para { n, text, .. } => {
                let t = tidy(text);
                if t.is_empty() {
                    k += 1;
                    continue;
                }
                if let Some(labels) = anchor_labels(text) {
                    found.labels = Some((labels, *n));
                } else if found.labels.is_some()
                    && found.values.is_none()
                    && number_row(&t).is_some()
                {
                    found.values = number_row(&t);
                } else if let Some((labels, values)) = equals_anchors(&t) {
                    found.labels = Some((labels, *n));
                    found.values = Some(values);
                } else if let Some((abbr, labels)) = abbreviation_key(&t) {
                    found.labels = Some((labels, *n));
                    found.abbreviations = Some(abbr);
                } else if number_row(&t).is_some() {
                    found.values = number_row(&t);
                } else {
                    let body = instr_prefix().replace(&t, "").to_string();
                    if !body.is_empty() {
                        instr.push((*n, body));
                    }
                }
            }
            Line::Table { n, rows } => {
                // An anchor table (labels over numbers), or instructions set in a one-cell table.
                let mut used = false;
                for row in rows.iter() {
                    let cells: Vec<String> = cell_texts(row)
                        .into_iter()
                        .filter(|c| !c.is_empty())
                        .collect();
                    if cells.len() >= 2 && cells.iter().all(|c| is_anchor_word(c)) {
                        found.labels = Some((cells, *n));
                        used = true;
                    } else if cells.len() >= 2 && cells.iter().all(|c| c.parse::<i32>().is_ok()) {
                        found.values = Some(cells.iter().map(|c| c.parse().unwrap()).collect());
                        used = true;
                    }
                }
                if !used {
                    let text = tidy(
                        &rows
                            .iter()
                            .flat_map(|r| r.iter().map(|c| c.text.clone()))
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    let body = instr_prefix().replace(&text, "").to_string();
                    if !body.is_empty() {
                        instr.push((*n, body));
                    }
                }
            }
        }
        k += 1;
    }
    if let Some(&(n, _)) = instr.first() {
        let text = instr
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(c) = points_phrase().captures(&text) {
            found.points_phrase = c[1].parse().ok();
        }
        scale.instructions = Instructions {
            text,
            origin: InstructionsOrigin::Verbatim,
            house_stem: None,
        };
        scale.provenance.push(prov(
            "instructions",
            file,
            n,
            0.85,
            "text between the title and the anchors",
        ));
    } else {
        scale.instructions = Instructions {
            text: String::new(),
            origin: InstructionsOrigin::Verbatim,
            house_stem: None,
        };
        notes.push("No instructions were found above the items.".into());
    }

    // The items themselves.
    let mut j = first_item;
    let mut table_points: Option<Vec<i32>> = None;
    let mut box_header: Option<Vec<String>> = None;
    match &lines[first_item] {
        Line::Para { .. } => {
            let mut sd_points: Option<Vec<i32>> = None;
            while j < lines.len() {
                match &lines[j] {
                    Line::Para { n, text, .. } => {
                        if let Some(c) = blank_item().captures(text) {
                            let num: u32 = c[1].parse().unwrap_or(0);
                            let text = tidy(&c[2]);
                            scale.items.push(Item::new(num, text));
                            scale.provenance.push(prov(
                                &format!("item:{num}"),
                                file,
                                *n,
                                0.97,
                                "\"_____1.\" item line",
                            ));
                        } else if let Some((num, left, right, points)) = sd_text_item(text) {
                            let mut it = Item::new(num, "");
                            it.left = Some(left);
                            it.right = Some(right);
                            scale.items.push(it);
                            sd_points = Some(points);
                            scale.provenance.push(prov(
                                &format!("item:{num}"),
                                file,
                                *n,
                                0.9,
                                "adjective pair line",
                            ));
                        } else if tidy(text).is_empty() {
                        } else if let Some(last) = scale.items.last_mut() {
                            // A wrapped item continues on the next line; anything else ends the items.
                            let t = tidy(text);
                            let looks_like_scoring = recode_cue().is_match(&t)
                                || source_label().is_match(&t)
                                || list_line().is_match(&t)
                                || odd_even().is_match(&t)
                                || odd_even_sum().is_match(&t)
                                || t.to_ascii_lowercase().starts_with("scor");
                            let prev_blank = matches!(&lines[j - 1], Line::Para { text, .. } if tidy(text).is_empty());
                            let is_pair = last.left.is_some();
                            if !is_pair
                                && !looks_like_scoring
                                && !prev_blank
                                && !last.text.is_empty()
                                && !last.text.ends_with(['.', '?', '!'])
                            {
                                last.text = join_wrapped(&last.text, &t);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    Line::Table { .. } => break,
                }
                j += 1;
            }
            if sd_points.is_some() {
                scale.format = Format::SemanticDifferential;
                table_points = sd_points;
            } else {
                scale.format = Format::LikertBlankLine;
            }
        }
        Line::Table { n, rows } => {
            let t = table_items(rows).expect("is_item_line checked this");
            scale.format = t.format;
            for it in &t.items {
                scale.provenance.push(prov(
                    &format!("item:{}", it.number),
                    file,
                    *n,
                    0.9,
                    "row of the item table",
                ));
            }
            scale.items = t.items;
            if t.format == Format::SemanticDifferential {
                table_points = Some(t.points);
            } else {
                box_header = Some(t.header);
            }
            j = first_item + 1;
        }
    }
    let item_count = scale.items.len() as u32;
    if scale
        .items
        .iter()
        .all(|it| it.text.is_empty() && it.left.as_deref().unwrap_or("").is_empty())
    {
        is_template = true;
    }
    // Numbering: say so when the file skips or repeats a number.
    let numbers: Vec<u32> = scale.items.iter().map(|i| i.number).collect();
    if numbers.iter().enumerate().any(|(k, &n)| n != k as u32 + 1) {
        notes.push(format!(
            "The item numbers in the file are not 1 to {item_count} in order: {}.",
            join_numbers(&numbers)
        ));
    }
    for it in &mut scale.items {
        it.source_text = Some(it.text.clone()).filter(|t| !t.is_empty());
    }

    // The response scale.
    let points_from = |found: &Found| -> Option<u8> {
        found
            .labels
            .as_ref()
            .map(|(l, _)| l.len() as u8)
            .or(found.values.as_ref().map(|v| v.len() as u8))
            .or(table_points.as_ref().map(|v| v.len() as u8))
            .or(box_header.as_ref().map(|h| h.len() as u8))
            .or(found.points_phrase)
    };
    let points = points_from(&found);
    match points {
        Some(p) => {
            let labels = found.labels.as_ref().map(|(l, _)| l.clone());
            let set = labels
                .as_ref()
                .map(|l| classify_labels(l))
                .unwrap_or(AnchorSet::Agreement);
            let mode = if labels.as_ref().is_some_and(|l| l.len() == p as usize) {
                LabelMode::Every
            } else {
                LabelMode::EndsOnly
            };
            let mut r = response_scale(p, set, mode);
            if let Some(l) = labels.filter(|l| l.len() == p as usize) {
                r.labels = l.into_iter().map(Some).collect();
                r.label_mode = LabelMode::Every;
                r.anchor_set = set;
            } else if found.labels.is_none() && set == AnchorSet::Agreement && box_header.is_none()
            {
                // Only a count is known (a numbers row or a semantic differential): label the ends.
                r = response_scale(p, AnchorSet::Agreement, LabelMode::EndsOnly);
                if scale.format == Format::SemanticDifferential {
                    r.labels = vec![None; p as usize];
                    r.anchor_set = AnchorSet::Custom;
                }
            }
            if let Some(v) = found
                .values
                .clone()
                .or(table_points.clone())
                .filter(|v| v.len() == p as usize)
            {
                r.values = v;
            }
            if let Some(a) = found
                .abbreviations
                .clone()
                .or(box_header.clone())
                .filter(|a| a.len() == p as usize)
            {
                r.abbreviations = a;
            } else {
                r.abbreviations = r
                    .labels
                    .iter()
                    .map(|l| l.as_deref().map(abbreviate).unwrap_or_default())
                    .collect();
            }
            scale.response = r;
            let line = found
                .labels
                .as_ref()
                .map(|l| l.1)
                .unwrap_or(lines[first_item].n());
            scale.provenance.push(prov(
                "anchors",
                file,
                line,
                if found.labels.is_some() { 0.95 } else { 0.6 },
                "anchor row",
            ));
        }
        None => {
            notes.push("No anchor row was found; a 5-point agreement scale is assumed. Check the points and labels.".into());
            scale
                .provenance
                .push(prov("anchors", file, lines[first_item].n(), 0.3, "assumed"));
        }
    }

    // After the items: scoring, subscales, range, sources, development tables.
    let mut recode_pairs: Vec<(i32, i32)> = Vec::new();
    let mut subscales: Vec<Subscale> = Vec::new();
    let mut scoring = Scoring::default();
    let mut total_stated = false;
    let mut source_lines: Vec<String> = Vec::new();
    let mut adapted_lines: Vec<String> = Vec::new();
    #[derive(PartialEq)]
    enum Collect {
        None,
        Source,
        Adapted,
    }
    let mut collecting = Collect::None;
    let mut development: Option<DevelopmentRecord> = None;
    // Notes about a development table that come before the table itself.
    let mut early_dev_notes: Vec<String> = Vec::new();
    for line in &lines[j..] {
        match line {
            Line::Table { n, rows } => {
                collecting = Collect::None;
                if is_factor_table(rows) {
                    let dev = parse_factor_table(rows);
                    scale.provenance.push(prov(
                        "development",
                        file,
                        *n,
                        0.8,
                        "table of factor loadings",
                    ));
                    development = Some(match development.take() {
                        Some(mut d) => {
                            d.items.extend(dev.items);
                            d
                        }
                        None => dev,
                    });
                } else {
                    let text = tidy(
                        &rows
                            .iter()
                            .flat_map(|r| r.iter().map(|c| c.text.clone()))
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    if !text.is_empty() {
                        unparsed.push(UnparsedLine {
                            line: *n,
                            text: format!("(table) {text}"),
                        });
                    }
                }
            }
            Line::Para { n, text, .. } => {
                let raw = text.as_str();
                let t = tidy(raw.trim_end_matches(['_', '\t', ' ']).trim_end_matches('_'));
                let t = trailing_blanks().replace(&t, "").trim().to_string();
                if t.is_empty() {
                    continue;
                }
                let lower = t.to_ascii_lowercase();
                // Labels that open a block.
                if let Some(c) = source_label().captures(&t) {
                    collecting = Collect::Source;
                    let rest = tidy(&c[2]);
                    if !rest.is_empty() {
                        source_lines.push(rest);
                    }
                    scale
                        .provenance
                        .push(prov("source", file, *n, 0.9, "\"Source:\" block"));
                    continue;
                }
                if let Some(c) = adapted_label().captures(&t) {
                    collecting = Collect::Adapted;
                    let rest = tidy(&c[1]);
                    if !rest.is_empty() {
                        adapted_lines.push(rest);
                    }
                    scale.provenance.push(prov(
                        "adaptedFrom",
                        file,
                        *n,
                        0.9,
                        "\"Adapted from:\" block",
                    ));
                    continue;
                }
                if collecting != Collect::None {
                    let is_citation = year().is_match(&t) && !lower.contains("factor analysis");
                    let target = if collecting == Collect::Source {
                        &mut source_lines
                    } else {
                        &mut adapted_lines
                    };
                    if is_citation && (target.is_empty() || year().is_match(&t)) {
                        target.push(t.clone());
                        continue;
                    }
                    collecting = Collect::None;
                }
                // Development table notes.
                if let Some(c) = factor_label().captures(&t) {
                    if let Some(d) = development.as_mut() {
                        d.factor_labels
                            .push(format!("Factor {} - {}", &c[1], tidy(&c[2])));
                        continue;
                    }
                }
                let dev_note = lower.contains("factor analysis")
                    || lower.contains("principal component")
                    || lower.contains("rotation")
                    || (lower.contains("bold")
                        && (lower.contains("kept")
                            || lower.contains("retained")
                            || lower.contains("final")))
                    || lower.starts_with("each factor has");
                if dev_note {
                    match development.as_mut() {
                        Some(d) => d.notes.push(t.clone()),
                        None => early_dev_notes.push(t.clone()),
                    }
                    continue;
                }
                // Recode table "1 = 5".
                if let Some(c) = recode_table_line().captures(&t) {
                    recode_pairs.push((c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0)));
                    continue;
                }
                // Range.
                if let Some(c) = range_stated().captures(&t) {
                    let a: f64 = c[1].parse().unwrap_or(0.0);
                    let b: f64 = c[2].parse().unwrap_or(0.0);
                    scale.score_range.stated = Some((a, b));
                    scale.score_range.stated_text = Some(t.clone());
                    scale.provenance.push(prov(
                        "scoreRange",
                        file,
                        *n,
                        0.95,
                        "\"Scores should range\" line",
                    ));
                    continue;
                }
                // Recode list.
                if recode_cue().is_match(&t) && any_digit().is_match(&t) {
                    let after = recode_cue().find(&t).map(|m| &t[m.end()..]).unwrap_or(&t);
                    let after = with_following().replace(after, "");
                    let nums = number_list()
                        .find(&after)
                        .map(|m| numbers_in(m.as_str()))
                        .unwrap_or_default();
                    if !nums.is_empty() {
                        scale.recode.stated = Some(StatedList {
                            numbers: nums,
                            text: t.clone(),
                        });
                        scale
                            .provenance
                            .push(prov("recode", file, *n, 0.95, "recode line"));
                        if mean_cue().is_match(&t) {
                            scoring.total_method = ScoreMethod::Mean;
                        }
                        continue;
                    }
                }
                // "Calm Continuum – Add All Odd Questions".
                if let Some(c) = odd_even_sum().captures(&t) {
                    let sel = if c[2].eq_ignore_ascii_case("odd") {
                        ItemSelector::Odd
                    } else {
                        ItemSelector::Even
                    };
                    let name = strip_factor_word(&tidy(&c[1]));
                    scale.provenance.push(prov(
                        &format!("subscale:{name}"),
                        file,
                        *n,
                        0.85,
                        "\"add all odd/even\" line",
                    ));
                    subscales.push(Subscale {
                        name,
                        items: sel,
                        other_names: vec![],
                    });
                    scoring.stated.push(t.clone());
                    continue;
                }
                // Odd and even items.
                if let Some(c) = odd_even().captures(&t) {
                    let sel = if c[1].eq_ignore_ascii_case("odd") {
                        ItemSelector::Odd
                    } else {
                        ItemSelector::Even
                    };
                    let name = capitalize(&tidy(&c[2]));
                    scale.provenance.push(prov(
                        &format!("subscale:{name}"),
                        file,
                        *n,
                        0.9,
                        "odd/even items line",
                    ));
                    subscales.push(Subscale {
                        name,
                        items: sel,
                        other_names: vec![],
                    });
                    continue;
                }
                // "Add questions 1-5 for your basic Avoidance of Life Score".
                if let Some(c) = block_sum().captures(&t) {
                    let first: u32 = c[1].parse().unwrap_or(1);
                    let last: u32 = c[2].parse().unwrap_or(first);
                    let name = strip_score_word(&tidy(&c[3]));
                    scale.provenance.push(prov(
                        &format!("subscale:{name}"),
                        file,
                        *n,
                        0.9,
                        "\"Add questions a-b\" line",
                    ));
                    subscales.push(Subscale {
                        name,
                        items: ItemSelector::Block { first, last },
                        other_names: vec![],
                    });
                    scoring.stated.push(t.clone());
                    continue;
                }
                // Totals.
                if let Some(c) = total_subscales().captures(&t) {
                    scoring.total_from = TotalFrom::Subscales;
                    set_total_name(&mut scoring, tidy(&c[1]).trim_end_matches('.').to_string());
                    total_stated = true;
                    scoring.stated.push(t.clone());
                    continue;
                }
                if let Some(c) = total_items().captures(&t) {
                    scoring.total_from = TotalFrom::Items;
                    set_total_name(&mut scoring, tidy(&c[1]).trim_end_matches('.').to_string());
                    total_stated = true;
                    scoring.stated.push(t.clone());
                    if mean_cue().is_match(&t) {
                        scoring.total_method = ScoreMethod::Mean;
                    }
                    continue;
                }
                // "<Name> Factor: 1, 3, 5" and other named lists.
                if let Some(c) = list_line().captures(&t) {
                    let name = strip_factor_word(&tidy(&c[1]));
                    let nums = numbers_in(&c[2]);
                    if !nums.is_empty() && !recode_cue().is_match(&name) {
                        scale.provenance.push(prov(
                            &format!("subscale:{name}"),
                            file,
                            *n,
                            0.9,
                            "named item list",
                        ));
                        subscales.push(Subscale {
                            name,
                            items: ItemSelector::List { numbers: nums },
                            other_names: vec![],
                        });
                        continue;
                    }
                }
                // Scoring sentences kept as stated.
                if lower.starts_with("scoring")
                    || lower.contains("add your scores")
                    || lower.contains("recoded the previous")
                {
                    if let Some(c) = total_mention().captures(&t) {
                        set_total_name(&mut scoring, tidy(&c[1]));
                    }
                    if mean_cue().is_match(&t) {
                        scoring.total_method = ScoreMethod::Mean;
                    }
                    let rest = scoring_prefix().replace(&t, "").to_string();
                    if !rest.is_empty() {
                        scoring.stated.push(rest);
                    }
                    continue;
                }
                // A reference with no "Source:" label above it, or "Created by A. Person (2001)".
                if source_lines.is_empty()
                    && (citation_like().is_match(&t) || created_by().is_match(&t))
                {
                    source_lines.push(t.clone());
                    scale.provenance.push(prov(
                        "source",
                        file,
                        *n,
                        0.6,
                        "a reference with no \"Source:\" label",
                    ));
                    continue;
                }
                unparsed.push(UnparsedLine {
                    line: *n,
                    text: t.clone(),
                });
            }
        }
    }

    // Development notes seen before the table itself.
    let early_notes = early_dev_notes;
    if let Some(d) = development.as_mut() {
        let mut all = early_notes;
        all.append(&mut d.notes);
        d.notes = all;
        map_development(d, &scale.items);
        // Factor names under the table that differ from the scoring lines.
        for label in &d.factor_labels {
            if let Some(name) = label.split(" - ").nth(1) {
                let name = strip_factor_word(name);
                let k = d.factor_labels.iter().position(|l| l == label).unwrap_or(0);
                if let Some(sub) = subscales.get_mut(k) {
                    if !sub.name.eq_ignore_ascii_case(&name) && !sub.other_names.contains(&name) {
                        sub.other_names.push(name);
                    }
                }
            }
        }
    } else {
        for t in early_notes {
            unparsed.push(UnparsedLine { line: 0, text: t });
        }
    }
    scale.development = development;

    // Recode rule and reverse flags. The file's own list is the author's statement, so it is taken as confirmed and
    // shown on the confirm form for the researcher to check.
    if !recode_pairs.is_empty() {
        let reflect: Vec<(i32, i32)> = scale
            .response
            .values
            .iter()
            .map(|&v| (v, scale.response.lowest() + scale.response.highest() - v))
            .collect();
        let mut sorted = recode_pairs.clone();
        sorted.sort();
        if sorted != reflect {
            scale.recode.rule = RecodeKind::Table {
                pairs: recode_pairs.clone(),
            };
            notes.push(
                "The file's recode table is not the usual reflection; it is kept as written."
                    .into(),
            );
        }
    }
    if let Some(stated) = scale.recode.stated.clone() {
        let line = scale
            .provenance
            .iter()
            .find(|p| p.field == "recode")
            .and_then(|p| p.line)
            .unwrap_or(0);
        let mut missing = Vec::new();
        for num in &stated.numbers {
            match scale.items.iter_mut().find(|it| it.number == *num) {
                Some(it) => {
                    it.reverse = ReverseFlag {
                        state: ReverseState::Confirmed,
                        reason: Some(format!(
                            "listed in the file's recode line (paragraph {line})"
                        )),
                    }
                }
                None => missing.push(*num),
            }
        }
        if !missing.is_empty() {
            notes.push(format!(
                "The recode line lists item(s) {} that the scale does not have.",
                join_numbers(&missing)
            ));
        }
    }

    // Scoring.
    if !subscales.is_empty() {
        for s in &subscales {
            let out_of_range: Vec<u32> = s
                .items
                .resolve(1000)
                .into_iter()
                .filter(|n| *n > item_count)
                .collect();
            if matches!(
                s.items,
                ItemSelector::List { .. } | ItemSelector::Block { .. }
            ) && !out_of_range.is_empty()
            {
                notes.push(format!(
                    "The {} list names item(s) {} beyond the {} items.",
                    s.name,
                    join_numbers(&out_of_range),
                    item_count
                ));
            }
        }
    }
    scoring.has_total = total_stated || subscales.is_empty();
    scale.subscales = subscales;
    scale.scoring = scoring;
    if !scale.scoring.has_total {
        notes.push("The file names subscales but no total score, so none is set. Turn one on if the scale has a total.".into());
    }

    // Sources.
    if !source_lines.is_empty() {
        scale.source = Some(Citation {
            apa: source_lines.join(" "),
            doi: find_doi(&source_lines.join(" ")),
            reference_manager_id: None,
        });
    }
    if !adapted_lines.is_empty() {
        scale.adapted_from = Some(Citation {
            apa: adapted_lines.join(" "),
            doi: find_doi(&adapted_lines.join(" ")),
            reference_manager_id: None,
        });
    }

    // Title suffix.
    if let Some(c) = title_count(&scale.title) {
        scale.count_suffix = true;
        if c != item_count && !is_template {
            notes.push(format!(
                "The title says {c} items, but the scale has {item_count}."
            ));
        }
    }
    if let Some(stem) = std::path::Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
    {
        if let Some(c) = title_count(&stem) {
            if c != item_count && !is_template {
                notes.push(format!(
                    "The file name says {c} items, but the scale has {item_count}."
                ));
            }
        }
    }

    // Templates keep house instructions when their text is the template's own.
    if is_template {
        notes.insert(
            0,
            "This is a blank template: its lines are empty, and its title is a placeholder.".into(),
        );
        if scale.source.is_some() {
            notes.push("The template still carries a source from an earlier use.".into());
        }
    }
    let reading = take_interpretation(&mut unparsed);
    if !reading.is_empty() {
        notes.push(format!(
            "{} line(s) on how to read the scores are kept with the scoring and print in the researcher copy.",
            reading.len()
        ));
        scale.scoring.interpretation = reading;
    }
    if !unparsed.is_empty() {
        notes.push(format!(
            "{} line(s) were not understood and are listed for you to place or drop.",
            unparsed.len()
        ));
    }
    let fit = instructions::fit_findings(&scale.instructions.text, scale.format, &scale.response);
    for f in fit {
        notes.push(format!("Instructions: \"{}\". {}", f.words, f.message));
    }
    scale.unparsed = unparsed;
    scale.score_range.computed = prismsurvey_core::scale::computed_range(&scale);
    ImportReport {
        scale,
        file: file.to_string(),
        part: 1,
        is_template,
        notes,
    }
}

fn set_total_name(scoring: &mut Scoring, name: String) {
    if name.is_empty() {
        return;
    }
    match &scoring.total_name {
        None => scoring.total_name = Some(name),
        Some(existing) if existing.eq_ignore_ascii_case(&name) => {}
        Some(_) => {
            if !scoring
                .other_total_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(&name))
            {
                scoring.other_total_names.push(name);
            }
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn strip_factor_word(s: &str) -> String {
    factor_word().replace(s.trim(), "").trim().to_string()
}

fn strip_score_word(s: &str) -> String {
    score_word().replace(s.trim(), "").trim().to_string()
}

fn join_numbers(n: &[u32]) -> String {
    n.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn join_wrapped(a: &str, b: &str) -> String {
    // A hyphen broken across lines is rejoined without a space.
    if let Some(stem) = a.strip_suffix('-') {
        if b.chars().next().is_some_and(|c| c.is_lowercase()) {
            return format!("{stem}{b}");
        }
    }
    format!("{a} {b}")
}

fn find_doi(s: &str) -> Option<String> {
    doi().captures(s).map(|c| c[1].to_string())
}

/// Reads a factor table: a header row of factor names, then one row per development item (number, text, loadings).
/// Rows that continue the text of the row above (empty number, no loadings) are joined to it.
fn parse_factor_table(rows: &[Vec<Cell>]) -> DevelopmentRecord {
    let mut factors: Vec<String> = Vec::new();
    let mut items: Vec<DevelopmentItem> = Vec::new();
    let mut any_bold_info = false;
    for row in rows {
        let cells = cell_texts(row);
        let loadings: Vec<(usize, Option<f64>, bool)> = row
            .iter()
            .enumerate()
            .filter(|(_, c)| is_loading(&c.text))
            .map(|(i, c)| (i, parse_loading(&c.text), c.bold == Some(true)))
            .collect();
        if row.iter().any(|c| c.bold.is_some()) {
            any_bold_info = true;
        }
        if loadings.is_empty() {
            let named: Vec<String> = cells
                .iter()
                .filter(|c| !c.is_empty() && number_cell().captures(c).is_none())
                .cloned()
                .collect();
            if items.is_empty() && !named.is_empty() && named.iter().all(|c| c.len() < 60) {
                factors = named;
                continue;
            }
            // Text that continues the previous item.
            if let Some(last) = items.last_mut() {
                let extra: Vec<&String> = cells
                    .iter()
                    .filter(|c| !c.is_empty() && number_cell().captures(c).is_none())
                    .collect();
                if !extra.is_empty()
                    && cells
                        .iter()
                        .all(|c| number_cell().captures(c).is_none() || c.is_empty())
                {
                    last.text = join_wrapped(
                        &last.text,
                        &extra
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                }
            }
            continue;
        }
        let first_loading = loadings[0].0;
        let lead: Vec<&String> = cells[..first_loading]
            .iter()
            .filter(|c| !c.is_empty())
            .collect();
        let mut number = lead.iter().find_map(|c| {
            number_cell()
                .captures(c)
                .and_then(|m| m[1].parse::<u32>().ok())
        });
        let mut text = lead
            .iter()
            .filter(|c| number_cell().captures(c).is_none())
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(m) = numbered_text_cell().captures(&text) {
            number = number.or(m[1].parse().ok());
            text = tidy(&m[2]);
        }
        if text.is_empty() && number.is_none() {
            // Loadings with no item: the continuation row of a text dump, or a blank row.
            continue;
        }
        let kept = loadings.iter().any(|l| l.2);
        items.push(DevelopmentItem {
            number: number.unwrap_or(items.len() as u32 + 1),
            text,
            loadings: loadings.iter().map(|l| l.1).collect(),
            marked_kept: Some(kept),
            final_number: None,
            match_score: None,
        });
    }
    if !any_bold_info {
        for it in &mut items {
            it.marked_kept = None;
        }
    }
    DevelopmentRecord {
        factors,
        factor_labels: Vec::new(),
        items,
        notes: Vec::new(),
    }
}

fn parse_loading(s: &str) -> Option<f64> {
    let t: String = s
        .trim()
        .replace('\u{2212}', "-")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '*')
        .collect();
    let t = if t.starts_with('.') {
        format!("0{t}")
    } else if t.starts_with("-.") {
        t.replacen("-.", "-0.", 1)
    } else {
        t
    };
    t.parse().ok()
}

pub use prismsurvey_core::text::wording_match;

/// Maps development items to final items by wording, never by number. Each final item is used once, best matches first.
pub fn map_development(dev: &mut DevelopmentRecord, items: &[Item]) {
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (d, di) in dev.items.iter().enumerate() {
        for (f, fi) in items.iter().enumerate() {
            let s = wording_match(&di.text, &fi.text);
            if s >= 0.75 {
                pairs.push((s, d, f));
            }
        }
    }
    pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut used_d = std::collections::HashSet::new();
    let mut used_f = std::collections::HashSet::new();
    for (s, d, f) in pairs {
        if used_d.contains(&d) || used_f.contains(&f) {
            continue;
        }
        used_d.insert(d);
        used_f.insert(f);
        dev.items[d].final_number = Some(items[f].number);
        dev.items[d].match_score = Some((s * 100.0).round() / 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::read_txt;

    #[test]
    fn the_same_items_under_new_prompts_are_one_scale() {
        let block = "_____1. I enjoy long walks.\n_____2. I like quiet evenings.\n_____3. I cook for friends.\n";
        // The second block is missing a full stop, as stored files sometimes are.
        let slip = block.replace("evenings.", "evenings");
        let text = format!(
            "Viewpoint Exercise\n\nPlease rate each statement as yourself.\n\n{block}\nNext, answer each one as a sailor would.\n\n{slip}\nNext, answer each one as a baker would.\n\n{block}\nAdd up each block and compare them:\n\nYou\nSailor\nBaker\n\nTalk about why the totals differ.\n"
        );
        let r = import(&text);
        let s = &r.scale;
        assert_eq!(s.items.len(), 9);
        assert_eq!(s.items[3].number, 4);
        assert_eq!(s.items[3].text, "I enjoy long walks.");
        assert!(s.items[3]
            .lead_in
            .as_deref()
            .unwrap()
            .starts_with("Next, answer each one as a sailor would."));
        assert!(s.items[0].lead_in.is_none() && s.items[1].lead_in.is_none());
        let names: Vec<&str> = s.subscales.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["You", "Sailor", "Baker"]);
        assert_eq!(s.subscales[2].items.resolve(9), vec![7, 8, 9]);
        assert!(!s.scoring.has_total);
        assert!(
            r.notes[0].starts_with("The same 3 items are asked 3 times"),
            "{:?}",
            r.notes
        );
        assert!(s.unparsed.iter().all(|u| u.text != "Sailor"));
    }

    #[test]
    fn a_guide_to_reading_the_scores_is_kept_with_the_scoring() {
        let r = import(
            "Mood Scale\n\n_____1. I feel bright.\n_____2. I feel calm.\n_____3. I feel steady.\n\nScores between 3 & 7 indicate a calm mood.\nUse this scale to understand your results.\n3-5 VERY CALM\n6-10 CALM\n11-15 LIVELY\n",
        );
        let s = &r.scale;
        assert_eq!(
            s.scoring.interpretation.len(),
            5,
            "{:?} / {:?}",
            s.scoring.interpretation,
            s.unparsed
        );
        assert_eq!(s.scoring.interpretation[2], "3-5 VERY CALM");
        assert!(s.unparsed.is_empty(), "{:?}", s.unparsed);
    }

    fn import(text: &str) -> ImportReport {
        let doc = read_txt(text.as_bytes());
        let mut v = import_document(&doc, "test.txt");
        assert_eq!(v.len(), 1, "{v:?}");
        v.remove(0)
    }

    #[test]
    fn blank_line_scale_with_recode_table_and_source() {
        let r = import(
            "Garden Comfort-6 Scale\n\nBelow are statements about gardens. Please use the scale below to rate the degree to which each statement applies to you.\n\nStrongly Disagree\tDisagree\tNeutral\tAgree\tStrongly Agree\n   1\t\t2\t\t3\t\t4\t\t5\n\n_____1. I enjoy weeding.\n\n_____2. Gardens bore me.\n\n_____3. I plan my beds in winter.\n\n_____4. I would rather not dig.\n\n_____5. Seed catalogues excite me.\n\n_____6. I like the smell of soil.\n\nScoring:  To compute your score, add your scores for each item as indicated below:\n\nRecode questions 2 & 4 with the following format:\n1\t=\t5\n2\t=\t4\n3\t=\t3\n4\t=\t2\n5\t=\t1\n\nAfter you have recoded the previous questions, add all of the numbers together to get your composite garden comfort score.\n\nScores should range between 6 and 30.\n\nSource:\n\n\nGardener, A. B.  (2010).  Measuring gardens.  Journal of Soil, 1, 1-10.\n",
        );
        let s = &r.scale;
        assert_eq!(s.title, "Garden Comfort-6 Scale");
        assert!(s.count_suffix);
        assert_eq!(s.format, Format::LikertBlankLine);
        assert_eq!(s.items.len(), 6);
        assert_eq!(s.items[1].text, "Gardens bore me.");
        assert_eq!(s.response.points, 5);
        assert_eq!(s.response.anchor_set, AnchorSet::Agreement);
        assert_eq!(s.recode.stated.as_ref().unwrap().numbers, vec![2, 4]);
        assert!(
            s.items[1].reverse.is_reversed()
                && s.items[3].reverse.is_reversed()
                && !s.items[0].reverse.is_reversed()
        );
        assert_eq!(s.recode.rule, RecodeKind::Reflect);
        assert_eq!(
            s.scoring.total_name.as_deref(),
            Some("composite garden comfort score")
        );
        assert_eq!(s.score_range.stated, Some((6.0, 30.0)));
        assert_eq!(s.score_range.computed, Some((6.0, 30.0)));
        assert!(s
            .source
            .as_ref()
            .unwrap()
            .apa
            .starts_with("Gardener, A. B. (2010)."));
        assert_eq!(s.instructions.origin, InstructionsOrigin::Verbatim);
        assert!(s.unparsed.is_empty(), "{:?}", s.unparsed);
    }

    #[test]
    fn frequency_anchors_odd_even_and_repeats_in_the_recode_list() {
        let r = import(
            "River Talk Scale\n\nInstructions: Tell us how often.\n\n1 = Never; 2 = Rarely; 3 = Occasionally; 4 = Often; 5 = Very Often\n\n_____1. I talk about rivers.\n\n_____2. I point at rivers.\n\n_____3. I do not talk about rivers.\n\n_____4. I do not point at rivers.\n\nOdd Items measure spoken river talk\nEven Items measure gestured river talk\n\nReverse Code Items 3, 4, 4\n",
        );
        let s = &r.scale;
        assert_eq!(s.response.anchor_set, AnchorSet::Frequency);
        assert_eq!(s.instructions.text, "Tell us how often.");
        assert_eq!(s.subscales.len(), 2);
        assert_eq!(s.subscales[0].name, "Spoken river talk");
        assert_eq!(s.subscales[0].items, ItemSelector::Odd);
        assert_eq!(
            s.recode.stated.as_ref().unwrap().numbers,
            vec![3, 4, 4],
            "repeats are kept for the self-check"
        );
        assert!(!s.scoring.has_total);
    }

    #[test]
    fn blocks_and_a_total_from_subscales_with_two_names() {
        let r = import(
            "Kite Worry-4\n\nRate each.\n\nStrongly Disagree\tDisagree\tNeutral\tAgree\tStrongly Agree\n1\t2\t3\t4\t5\n\n_____1. a.\n\n_____2. b.\n\n_____3. c.\n\n_____4. d.\n\nAfter you have recoded the previous questions, follow the following to see your composite Kite Worry score\n\nAdd questions 1-2 for your basic String Score\t\t________\n\nAdd questions 3-4 for your basic Wind Score\t\t________\n\nAdd the two subscales together to get your Overall Kite Fret Score\t________\n",
        );
        let s = &r.scale;
        assert_eq!(s.subscales.len(), 2);
        assert_eq!(s.subscales[1].name, "Wind");
        assert_eq!(
            s.subscales[1].items,
            ItemSelector::Block { first: 3, last: 4 }
        );
        assert_eq!(s.scoring.total_from, TotalFrom::Subscales);
        assert_eq!(
            s.scoring.total_name.as_deref(),
            Some("composite Kite Worry score")
        );
        assert_eq!(
            s.scoring.other_total_names,
            vec!["Overall Kite Fret Score".to_string()]
        );
        assert_eq!(s.score_range.computed, Some((4.0, 20.0)));
    }

    #[test]
    fn factor_lines_and_a_development_table_matched_by_wording() {
        let r = import(
            "Lamp Use Scale\n\nRate each.\n\nStrongly Disagree\tDisagree\tNeutral\tAgree\tStrongly Agree\n1\t2\t3\t4\t5\n\n_____1. I switch lamps on at dusk.\n\n_____2. I read by lamplight.\n\nGlow Factor:  1, 2\n\nFactor Analysis of the Lamp Use Scale\n\n|  |  |Glow |\n|1. |I like candles. |.300 |\n|2. |I read by lamplight. |.700 |\n|3. |I switch lamps on at dusk. |.650 |\n\nBolded items were kept.\n",
        );
        let s = &r.scale;
        assert_eq!(s.subscales[0].name, "Glow");
        let d = s.development.as_ref().unwrap();
        assert_eq!(d.factors, vec!["Glow"]);
        assert_eq!(d.items.len(), 3);
        // Development item 3 is final item 1: matched by wording, not by number.
        assert_eq!(d.items[2].final_number, Some(1));
        assert_eq!(d.items[1].final_number, Some(2));
        assert_eq!(d.items[0].final_number, None);
        assert!(d.notes.iter().any(|n| n.contains("Bolded")));
        assert!(s.unparsed.is_empty(), "{:?}", s.unparsed);
    }

    #[test]
    fn a_blank_template_is_recognised() {
        let r = import("TITLE\n\nRead the following.\n\n_____1. \n\n_____2. \n\n_____3.\n\nSource:\n\nOld, A. (2003). A fatigue measure. J, 1, 2.\n");
        assert!(r.is_template);
        assert_eq!(r.scale.items.len(), 3);
        assert!(r
            .notes
            .iter()
            .any(|n| n.contains("source from an earlier use")));
    }
}
