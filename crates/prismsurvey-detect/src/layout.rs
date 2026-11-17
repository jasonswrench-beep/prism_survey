//! Rebuilding the reading order (specification, "Rebuild the reading order"): words become line pieces, line pieces
//! are placed in columns, running heads, page numbers and repeated footers are set aside, and a page is read band by
//! band (a wide block such as a title or a table breaks the columns), left column first. Rows whose pieces line up in
//! two or more cells become tables, read row by row.

use crate::pdf::{Page, Word};
use serde::Serialize;
use std::collections::HashMap;

/// A run of words on one baseline with no wide gap inside.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Piece {
    pub text: String,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub size: f64,
    /// The lowest word confidence (1.0 for a text layer).
    pub confidence: f64,
}

/// A visual line: one or more pieces at the same height. A line with two or more pieces is a table row candidate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub page: u32,
    pub pieces: Vec<Piece>,
    pub y0: f64,
    pub y1: f64,
    pub x0: f64,
    pub x1: f64,
    pub size: f64,
}

impl Line {
    pub fn text(&self) -> String {
        self.pieces
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\t")
    }
    pub fn plain(&self) -> String {
        self.pieces
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    Paragraph,
    Table,
}

/// A block in reading order: a paragraph (its lines joined) or a table (its rows and cells).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub kind: BlockKind,
    pub page: u32,
    pub lines: Vec<Line>,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub size: f64,
}

impl Block {
    /// Paragraph text with hard-wrapped lines joined and hyphens put back where a word was split.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for l in &self.lines {
            let t = l.plain();
            if out.ends_with('-')
                && t.chars().next().is_some_and(|c| c.is_lowercase())
                && out.chars().rev().nth(1).is_some_and(char::is_alphabetic)
            {
                out.pop();
                out.push_str(&t);
            } else {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&t);
            }
        }
        out
    }
    /// Table rows as cells.
    pub fn rows(&self) -> Vec<Vec<String>> {
        self.lines
            .iter()
            .map(|l| l.pieces.iter().map(|p| p.text.clone()).collect())
            .collect()
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 10.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[v.len() / 2]
}

/// Words on one baseline, split into pieces at wide gaps.
fn lines_of(page: &Page) -> Vec<Line> {
    let mut words: Vec<&Word> = page
        .words
        .iter()
        .filter(|w| !w.text.trim().is_empty())
        .collect();
    words.sort_by(|a, b| {
        a.cy()
            .partial_cmp(&b.cy())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal))
    });
    // Group into rows by vertical overlap.
    let mut rows: Vec<Vec<&Word>> = Vec::new();
    for w in words {
        let h = (w.y1 - w.y0).max(1.0);
        let fits = rows.last().is_some_and(|r| {
            let (ry0, ry1) = (
                r.iter().map(|x| x.y0).fold(f64::MAX, f64::min),
                r.iter().map(|x| x.y1).fold(f64::MIN, f64::max),
            );
            let overlap = (w.y1.min(ry1) - w.y0.max(ry0)).max(0.0);
            overlap >= 0.5 * h.min(ry1 - ry0)
        });
        if fits {
            rows.last_mut().unwrap().push(w);
        } else {
            rows.push(vec![w]);
        }
    }
    let mut out = Vec::new();
    for mut r in rows {
        r.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));
        let size = median(r.iter().map(|w| w.size).collect());
        let mut pieces: Vec<Piece> = Vec::new();
        for w in r {
            let joins = pieces
                .last()
                .is_some_and(|p| w.x0 - p.x1 < (1.2 * size).max(4.0));
            if joins {
                let p = pieces.last_mut().unwrap();
                p.text.push(' ');
                p.text.push_str(&w.text);
                p.x1 = p.x1.max(w.x1);
                p.y0 = p.y0.min(w.y0);
                p.y1 = p.y1.max(w.y1);
                p.size = p.size.max(w.size);
                p.confidence = p.confidence.min(w.confidence);
            } else {
                pieces.push(Piece {
                    text: w.text.clone(),
                    x0: w.x0,
                    y0: w.y0,
                    x1: w.x1,
                    y1: w.y1,
                    size: w.size,
                    confidence: w.confidence,
                });
            }
        }
        let y0 = pieces.iter().map(|p| p.y0).fold(f64::MAX, f64::min);
        let y1 = pieces.iter().map(|p| p.y1).fold(f64::MIN, f64::max);
        let x0 = pieces.iter().map(|p| p.x0).fold(f64::MAX, f64::min);
        let x1 = pieces.iter().map(|p| p.x1).fold(f64::MIN, f64::max);
        out.push(Line {
            page: page.number,
            pieces,
            y0,
            y1,
            x0,
            x1,
            size,
        });
    }
    out
}

/// The x where two columns meet, if the page has two: a band of the page's width near the middle that the body text
/// does not cross.
fn gutter(lines: &[Line], width: f64) -> Option<f64> {
    let bins = 200usize;
    let mut cover = vec![0usize; bins];
    let mut counted = 0usize;
    for l in lines {
        for p in &l.pieces {
            // Pieces that span most of the page (titles, wide tables) do not tell where the columns are.
            if p.x1 - p.x0 > 0.6 * width {
                continue;
            }
            counted += 1;
            let a = ((p.x0 / width) * bins as f64).clamp(0.0, bins as f64 - 1.0) as usize;
            let b = ((p.x1 / width) * bins as f64).clamp(0.0, bins as f64 - 1.0) as usize;
            for c in cover.iter_mut().take(b + 1).skip(a) {
                *c += 1;
            }
        }
    }
    if counted < 10 {
        return None;
    }
    // Look for the emptiest stretch between 35% and 65% of the width.
    let (lo, hi) = (bins * 35 / 100, bins * 65 / 100);
    let (best, &min) = cover[lo..hi].iter().enumerate().min_by_key(|(_, c)| **c)?;
    let peak = *cover.iter().max()?;
    // Both sides must carry text for it to be a gutter.
    let left = cover[..lo].iter().copied().max().unwrap_or(0);
    let right = cover[hi..].iter().copied().max().unwrap_or(0);
    if min * 20 <= peak && left * 4 >= peak && right * 4 >= peak {
        Some((lo + best) as f64 / bins as f64 * width)
    } else {
        None
    }
}

/// Running heads, page numbers and repeated footers: short lines in the top or bottom tenth of the page that are a
/// number or repeat on other pages (digits ignored).
fn furniture(pages: &[(u32, f64, Vec<Line>)]) -> HashMap<(u32, usize), bool> {
    let key = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphabetic())
            .collect::<String>()
            .to_lowercase()
    };
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (_, h, lines) in pages {
        for l in lines {
            if l.y0 < 0.1 * h || l.y1 > 0.9 * h {
                let k = key(&l.plain());
                if !k.is_empty() {
                    *counts.entry(k).or_default() += 1;
                }
            }
        }
    }
    let mut out = HashMap::new();
    for (n, h, lines) in pages {
        for (i, l) in lines.iter().enumerate() {
            let edge = l.y0 < 0.1 * h || l.y1 > 0.9 * h;
            if !edge {
                continue;
            }
            let t = l.plain();
            let k = key(&t);
            let number_only = !t.is_empty()
                && t.chars()
                    .all(|c| c.is_ascii_digit() || c.is_whitespace() || c == '|');
            let repeats = !k.is_empty()
                && counts.get(&k).copied().unwrap_or(0) >= 2
                && t.split_whitespace().count() <= 20;
            if number_only || repeats {
                out.insert((*n, i), true);
            }
        }
    }
    out
}

/// A line that is a table row: two or more pieces, at least one of them short.
fn is_row(l: &Line) -> bool {
    l.pieces.len() >= 2
}

/// How many row lines follow from `i`.
fn row_run(lines: &[Line], i: usize) -> usize {
    lines[i..].iter().take_while(|l| is_row(l)).count()
}

/// Turns one column's lines (top to bottom) into paragraphs and tables.
fn blocks_of(lines: Vec<Line>, body: f64) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        // A run of three or more row lines is a table.
        let j = i + row_run(&lines, i);
        if j - i >= 3 {
            let rows: Vec<Line> = lines[i..j].to_vec();
            out.push(make_block(BlockKind::Table, rows));
            i = j;
            continue;
        }
        // Otherwise a paragraph: lines close together, of one size, with no new item number or indent.
        let start = i;
        i += 1;
        while i < lines.len() {
            let (prev, next) = (&lines[i - 1], &lines[i]);
            let gap = next.y0 - prev.y1;
            let size_change = (next.size - prev.size).abs() > 0.15 * body;
            let starts_item = starts_like_item(&next.plain());
            let indent = next.x0 - lines[start].x0 > 1.5 * body && !starts_item;
            if gap > 0.6 * body
                || size_change
                || starts_item
                || (indent && i - start > 1)
                || row_run(&lines, i) >= 3
            {
                break;
            }
            i += 1;
        }
        out.push(make_block(BlockKind::Paragraph, lines[start..i].to_vec()));
    }
    out
}

/// "1.", "12)", "(3)", "a.", "R1." and "____ 1." at the start of a line.
pub fn starts_like_item(t: &str) -> bool {
    let t = t.trim_start_matches(['_', ' ']);
    let mut chars = t.chars();
    let first = chars.next();
    match first {
        Some(c) if c.is_ascii_digit() => {
            let rest: String = t
                .chars()
                .skip_while(|c| c.is_ascii_digit())
                .take(2)
                .collect();
            rest.starts_with(". ") || rest.starts_with(") ") || rest.starts_with('.') && t.len() < 5
        }
        Some('(') => t.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) && t.contains(')'),
        _ => false,
    }
}

fn make_block(kind: BlockKind, lines: Vec<Line>) -> Block {
    let x0 = lines.iter().map(|l| l.x0).fold(f64::MAX, f64::min);
    let x1 = lines.iter().map(|l| l.x1).fold(f64::MIN, f64::max);
    let y0 = lines.iter().map(|l| l.y0).fold(f64::MAX, f64::min);
    let y1 = lines.iter().map(|l| l.y1).fold(f64::MIN, f64::max);
    let size = median(lines.iter().map(|l| l.size).collect());
    Block {
        kind,
        page: lines.first().map(|l| l.page).unwrap_or(0),
        lines,
        x0,
        y0,
        x1,
        y1,
        size,
    }
}

/// Every page's blocks in reading order, with running heads and page numbers left out.
pub fn reading_order(pages: &[Page]) -> Vec<Block> {
    let lined: Vec<(u32, f64, Vec<Line>)> = pages
        .iter()
        .map(|p| (p.number, p.height, lines_of(p)))
        .collect();
    let furn = furniture(&lined);
    let body = median(
        lined
            .iter()
            .flat_map(|(_, _, ls)| ls.iter().map(|l| l.size))
            .collect(),
    );
    let mut out = Vec::new();
    for (page, (n, _h, lines)) in pages.iter().zip(lined) {
        let kept: Vec<Line> = lines
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !furn.contains_key(&(n, *i)))
            .map(|(_, l)| l)
            .collect();
        let Some(g) = gutter(&kept, page.width) else {
            out.extend(blocks_of(kept, body));
            continue;
        };
        // Split each line at the gutter; a line whose one piece crosses it spans both columns.
        let mut bands: Vec<(Vec<Line>, Vec<Line>, Option<Line>)> =
            vec![(Vec::new(), Vec::new(), None)];
        for l in kept {
            let crosses = l.pieces.iter().any(|p| p.x0 < g - 2.0 && p.x1 > g + 2.0);
            if crosses {
                bands.push((Vec::new(), Vec::new(), Some(l)));
                bands.push((Vec::new(), Vec::new(), None));
                continue;
            }
            let (left, right): (Vec<Piece>, Vec<Piece>) =
                l.pieces.iter().cloned().partition(|p| p.x1 <= g + 2.0);
            let band = bands.last_mut().unwrap();
            for (side, pieces) in [(&mut band.0, left), (&mut band.1, right)] {
                if !pieces.is_empty() {
                    let mut part = l.clone();
                    part.x0 = pieces.iter().map(|p| p.x0).fold(f64::MAX, f64::min);
                    part.x1 = pieces.iter().map(|p| p.x1).fold(f64::MIN, f64::max);
                    part.pieces = pieces;
                    side.push(part);
                }
            }
        }
        // Consecutive spanning lines (a wide table) are kept together.
        let mut spanning: Vec<Line> = Vec::new();
        for (left, right, span) in bands {
            if let Some(s) = span {
                spanning.push(s);
                continue;
            }
            if !spanning.is_empty() {
                out.extend(blocks_of(std::mem::take(&mut spanning), body));
            }
            out.extend(blocks_of(left, body));
            out.extend(blocks_of(right, body));
        }
        if !spanning.is_empty() {
            out.extend(blocks_of(spanning, body));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::TextSource;

    fn word(t: &str, x: f64, y: f64) -> Word {
        Word {
            text: t.into(),
            x0: x,
            y0: y,
            x1: x + 6.0 * t.len() as f64,
            y1: y + 10.0,
            size: 10.0,
            confidence: 1.0,
        }
    }

    fn page(words: Vec<Word>) -> Page {
        Page {
            number: 1,
            width: 600.0,
            height: 800.0,
            words,
            source: TextSource::TextLayer,
            garbled: false,
            failed: false,
        }
    }

    #[test]
    fn two_columns_read_left_first() {
        let mut ws = Vec::new();
        for i in 0..20 {
            let y = 100.0 + i as f64 * 12.0;
            ws.push(word(&format!("left{i}"), 60.0, y));
            ws.push(word("text", 120.0, y));
            ws.push(word(&format!("right{i}"), 330.0, y));
            ws.push(word("text", 390.0, y));
        }
        let blocks = reading_order(&[page(ws)]);
        let all: Vec<String> = blocks.iter().map(|b| b.text()).collect();
        let joined = all.join(" | ");
        let l19 = joined.find("left19").unwrap();
        let r0 = joined.find("right0").unwrap();
        assert!(l19 < r0, "{joined}");
    }

    #[test]
    fn numbered_items_start_their_own_paragraphs() {
        let ws = vec![
            word("1.", 60.0, 100.0),
            word("I", 80.0, 100.0),
            word("like", 90.0, 100.0),
            word("tea.", 120.0, 100.0),
            word("2.", 60.0, 112.0),
            word("I", 80.0, 112.0),
            word("like", 90.0, 112.0),
            word("coffee.", 120.0, 112.0),
        ];
        let blocks = reading_order(&[page(ws)]);
        assert_eq!(
            blocks.len(),
            2,
            "{:?}",
            blocks.iter().map(|b| b.text()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn aligned_rows_make_a_table() {
        let mut ws = Vec::new();
        for (i, item) in ["I enjoy maps.", "I read maps.", "I draw maps."]
            .iter()
            .enumerate()
        {
            let y = 100.0 + i as f64 * 12.0;
            ws.push(word(item, 60.0, y));
            ws.push(word(".71", 300.0, y));
            ws.push(word(".12", 380.0, y));
        }
        let blocks = reading_order(&[page(ws)]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, BlockKind::Table);
        assert_eq!(blocks[0].rows()[1], vec!["I read maps.", ".71", ".12"]);
    }

    #[test]
    fn page_numbers_are_left_out() {
        let ws = vec![
            word("Some", 60.0, 400.0),
            word("text.", 100.0, 400.0),
            word("17", 300.0, 760.0),
        ];
        let blocks = reading_order(&[page(ws)]);
        assert!(blocks.iter().all(|b| !b.text().contains("17")));
    }

    #[test]
    fn item_starts() {
        assert!(starts_like_item("1. I like tea"));
        assert!(starts_like_item("12) I like tea"));
        assert!(starts_like_item("(3) I like tea"));
        assert!(starts_like_item("____ 4. I like tea"));
        assert!(!starts_like_item("1990 was a year"));
        assert!(!starts_like_item("3.5 points"));
    }
}
