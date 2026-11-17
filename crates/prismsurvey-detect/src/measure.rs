//! Finding candidate measures in an article and parsing one into a scale (specification, "Find candidate measures",
//! "What it parses" and "Reverse-item suggestions").
//!
//! The article's blocks, in reading order, become units: a paragraph is one unit and every table row is one. A unit is
//! item-like when it is a numbered short statement, a table row with one statement cell and short other cells (codes,
//! loadings), or an adjective pair. A run of three or more item-like units is a candidate, scored on the cues the
//! specification lists: the run itself, a response line nearby, words such as scale, subscale and reverse-coded, a
//! factor-loading table, and first-person wording. Candidates above a threshold are offered, ranked, with their page.

use crate::layout::{reading_order, starts_like_item, Block, BlockKind};
use crate::pdf::Page;
use prismsurvey_core::formats::{abbreviate, classify_labels, is_anchor_word, response_scale};
use prismsurvey_core::scale::new_blank;
use prismsurvey_core::*;
use regex::Regex;
use serde::Serialize;
use std::sync::OnceLock;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        fn $name() -> &'static Regex {
            static R: OnceLock<Regex> = OnceLock::new();
            R.get_or_init(|| Regex::new($pat).unwrap())
        }
    };
}

re!(
    item_number,
    r"^\s*_*\s*\(?([0-9]{1,3})[.)]?\s*\)?\s*[.:]?\s+(\S.*)$"
);
re!(
    cue_words,
    r"(?i)\b(scale|measure|instrument|inventory|questionnaire|items?|subscales?|factors?|appendix|reverse[\s-]*(?:coded|scored|keyed)|adapted\s+from|likert|cronbach|alpha|anchored|rated)\b"
);
re!(
    points_phrase,
    r"(?i)\b(\d{1,2}|two|three|four|five|six|seven|eight|nine|ten|eleven)[\s-]*point\b"
);
re!(
    range_phrase,
    r#"(?i)\b(?:from|ranging\s+from|between|range\s+from)\s+(\d{1,2})\s*(?:=|\(|,|:)?\s*[\"“']?([A-Za-z][A-Za-z \-]{1,40}?)[\"”']?\s*\)?\s*,?\s*(?:to|-|–|and)\s+(\d{1,2})\s*(?:=|\(|,|:)?\s*[\"“']?([A-Za-z][A-Za-z \-]{1,40}?)[\"”']?\s*\)"#
);
re!(
    reverse_statement,
    r"(?i)\bitems?\s+((?:\d{1,3}\s*(?:,|and|&|or)?\s*)+)\s*(?:were|was|are|is|had\s+been|have\s+been)\s+(?:reverse[\s-]*(?:coded|scored|keyed)|recoded|reversed)"
);
re!(
    reverse_statement2,
    r"(?i)\b(?:reverse[\s-]*(?:coded|scored|keyed)|recoded|reversed)\s+(?:items?|questions?)\s*(?:were|was|are|is|include|:)?\s*(?:items?\s*)?((?:\d{1,3}\s*(?:,|and|&)?\s*)+)"
);
re!(marker_r, r"\s*(?:\((?:R|r|RC|rev)\)|\[R\]|\*)\s*$");
re!(
    inline_items,
    r#"(?i)\b(?:\d+|two|three|four|five|six|seven|eight|nine|ten|twelve|following)\s+(?:items?|statements|questions)\b[^.]{0,80}[:“"]"#
);
re!(
    sample_items,
    r"(?i)\b(sample|example)\s+items?\s+(include|are|is|were)\b"
);
re!(quoted, r#"[“"]([^”"]{8,200})[”"]"#);
re!(
    named_measure,
    r"\b((?:[A-Z][A-Za-z\-’']+\s+){1,8}(?:Scale|Inventory|Questionnaire|Measure|Index|Instrument|Test))\b(?:\s*\(([A-Z][A-Za-z0-9\-]{1,12})\))?"
);
re!(doi, r"(?i)\b(10\.\d{4,9}/[^\s\]\)>,;]+[^\s\]\)>,;.])");
re!(year, r"\b(19[5-9]\d|20[0-4]\d)\b");
re!(first_person, r"(?i)\b(i|i'm|i’m|my|me|myself|we|our)\b");
re!(loading, r"^[-−]?\s*(?:0|1)?\.\d{2,3}\*?$");
re!(
    code_cell,
    r"^(?:[A-Za-z]{1,3}\d{0,2}|R|\*|[-−]?\d{1,2}(?:\.\d+)?)$"
);
re!(sup_letter, r"^(.*[a-z\)\.])([a-c])$");
re!(
    footnote_reverse,
    r"(?i)^\s*([a-c*†‡])\s*(?:reverse[\s-]*(?:coded|scored|keyed)|reversed)\b"
);

fn words(s: &str) -> usize {
    s.split_whitespace().count()
}

fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One unit of the article: a paragraph, or one table row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub page: u32,
    pub region: (f64, f64, f64, f64),
    pub text: String,
    pub cells: Vec<String>,
    pub in_table: bool,
}

pub fn units(blocks: &[Block]) -> Vec<Unit> {
    let mut out = Vec::new();
    for b in blocks {
        match b.kind {
            BlockKind::Paragraph => out.push(Unit {
                page: b.page,
                region: (b.x0, b.y0, b.x1, b.y1),
                text: b.text(),
                cells: vec![],
                in_table: false,
            }),
            BlockKind::Table => {
                for l in &b.lines {
                    out.push(Unit {
                        page: l.page,
                        region: (l.x0, l.y0, l.x1, l.y1),
                        text: l.plain(),
                        cells: l.pieces.iter().map(|p| p.text.clone()).collect(),
                        in_table: true,
                    })
                }
            }
        }
    }
    out
}

/// What an item-like unit holds.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemGuess {
    pub number: Option<u32>,
    pub text: String,
    pub left: Option<String>,
    pub right: Option<String>,
    /// Short cells beside the statement (subscale codes, loadings, marks).
    pub extra: Vec<String>,
    pub reverse_mark: bool,
    pub unit: usize,
}

fn is_statement(t: &str) -> bool {
    let n = words(t);
    (2..=60).contains(&n) && t.chars().filter(|c| c.is_alphabetic()).count() * 2 > t.chars().count()
}

/// Reads one unit as an item, if it is one.
fn item_of(u: &Unit, index: usize) -> Option<ItemGuess> {
    if u.in_table && u.cells.len() >= 2 {
        let cells: Vec<String> = u.cells.iter().map(|c| tidy(c)).collect();
        // Adjective pair: word, numbers, word.
        let nums = cells.iter().filter(|c| c.parse::<i32>().is_ok()).count();
        if nums >= 3 && cells.len() >= nums + 2 {
            let words_only: Vec<&String> = cells
                .iter()
                .filter(|c| c.parse::<i32>().is_err() && !c.ends_with('.'))
                .collect();
            if words_only.len() >= 2 {
                return Some(ItemGuess {
                    number: None,
                    text: String::new(),
                    left: Some(words_only[0].clone()),
                    right: Some(words_only[words_only.len() - 1].clone()),
                    extra: vec![],
                    reverse_mark: false,
                    unit: index,
                });
            }
        }
        let (si, st) = cells.iter().enumerate().max_by_key(|(_, c)| c.len())?;
        let others: Vec<&String> = cells
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != si)
            .map(|(_, c)| c)
            .collect();
        let short_others = others.iter().all(|c| c.len() <= 14);
        if !short_others {
            return None;
        }
        let mut number = cells[..si]
            .iter()
            .find_map(|c| c.trim_end_matches(['.', ')']).parse::<u32>().ok());
        let mut text = st.clone();
        if let Some(m) = item_number().captures(&text) {
            if number.is_none() {
                number = m[1].parse().ok();
                text = m[2].to_string();
            }
        }
        if !is_statement(&text) {
            return None;
        }
        // A table row counts as an item only with an item number or a loading beside it (not a header or a legend).
        let has_loading = others.iter().any(|c| loading().is_match(c));
        if number.is_none() && !has_loading {
            return None;
        }
        let extra: Vec<String> = cells
            .iter()
            .enumerate()
            .filter(|(i, c)| *i > si && !c.is_empty())
            .map(|(_, c)| c.clone())
            .collect();
        let (text, mark) = strip_marker(&text);
        return Some(ItemGuess {
            number,
            text,
            left: None,
            right: None,
            extra,
            reverse_mark: mark,
            unit: index,
        });
    }
    if !starts_like_item(&u.text) {
        return None;
    }
    let m = item_number().captures(&u.text)?;
    let text = m[2].to_string();
    if !is_statement(&text) || words(&text) > 45 {
        return None;
    }
    let (text, mark) = strip_marker(&text);
    Some(ItemGuess {
        number: m[1].parse().ok(),
        text,
        left: None,
        right: None,
        extra: vec![],
        reverse_mark: mark,
        unit: index,
    })
}

fn strip_marker(t: &str) -> (String, bool) {
    if marker_r().is_match(t) {
        (tidy(&marker_r().replace(t, "")), true)
    } else {
        (tidy(t), false)
    }
}

/// A candidate measure: a run of items with the evidence around it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub score: f64,
    pub page: u32,
    pub first_unit: usize,
    pub last_unit: usize,
    pub items: Vec<ItemGuess>,
    pub reasons: Vec<String>,
    /// Only sample items were found ("sample items include"), so the scale is incomplete.
    pub sample_only: bool,
}

/// Every candidate in the article, best first, above the threshold.
pub fn candidates(us: &[Unit]) -> Vec<Candidate> {
    let mut runs: Vec<Vec<ItemGuess>> = Vec::new();
    let mut current: Vec<ItemGuess> = Vec::new();
    let mut misses = 0;
    for (i, u) in us.iter().enumerate() {
        match item_of(u, i) {
            Some(g) => {
                current.push(g);
                misses = 0;
            }
            None => {
                // A short line under an item continues it (a wrapped cell).
                if let Some(last) = current.last_mut() {
                    let footnote =
                        footnote_reverse().is_match(&u.text) || footnote_start().is_match(&u.text);
                    let wraps = misses == 0
                        && !footnote
                        && words(&u.text) <= 8
                        && !starts_like_item(&u.text)
                        && u.page == us[last.unit].page
                        && u.text.chars().next().is_some_and(char::is_lowercase);
                    if wraps && last.left.is_none() {
                        last.text = tidy(&format!("{} {}", last.text, u.text));
                        let (t, m) = strip_marker(&last.text);
                        last.text = t;
                        last.reverse_mark |= m;
                        continue;
                    }
                }
                misses += 1;
                if misses > 1 && !current.is_empty() {
                    runs.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    let mut out: Vec<Candidate> = runs
        .into_iter()
        .filter(|r| r.len() >= 3)
        .map(|items| score(us, items))
        .filter(|c| c.score >= 4.0)
        .collect();
    // Items quoted inline in a paragraph ("with four items: "...", "..."") or given as samples.
    for (i, u) in us.iter().enumerate() {
        let sample = sample_items().is_match(&u.text) || such_as_quote().is_match(&u.text);
        let listed = inline_items().is_match(&u.text);
        if !(sample || listed) {
            continue;
        }
        let qs: Vec<String> = quoted()
            .captures_iter(&u.text)
            .map(|c| tidy(&c[1]))
            .filter(|q| is_statement(q) && words(q) >= 3)
            .collect();
        if qs.len() < if sample { 1 } else { 2 } {
            continue;
        }
        let items = qs
            .into_iter()
            .map(|t| ItemGuess {
                number: None,
                text: t,
                left: None,
                right: None,
                extra: vec![],
                reverse_mark: false,
                unit: i,
            })
            .collect();
        let mut c = score(us, items);
        if sample {
            c.sample_only = true;
            c.score = c.score.min(4.5);
            c.reasons
                .push("only sample items are given, so the scale is incomplete".into());
        } else {
            c.score += 2.0;
            c.reasons.push("items quoted in the text".into());
        }
        out.push(c);
    }
    let mut out = merge_across_pages(out);
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// A measure printed over a page break comes out as two runs; when the second run's numbering carries on from the
/// first, they are one candidate.
fn merge_across_pages(mut cands: Vec<Candidate>) -> Vec<Candidate> {
    cands.sort_by_key(|c| c.first_unit);
    let mut out: Vec<Candidate> = Vec::new();
    for c in cands {
        if let Some(prev) = out.last_mut() {
            let last_n = prev.items.last().and_then(|g| g.number);
            let first_n = c.items.first().and_then(|g| g.number);
            let continues = matches!((last_n, first_n), (Some(a), Some(b)) if b == a + 1);
            if continues
                && !prev.sample_only
                && !c.sample_only
                && c.first_unit <= prev.last_unit + 40
            {
                prev.items.extend(c.items);
                prev.last_unit = c.last_unit;
                prev.score = prev.score.max(c.score) + 1.0;
                prev.reasons.push("continues over a page break".into());
                continue;
            }
        }
        out.push(c);
    }
    out
}

fn score(us: &[Unit], items: Vec<ItemGuess>) -> Candidate {
    let first = items.first().map(|g| g.unit).unwrap_or(0);
    let last = items.last().map(|g| g.unit).unwrap_or(0);
    let mut s = 0.0;
    let mut reasons = Vec::new();
    let n = items.len();
    s += (n as f64).min(12.0) * 0.5;
    reasons.push(format!("{n} item-like lines in a row"));
    // Numbering that runs 1, 2, 3.
    let nums: Vec<u32> = items.iter().filter_map(|g| g.number).collect();
    if nums.len() >= 3
        && nums.windows(2).filter(|w| w[1] == w[0] + 1).count() * 10 >= (nums.len() - 1) * 8
    {
        s += 1.5;
        reasons.push("numbered in order".into());
    }
    let fp = items
        .iter()
        .filter(|g| first_person().is_match(&g.text))
        .count();
    if fp * 2 >= n {
        s += 1.5;
        reasons.push("first-person statements".into());
    }
    if items.iter().any(|g| g.left.is_some()) {
        s += 2.0;
        reasons.push("adjective pairs".into());
    }
    let loadings = items
        .iter()
        .filter(|g| g.extra.iter().any(|e| loading().is_match(e)))
        .count();
    if loadings * 2 >= n {
        s += 2.0;
        reasons.push("a factor-loading table".into());
    }
    // Long, past-tense procedure steps are not items.
    let long = items.iter().filter(|g| words(&g.text) > 30).count();
    if long * 2 > n {
        s -= 3.0;
    }
    // Reference lists: years and initials.
    let refs = items
        .iter()
        .filter(|g| year().is_match(&g.text) && initials().is_match(&g.text))
        .count();
    if refs * 2 > n {
        s -= 5.0;
    }
    // Cues in the units around the run.
    let lo = first.saturating_sub(8);
    let hi = (last + 8).min(us.len().saturating_sub(1));
    let around: String = us[lo..=hi]
        .iter()
        .map(|u| u.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let cues: std::collections::BTreeSet<String> = cue_words()
        .find_iter(&around)
        .map(|m| m.as_str().to_lowercase())
        .collect();
    if !cues.is_empty() {
        s += (cues.len() as f64).min(4.0) * 0.5;
        reasons.push(format!(
            "nearby words: {}",
            cues.into_iter().take(5).collect::<Vec<_>>().join(", ")
        ));
    }
    if points_phrase().is_match(&around)
        || range_phrase().is_match(&around)
        || anchor_line(&us[lo..=hi]).is_some()
    {
        s += 2.0;
        reasons.push("a response line or point count nearby".into());
    }
    Candidate {
        score: (s * 10.0f64).round() / 10.0,
        page: us[first].page,
        first_unit: first,
        last_unit: last,
        items,
        reasons,
        sample_only: false,
    }
}

/// An anchor row near the items: two or more anchor words in one unit's cells, or "1 = Never, 2 = ...".
fn anchor_line(us: &[Unit]) -> Option<Vec<String>> {
    for u in us {
        let cells: Vec<String> = if u.cells.len() >= 2 {
            u.cells.iter().map(|c| tidy(c)).collect()
        } else {
            vec![]
        };
        let anchors: Vec<String> = cells
            .iter()
            .filter(|c| is_anchor_word(c))
            .cloned()
            .collect();
        if anchors.len() >= 2 && anchors.len() * 2 >= cells.len() {
            return Some(anchors);
        }
        let eq: Vec<String> = equals_label()
            .captures_iter(&u.text)
            .map(|c| tidy(&c[1]))
            .collect();
        if eq.len() >= 2 {
            return Some(eq);
        }
    }
    None
}

// ---------------------------------------------------------------- parsing a candidate

/// Turns a word number ("seven") into a count.
fn count_word(w: &str) -> Option<u8> {
    w.parse().ok().or(match w.to_lowercase().as_str() {
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        _ => None,
    })
}

fn numbers_in(s: &str) -> Vec<u32> {
    small_number()
        .find_iter(s)
        .filter_map(|m| m.as_str().parse().ok())
        .collect()
}

fn prov(field: &str, file: &str, u: &Unit, confidence: f64, rule: &str) -> Provenance {
    Provenance {
        field: field.into(),
        file: file.into(),
        page: Some(u.page),
        line: None,
        region: Some(u.region),
        confidence,
        rule: Some(rule.into()),
    }
}

/// The article's own reference: title (the largest type on the first page), DOI and year.
pub fn article_reference(pages: &[Page], blocks: &[Block]) -> Option<Citation> {
    let first = pages.first()?;
    let first_page = blocks
        .iter()
        .filter(|b| b.page == first.number)
        .collect::<Vec<_>>();
    let title = first_page
        .iter()
        .filter(|b| {
            b.kind == BlockKind::Paragraph
                && (3..=30).contains(&words(&b.text()))
                && b.y0 < first.height * 0.5
        })
        .max_by(|a, b| {
            a.size
                .partial_cmp(&b.size)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|b| b.text());
    let all: String = blocks
        .iter()
        .filter(|b| b.page <= 2)
        .map(|b| b.text())
        .collect::<Vec<_>>()
        .join(" ");
    let doi = doi().captures(&all).map(|c| c[1].to_string());
    let yr = year().captures(&all).map(|c| c[1].to_string());
    let title = title?;
    let apa = match &yr {
        Some(y) => format!(
            "({y}). {title}.{}",
            doi.as_ref()
                .map(|d| format!(" https://doi.org/{d}"))
                .unwrap_or_default()
        ),
        None => format!(
            "{title}.{}",
            doi.as_ref()
                .map(|d| format!(" https://doi.org/{d}"))
                .unwrap_or_default()
        ),
    };
    Some(Citation {
        apa,
        doi,
        reference_manager_id: None,
    })
}

/// Parses a candidate into a scale for the confirm screen. Every field carries its page, region and confidence.
pub fn parse(c: &Candidate, us: &[Unit], pages: &[Page], blocks: &[Block], file: &str) -> Scale {
    let mut s = new_blank(Format::LikertBlankLine);
    s.items.clear();
    s.licence = Licence {
        status: LicenceStatus::LocalOnly,
        note: Some("Found in a journal article; stays on this machine.".into()),
    };
    let all_text: String = us
        .iter()
        .map(|u| u.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let lo = c.first_unit.saturating_sub(10);
    let hi = (c.last_unit + 12).min(us.len().saturating_sub(1));

    // Items.
    let sd = c.items.iter().any(|g| g.left.is_some());
    for (k, g) in c.items.iter().enumerate() {
        let mut it = Item::new(k as u32 + 1, g.text.clone());
        if sd {
            it.left = g.left.clone();
            it.right = g.right.clone();
        }
        it.source_text = Some(g.text.clone()).filter(|t| !t.is_empty());
        if g.reverse_mark {
            it.reverse = ReverseFlag {
                state: ReverseState::Suggested,
                reason: Some(format!(
                    "marked as reversed in the article, page {}",
                    us[g.unit].page
                )),
            };
        }
        s.provenance.push(prov(
            &format!("item:{}", k + 1),
            file,
            &us[g.unit],
            0.85,
            "item-like line",
        ));
        s.items.push(it);
    }
    s.format = if sd {
        Format::SemanticDifferential
    } else {
        Format::LikertBlankLine
    };

    // Superscript reverse marks explained by a footnote ("a Reversed item").
    let footnote = us[c.last_unit.min(us.len() - 1)..=hi].iter().find_map(|u| {
        footnote_reverse()
            .captures(&u.text)
            .map(|m| m[1].to_string())
    });
    if let Some(mark) = footnote {
        for (k, g) in c.items.iter().enumerate() {
            if let Some(m) = sup_letter().captures(&g.text) {
                if m[2] == *mark {
                    let it = &mut s.items[k];
                    it.text = m[1].trim().to_string();
                    it.source_text = Some(it.text.clone());
                    it.reverse = ReverseFlag {
                        state: ReverseState::Suggested,
                        reason: Some(format!(
                            "marked \"{mark}\", which the table's note says is reversed (page {})",
                            us[g.unit].page
                        )),
                    };
                }
            }
        }
    }
    // A sentence that lists the reversed items wins over marks.
    let window: String = us[lo..=hi]
        .iter()
        .map(|u| u.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let stated = reverse_statement()
        .captures(&window)
        .or_else(|| reverse_statement2().captures(&window));
    if let Some(m) = stated {
        let nums = numbers_in(&m[1]);
        let page = us[lo..=hi]
            .iter()
            .find(|u| u.text.contains(m.get(0).unwrap().as_str()))
            .map(|u| u.page)
            .unwrap_or(c.page);
        for it in &mut s.items {
            if nums.contains(&it.number) {
                it.reverse = ReverseFlag {
                    state: ReverseState::Suggested,
                    reason: Some(format!("stated in the article, page {page}")),
                };
            }
        }
        s.recode.stated = Some(StatedList {
            numbers: nums,
            text: tidy(m.get(0).unwrap().as_str()),
        });
    }

    // Subscales: a code column (S1, F2) explained by a legend, or headings inside the table.
    let codes: Vec<Option<String>> = c
        .items
        .iter()
        .map(|g| {
            g.extra
                .iter()
                .find(|e| {
                    code_cell().is_match(e)
                        && !loading().is_match(e)
                        && e.chars().any(char::is_alphabetic)
                })
                .cloned()
        })
        .collect();
    let distinct: std::collections::BTreeSet<&String> = codes.iter().flatten().collect();
    if distinct.len() >= 2 && codes.iter().filter(|c| c.is_some()).count() * 10 >= c.items.len() * 8
    {
        for code in distinct {
            let legend = us[c.first_unit..=hi]
                .iter()
                .find(|u| u.cells.first().is_some_and(|f| tidy(f) == *code) && u.cells.len() >= 2)
                .map(|u| tidy(&u.cells[1..].join(" ")));
            let numbers: Vec<u32> = codes
                .iter()
                .enumerate()
                .filter(|(_, x)| x.as_deref() == Some(code.as_str()))
                .map(|(k, _)| k as u32 + 1)
                .collect();
            s.subscales.push(Subscale {
                name: legend.unwrap_or_else(|| code.clone()),
                items: ItemSelector::List { numbers },
                other_names: vec![],
            });
        }
        s.scoring.has_total = false;
    }

    // Points and anchors.
    let near = &us[lo..=hi];
    let mut found_points: Option<u8> = None;
    let mut labels: Option<Vec<String>> = None;
    if let Some(m) = range_phrase().captures(&window) {
        let (a, b): (u8, u8) = (m[1].parse().unwrap_or(1), m[3].parse().unwrap_or(5));
        if b > a && b - a < 11 {
            found_points = Some(b - a + 1);
            let lo_l = tidy(&m[2]);
            let hi_l = tidy(&m[4]);
            let mut ls = vec![None; (b - a + 1) as usize];
            ls[0] = Some(capitalize(&lo_l));
            *ls.last_mut().unwrap() = Some(capitalize(&hi_l));
            let mut r = response_scale(b - a + 1, AnchorSet::Custom, LabelMode::EndsOnly);
            r.labels = ls;
            r.values = (a as i32..=b as i32).collect();
            r.abbreviations = r
                .labels
                .iter()
                .map(|l| l.as_deref().map(abbreviate).unwrap_or_default())
                .collect();
            // End labels that are the usual agreement or frequency ends name the set.
            let ends = (lo_l.to_lowercase(), hi_l.to_lowercase());
            r.anchor_set = match (ends.0.as_str(), ends.1.as_str()) {
                ("strongly disagree", "strongly agree") | ("disagree", "agree") => {
                    AnchorSet::Agreement
                }
                ("never", "always") | ("never", "very often") => AnchorSet::Frequency,
                _ => AnchorSet::Custom,
            };
            s.response = r;
            let u = near
                .iter()
                .find(|u| range_phrase().is_match(&u.text))
                .unwrap_or(&us[c.first_unit]);
            s.provenance.push(prov(
                "anchors",
                file,
                u,
                0.8,
                "\"ranging from 1 (...) to 5 (...)\"",
            ));
        }
    }
    if found_points.is_none() {
        if let Some(a) = anchor_line(near) {
            labels = Some(a);
        }
        if let Some(m) = points_phrase().captures(&window) {
            found_points = count_word(&m[1]);
        }
        match (found_points, labels) {
            (_, Some(l)) => {
                let n = l.len() as u8;
                let mut r = response_scale(n, classify_labels(&l), LabelMode::Every);
                r.labels = l.into_iter().map(Some).collect();
                r.abbreviations = r
                    .labels
                    .iter()
                    .map(|l| l.as_deref().map(abbreviate).unwrap_or_default())
                    .collect();
                s.response = r;
                s.provenance.push(prov(
                    "anchors",
                    file,
                    &us[c.first_unit],
                    0.75,
                    "anchor words near the items",
                ));
            }
            (Some(p), None) => {
                s.response = response_scale(p, AnchorSet::Agreement, LabelMode::EndsOnly);
                s.provenance.push(prov(
                    "anchors",
                    file,
                    &us[c.first_unit],
                    0.5,
                    "point count only; labels assumed",
                ));
            }
            (None, None) => {
                if sd {
                    s.response = response_scale(7, AnchorSet::Custom, LabelMode::EndsOnly);
                    s.response.labels = vec![None; 7];
                }
                s.provenance.push(prov(
                    "anchors",
                    file,
                    &us[c.first_unit],
                    0.2,
                    "not found; assumed",
                ));
            }
        }
    }

    // Title: a named measure mentioned near the items, else the most named one in the article.
    let usable = |t: &str| {
        !STAT_NAMES
            .iter()
            .any(|w| t.split_whitespace().any(|x| x.eq_ignore_ascii_case(w)))
    };
    let article_title = article_reference(pages, blocks)
        .map(|c| c.apa)
        .unwrap_or_default();
    let in_title = named_measure()
        .captures_iter(&article_title)
        .map(|m| m[1].to_string())
        .find(|t| usable(t));
    let name_near = named_measure()
        .captures_iter(&window)
        .map(|m| m[1].to_string())
        .find(|t| usable(t));
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for m in named_measure().captures_iter(&all_text) {
        if usable(&m[1]) {
            *counts.entry(m[1].to_string()).or_default() += 1;
        }
    }
    let most = counts
        .into_iter()
        .filter(|(_, n)| *n >= 2)
        .max_by_key(|(_, n)| *n)
        .map(|(t, _)| t);
    // The nearest heading above the items: a short line with no closing full stop, in title case.
    let heading = us[c.first_unit.saturating_sub(12)..c.first_unit]
        .iter()
        .rev()
        .filter(|u| !u.in_table)
        .map(|u| tidy(&u.text))
        .find(|t| {
            let real_words = t
                .split_whitespace()
                .filter(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 3)
                .count();
            if t.chars().any(|c| c.is_ascii_digit()) || real_words * 2 < words(t).max(1) {
                return false;
            }
            let n = words(t);
            (1..=8).contains(&n)
                && !t.ends_with('.')
                && !t.to_lowercase().starts_with("table")
                && !t.to_lowercase().starts_with("figure")
                && t.split_whitespace()
                    .filter(|w| w.len() > 3)
                    .all(|w| w.chars().next().is_some_and(char::is_uppercase))
                && !STAT_NAMES.iter().any(|x| t.to_lowercase() == *x)
                && !matches!(
                    t.to_lowercase().as_str(),
                    "measures"
                        | "method"
                        | "methods"
                        | "results"
                        | "procedure"
                        | "participants"
                        | "instruments"
                        | "materials"
                        | "references"
                        | "abstract"
                        | "introduction"
                        | "discussion"
                        | "conclusion"
                        | "conclusions"
                        | "appendix"
                        | "acknowledgements"
                        | "keywords"
                )
                && !t.split_whitespace().any(|w| {
                    matches!(
                        w.to_lowercase().as_str(),
                        "factor" | "item" | "items" | "mean" | "loading" | "loadings"
                    )
                })
        });
    let title = name_near
        .or(heading)
        .or(in_title)
        .or(most)
        .unwrap_or_else(|| "Untitled measure".into());
    s.title = trim_title(&title);
    s.provenance.push(prov(
        "title",
        file,
        &us[c.first_unit],
        0.5,
        "the measure's name in the text",
    ));

    // Instructions: a sentence near the items that tells respondents what to do.
    let instr = near.iter().find(|u| {
        let l = u.text.to_lowercase();
        (l.contains("asked to")
            || l.contains("instructed")
            || l.contains("indicate")
            || l.contains("please")
            || l.contains("rate the"))
            && words(&u.text) < 120
    });
    if let Some(u) = instr {
        let sentence = quoted()
            .captures(&u.text)
            .map(|m| tidy(&m[1]))
            .unwrap_or_else(|| tidy(&u.text));
        s.instructions = Instructions {
            text: sentence,
            origin: InstructionsOrigin::Verbatim,
            house_stem: None,
        };
        s.provenance.push(prov(
            "instructions",
            file,
            u,
            0.4,
            "a sentence about the respondents' task",
        ));
    } else {
        s.instructions = prismsurvey_core::instructions::house(s.format, &s.response, None);
    }

    // Source: the article itself; any "adapted from" work is named for the researcher to find in the references.
    s.source = article_reference(pages, blocks);
    if let Some(m) = adapted().captures(&window) {
        s.adapted_from = Some(Citation {
            apa: tidy(&m[1]),
            doi: None,
            reference_manager_id: None,
        });
    }
    if c.sample_only {
        s.unparsed.push(UnparsedLine {
            line: 0,
            text: "Only sample items were found; the full scale is not in this article.".into(),
        });
    }
    s.score_range.computed = prismsurvey_core::scale::computed_range(&s);
    s
}

/// Words that mark a name as a statistic or a step of the study, not a measure ("Tucker Lewis Index", "Pilot Test").
const STAT_NAMES: &[&str] = &[
    "fit",
    "kaiser",
    "meyer",
    "olkin",
    "bartlett",
    "bartlett’s",
    "bartlett's",
    "tucker",
    "lewis",
    "validity",
    "reliability",
    "sampling",
    "adequacy",
    "pilot",
    "field",
    "chi",
    "square",
    "cronbach",
    "content",
    "discriminant",
    "convergent",
    "analyzing",
    "quantifying",
    "second",
    "final",
    "first",
    "measures",
    "confirmatory",
    "comparative",
    "goodness",
];

fn capitalize(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn trim_title(t: &str) -> String {
    // "The Negative Attitude toward Robots Scale" -> without the leading article.
    let t = tidy(t);
    for lead in ["The ", "the ", "A ", "An ", "Our ", "This "] {
        if let Some(r) = t.strip_prefix(lead) {
            return r.to_string();
        }
    }
    t
}

/// Everything detection found in one article.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub file: String,
    pub notes: Vec<String>,
    pub candidates: Vec<Candidate>,
    pub scales: Vec<Scale>,
}

/// An article read and laid out once, so the confirm screen can re-parse regions without reading it again.
pub struct Prepared {
    pub article: crate::article::Article,
    pub blocks: Vec<Block>,
    pub units: Vec<Unit>,
}

pub fn prepare(path: &std::path::Path) -> Result<Prepared, String> {
    let article = crate::article::read_article(path)?;
    let blocks = reading_order(&article.pages);
    let units = units(&blocks);
    Ok(Prepared {
        article,
        blocks,
        units,
    })
}

impl Prepared {
    pub fn detection(&self) -> Detection {
        let cands = candidates(&self.units);
        let scales = cands
            .iter()
            .map(|c| {
                parse(
                    c,
                    &self.units,
                    &self.article.pages,
                    &self.blocks,
                    &self.article.file,
                )
            })
            .collect();
        Detection {
            file: self.article.file.clone(),
            notes: self.article.notes.clone(),
            candidates: cands,
            scales,
        }
    }

    /// Re-parses what the researcher outlined on a page (specification: "Dragging a new outline on the page re-parses
    /// that region"). Every unit whose middle falls inside the outline is read; when none of them looks like an item,
    /// each line is taken as one, since the researcher drew the outline round the items.
    pub fn parse_region(&self, page: u32, region: (f64, f64, f64, f64)) -> Option<Scale> {
        let (x0, y0, x1, y1) = region;
        let inside: Vec<usize> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                let (cx, cy) = (
                    (u.region.0 + u.region.2) / 2.0,
                    (u.region.1 + u.region.3) / 2.0,
                );
                u.page == page && cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1
            })
            .map(|(i, _)| i)
            .collect();
        if inside.is_empty() {
            return None;
        }
        let mut items: Vec<ItemGuess> = inside
            .iter()
            .filter_map(|&i| item_of(&self.units[i], i))
            .collect();
        if items.is_empty() {
            items = inside
                .iter()
                .map(|&i| {
                    let (text, mark) = strip_marker(&self.units[i].text);
                    ItemGuess {
                        number: None,
                        text,
                        left: None,
                        right: None,
                        extra: vec![],
                        reverse_mark: mark,
                        unit: i,
                    }
                })
                .collect();
        }
        let mut c = score(&self.units, items);
        c.reasons.push("outlined by the researcher".into());
        Some(parse(
            &c,
            &self.units,
            &self.article.pages,
            &self.blocks,
            &self.article.file,
        ))
    }
}

/// Reads an article and returns every candidate with its parsed scale, best first.
pub fn detect(path: &std::path::Path) -> Result<Detection, String> {
    // Stored scale documents go through the importer, so the benchmark covers them the same way.
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(ext.as_str(), "doc" | "docx" | "txt") {
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let reports = prismsurvey_import::import_path(path).map_err(|e| format!("{file}: {e}"))?;
        let notes = reports.iter().flat_map(|r| r.notes.clone()).collect();
        return Ok(Detection {
            file,
            notes,
            candidates: vec![],
            scales: reports.into_iter().map(|r| r.scale).collect(),
        });
    }
    let art = crate::article::read_article(path)?;
    let blocks = reading_order(&art.pages);
    let us = units(&blocks);
    let cands = candidates(&us);
    let scales = cands
        .iter()
        .map(|c| parse(c, &us, &art.pages, &blocks, &art.file))
        .collect();
    Ok(Detection {
        file: art.file,
        notes: art.notes,
        candidates: cands,
        scales,
    })
}

// Patterns used inside loops, compiled once.
re!(footnote_start, r"^[a-c*†‡]\s");
re!(
    such_as_quote,
    r#"(?i)\b(such as|e\.g\.|for example)\s*[“"]"#
);
re!(initials, r"\b[A-Z]\.\s");
re!(
    equals_label,
    r"\b\d{1,2}\s*=\s*([A-Za-z][A-Za-z \-]{1,30}?)(?:[,;)]|\s+\d|$)"
);
re!(small_number, r"\d{1,3}");
re!(adapted, r"(?i)\badapted\s+from\s+([^.;]{3,80})");

#[cfg(test)]
mod tests {
    use super::*;

    fn u(text: &str) -> Unit {
        Unit {
            page: 1,
            region: (0.0, 0.0, 1.0, 1.0),
            text: text.into(),
            cells: vec![],
            in_table: false,
        }
    }
    fn row(cells: &[&str]) -> Unit {
        Unit {
            page: 1,
            region: (0.0, 0.0, 1.0, 1.0),
            text: cells.join(" "),
            cells: cells.iter().map(|c| c.to_string()).collect(),
            in_table: true,
        }
    }

    #[test]
    fn a_numbered_list_with_cues_is_a_candidate() {
        let us = vec![
            u("Measures"),
            u("Participants completed the Garden Joy Scale, rated on a 5-point scale ranging from 1 (strongly disagree) to 5 (strongly agree)."),
            u("1. I enjoy weeding my garden."),
            u("2. I feel calm among my plants."),
            u("3. I do not like digging. (R)"),
            u("4. My garden makes me happy."),
            u("Items 3 was reverse scored."),
        ];
        let cs = candidates(&us);
        assert_eq!(cs.len(), 1);
        let s = parse(&cs[0], &us, &[], &[], "a.pdf");
        assert_eq!(s.items.len(), 4);
        assert_eq!(s.items[2].text, "I do not like digging.");
        assert_eq!(s.items[2].reverse.state, ReverseState::Suggested);
        assert_eq!(s.response.points, 5);
        assert_eq!(s.response.labels[0].as_deref(), Some("Strongly Disagree"));
        assert_eq!(s.response.anchor_set, AnchorSet::Agreement);
        assert_eq!(s.title, "Garden Joy Scale");
    }

    #[test]
    fn a_reference_list_is_not_a_measure() {
        let us = vec![
            u("1. Smith, J. A. (2001). A study of things. Journal of Stuff, 3, 1-10."),
            u("2. Jones, B. C. (2003). Another study. Journal of Stuff, 4, 11-20."),
            u("3. Brown, D. E. (2005). Yet another. Journal of Stuff, 5, 21-30."),
        ];
        assert!(candidates(&us).is_empty());
    }

    #[test]
    fn a_table_with_codes_and_a_legend_gives_subscales_and_footnote_reversals() {
        let us = vec![
            u("Table 1 All the questionnaire items and subscales in the scale"),
            row(&["No.", "Questionnaire items", "Subscale"]),
            row(&["1", "I would feel uneasy near robots", "S1"]),
            row(&["2", "I would feel relaxed talking with robotsa", "S2"]),
            row(&["3", "I would feel nervous near robots", "S1"]),
            row(&["4", "I feel comforted being with robotsa", "S2"]),
            row(&["S1", "Unease with robots"]),
            row(&["S2", "Comfort with robots"]),
            u("a Reversed item"),
        ];
        let cs = candidates(&us);
        assert!(!cs.is_empty());
        let s = parse(&cs[0], &us, &[], &[], "a.pdf");
        assert_eq!(s.items.len(), 4, "{:?}", s.items);
        assert_eq!(s.items[1].text, "I would feel relaxed talking with robots");
        assert_eq!(s.items[1].reverse.state, ReverseState::Suggested);
        assert_eq!(s.items[0].reverse.state, ReverseState::NotReversed);
        assert_eq!(s.subscales.len(), 2);
        assert_eq!(s.subscales[0].name, "Unease with robots");
    }

    #[test]
    fn sample_items_mark_the_scale_incomplete() {
        let us = vec![u(
            "Sample items include “I enjoy weeding my garden” and “My garden makes me happy.”",
        )];
        let cs = candidates(&us);
        assert_eq!(cs.len(), 1);
        assert!(cs[0].sample_only);
    }
}
