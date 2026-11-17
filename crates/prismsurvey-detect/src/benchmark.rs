//! The accuracy benchmark (specification, "Accuracy benchmark"). The first time an article is confirmed in benchmark
//! mode, the confirmed scale is saved beside the tool's first guess as that article's answer key, in a folder outside
//! the library. The runner detects every file again, compares with the keys and writes a plain report: one line per
//! article and the totals below, against the version 1 targets. Each build reruns the same keys, so a change that makes
//! detection worse shows at once. Articles never leave the computer.

use crate::measure::{detect, Detection};
use prismsurvey_core::text::wording_match;
use prismsurvey_core::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerKey {
    /// The article's file name, and its size so a changed file is noticed.
    pub file: String,
    pub size: u64,
    pub saved_at: String,
    /// What the tool offered first, before any correction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_guess: Option<Scale>,
    /// The measure as the researcher confirmed it. An article can hold several measures.
    pub confirmed: Vec<Scale>,
}

/// Where the keys live: `%APPDATA%\Prism Survey\Benchmark keys`, outside the library.
pub fn default_key_folder() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Prism Survey")
        .join("Benchmark keys")
}

fn key_path(folder: &Path, file: &str) -> PathBuf {
    let safe: String = file
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    folder.join(format!("{safe}.key.json"))
}

pub fn load_key(folder: &Path, file: &str) -> Option<AnswerKey> {
    serde_json::from_str(&std::fs::read_to_string(key_path(folder, file)).ok()?).ok()
}

/// Saves (or adds to) an article's answer key.
pub fn save_key(
    folder: &Path,
    article: &Path,
    first_guess: Option<Scale>,
    confirmed: Scale,
    now: &str,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(folder)?;
    let file = article
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let size = std::fs::metadata(article).map(|m| m.len()).unwrap_or(0);
    let mut key = load_key(folder, &file).unwrap_or(AnswerKey {
        file: file.clone(),
        size,
        saved_at: now.into(),
        first_guess: first_guess.clone(),
        confirmed: vec![],
    });
    key.saved_at = now.into();
    key.size = size;
    if key.first_guess.is_none() {
        key.first_guess = first_guess;
    }
    // A measure confirmed again replaces the earlier confirmation of the same measure.
    key.confirmed
        .retain(|s| s.id != confirmed.id && !same_measure(s, &confirmed));
    key.confirmed.push(confirmed);
    let path = key_path(folder, &file);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&key).unwrap_or_default(),
    )?;
    Ok(path)
}

fn item_text(i: &Item) -> String {
    if i.text.trim().is_empty() {
        format!(
            "{} {}",
            i.left.clone().unwrap_or_default(),
            i.right.clone().unwrap_or_default()
        )
    } else {
        i.text.clone()
    }
}

/// Two scales are the same measure when most of their items match by wording.
fn same_measure(a: &Scale, b: &Scale) -> bool {
    let matched = a
        .items
        .iter()
        .filter(|x| {
            b.items
                .iter()
                .any(|y| wording_match(&item_text(x), &item_text(y)) >= 0.8)
        })
        .count();
    matched * 2 >= a.items.len().max(1)
}

/// Pairs the key's items with the detected items, best wording first, each used once.
fn pair_items(key: &Scale, got: &Scale) -> Vec<(usize, usize, f64)> {
    let mut all = Vec::new();
    for (i, k) in key.items.iter().enumerate() {
        for (j, g) in got.items.iter().enumerate() {
            let m = wording_match(&item_text(k), &item_text(g));
            if m >= 0.6 {
                all.push((i, j, m));
            }
        }
    }
    all.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let (mut ui, mut uj) = (HashSet::new(), HashSet::new());
    all.into_iter()
        .filter(|(i, j, _)| ui.insert(*i) && uj.insert(*j))
        .collect()
}

/// How one confirmed measure compares with what detection found.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasureScore {
    pub found: bool,
    pub anchors_right: bool,
    pub items_exact: bool,
    pub items_total: usize,
    pub items_text_right: usize,
    pub reversed_total: usize,
    pub reversed_suggested: usize,
    pub suggestions_total: usize,
    pub suggestions_right: usize,
    pub subscale_items_total: usize,
    pub subscale_items_right: usize,
    pub citation_right: bool,
}

fn labels(s: &Scale) -> Vec<String> {
    s.response
        .labels
        .iter()
        .map(|l| l.clone().unwrap_or_default().to_lowercase())
        .collect()
}

fn norm(t: &str) -> String {
    prismsurvey_core::text::words(t).join(" ")
}

pub fn score_measure(key: &Scale, d: &Detection) -> MeasureScore {
    let mut out = MeasureScore {
        items_total: key.items.len(),
        ..Default::default()
    };
    out.reversed_total = key.items.iter().filter(|i| i.reverse.is_reversed()).count();
    // The detected scale that matches the key best.
    let best = d
        .scales
        .iter()
        .map(|s| (s, pair_items(key, s)))
        .max_by_key(|(_, p)| p.len());
    let Some((got, pairs)) = best else { return out };
    out.found = pairs.len() * 2 >= key.items.len().max(1);
    if !out.found {
        return out;
    }
    out.anchors_right = key.response.points == got.response.points && labels(key) == labels(got);
    out.items_exact = got.items.len() == key.items.len() && pairs.len() == key.items.len();
    out.items_text_right = pairs
        .iter()
        .filter(|(i, j, _)| norm(&item_text(&key.items[*i])) == norm(&item_text(&got.items[*j])))
        .count();
    let suggested = |i: &Item| {
        matches!(
            i.reverse.state,
            ReverseState::Suggested | ReverseState::Confirmed
        )
    };
    out.reversed_suggested = pairs
        .iter()
        .filter(|(i, j, _)| key.items[*i].reverse.is_reversed() && suggested(&got.items[*j]))
        .count();
    out.suggestions_total = got.items.iter().filter(|i| suggested(i)).count();
    out.suggestions_right = pairs
        .iter()
        .filter(|(i, j, _)| key.items[*i].reverse.is_reversed() && suggested(&got.items[*j]))
        .count();
    // Subscales: each detected subscale stands for the key subscale it shares most items with.
    if !key.subscales.is_empty() {
        let kn = key.items.len() as u32;
        let gn = got.items.len() as u32;
        let key_of: HashMap<u32, usize> = key
            .subscales
            .iter()
            .enumerate()
            .flat_map(|(k, s)| s.items.resolve(kn).into_iter().map(move |n| (n, k)))
            .collect();
        let got_of: HashMap<u32, usize> = got
            .subscales
            .iter()
            .enumerate()
            .flat_map(|(k, s)| s.items.resolve(gn).into_iter().map(move |n| (n, k)))
            .collect();
        let mut overlap: HashMap<(usize, usize), usize> = HashMap::new();
        for (i, j, _) in &pairs {
            if let (Some(a), Some(b)) = (
                key_of.get(&key.items[*i].number),
                got_of.get(&got.items[*j].number),
            ) {
                *overlap.entry((*b, *a)).or_default() += 1;
            }
        }
        let mut stands_for: HashMap<usize, usize> = HashMap::new();
        for ((g, k), n) in &overlap {
            if stands_for
                .get(g)
                .is_none_or(|kk| overlap.get(&(*g, *kk)).copied().unwrap_or(0) < *n)
            {
                stands_for.insert(*g, *k);
            }
        }
        out.subscale_items_total = key_of.len();
        out.subscale_items_right = pairs
            .iter()
            .filter(|(i, j, _)| {
                let k = key_of.get(&key.items[*i].number);
                let g = got_of
                    .get(&got.items[*j].number)
                    .and_then(|g| stands_for.get(g));
                k.is_some() && k == g
            })
            .count();
    }
    out.citation_right = match (&key.source, &got.source) {
        (Some(k), Some(g)) => match (&k.doi, &g.doi) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            _ => wording_match(&k.apa, &g.apa) >= 0.6,
        },
        _ => false,
    };
    out
}

/// One article's line in the report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleResult {
    pub file: String,
    pub has_key: bool,
    pub candidates: usize,
    pub top: Option<String>,
    pub measures: Vec<MeasureScore>,
    pub error: Option<String>,
}

pub fn run(files: &[PathBuf], keys: &Path) -> Vec<ArticleResult> {
    files
        .iter()
        .map(|f| {
            let file = f
                .file_name()
                .map(|x| x.to_string_lossy().into_owned())
                .unwrap_or_default();
            let key = load_key(keys, &file);
            match detect(f) {
                Ok(d) => ArticleResult {
                    file: file.clone(),
                    has_key: key.is_some(),
                    candidates: d.candidates.len(),
                    top: d
                        .scales
                        .first()
                        .map(|s| format!("{} ({} items)", s.title, s.items.len())),
                    measures: key
                        .map(|k| k.confirmed.iter().map(|m| score_measure(m, &d)).collect())
                        .unwrap_or_default(),
                    error: None,
                },
                Err(e) => ArticleResult {
                    file,
                    has_key: key.is_some(),
                    candidates: 0,
                    top: None,
                    measures: vec![],
                    error: Some(e),
                },
            }
        })
        .collect()
}

fn pct(a: usize, b: usize) -> String {
    if b == 0 {
        "n/a".into()
    } else {
        format!("{:.0}%", a as f64 * 100.0 / b as f64)
    }
}

/// The plain report: one line per article, then the totals against the version 1 targets.
pub fn report(results: &[ArticleResult], run_at: &str) -> String {
    let mut s = String::new();
    s.push_str("Prism Survey detection benchmark\r\n");
    s.push_str(&format!("Run: {run_at}\r\n"));
    let keyed = results.iter().filter(|r| r.has_key).count();
    s.push_str(&format!(
        "Articles: {} ({keyed} with an answer key)\r\n\r\n",
        results.len()
    ));
    for r in results {
        let status = if let Some(e) = &r.error {
            format!("could not be read: {e}")
        } else if !r.has_key {
            format!(
                "no answer key yet; {} candidate(s); first: {}",
                r.candidates,
                r.top.clone().unwrap_or_else(|| "none".into())
            )
        } else {
            r.measures
                .iter()
                .map(|m| {
                    if !m.found {
                        "measure NOT found".to_string()
                    } else {
                        format!(
                            "found; anchors {}; items {}; text {}/{}; reversed {}/{}; subscales {}/{}; citation {}",
                            if m.anchors_right { "right" } else { "wrong" },
                            if m.items_exact { "exact" } else { "off" },
                            m.items_text_right,
                            m.items_total,
                            m.reversed_suggested,
                            m.reversed_total,
                            m.subscale_items_right,
                            m.subscale_items_total,
                            if m.citation_right { "right" } else { "wrong" }
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(" | ")
        };
        s.push_str(&format!("{}: {status}\r\n", r.file));
    }
    let ms: Vec<&MeasureScore> = results.iter().flat_map(|r| r.measures.iter()).collect();
    let sum = |f: &dyn Fn(&MeasureScore) -> usize| ms.iter().map(|m| f(m)).sum::<usize>();
    let found: Vec<&&MeasureScore> = ms.iter().filter(|m| m.found).collect();
    s.push_str("\r\nTotals against the version 1 targets\r\n");
    let lines = [
        (
            "Measure found in the article",
            pct(found.len(), ms.len()),
            "90% of measures",
        ),
        (
            "Point count and anchors correct",
            pct(
                found.iter().filter(|m| m.anchors_right).count(),
                found.len(),
            ),
            "95% of scales",
        ),
        (
            "Items found, none missing or extra",
            pct(found.iter().filter(|m| m.items_exact).count(), found.len()),
            "90% of scales",
        ),
        (
            "Item text matching the source",
            pct(
                sum(&|m| m.items_text_right),
                sum(&|m| if m.found { m.items_total } else { 0 }),
            ),
            "98% of items",
        ),
        (
            "Reverse items suggested (of those reversed)",
            pct(
                sum(&|m| m.reversed_suggested),
                sum(&|m| if m.found { m.reversed_total } else { 0 }),
            ),
            "90%",
        ),
        (
            "Reverse suggestions that are right",
            pct(sum(&|m| m.suggestions_right), sum(&|m| m.suggestions_total)),
            "80%",
        ),
        (
            "Items placed in the right subscale",
            pct(
                sum(&|m| m.subscale_items_right),
                sum(&|m| m.subscale_items_total),
            ),
            "90% of items",
        ),
        (
            "Source citation correct",
            pct(
                found.iter().filter(|m| m.citation_right).count(),
                found.len(),
            ),
            "95% of scales",
        ),
    ];
    for (what, got, target) in lines {
        s.push_str(&format!("{what}: {got} (target {target})\r\n"));
    }
    s.push_str("\r\nResults are with the rules alone; the optional model is not used.\r\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismsurvey_core::scale::new_blank;

    fn scale(items: &[&str], reversed: &[u32]) -> Scale {
        let mut s = new_blank(Format::LikertBlankLine);
        s.items = items
            .iter()
            .enumerate()
            .map(|(k, t)| {
                let mut i = Item::new(k as u32 + 1, *t);
                if reversed.contains(&(k as u32 + 1)) {
                    i.reverse = ReverseFlag {
                        state: ReverseState::Confirmed,
                        reason: None,
                    };
                }
                i
            })
            .collect();
        s
    }

    #[test]
    fn a_detection_is_scored_against_its_key() {
        let key = scale(
            &["I like maps.", "I read maps daily.", "I never use maps."],
            &[3],
        );
        let mut got = scale(
            &["I like maps.", "I read maps dally.", "I never use maps."],
            &[],
        );
        got.items[2].reverse.state = ReverseState::Suggested;
        let d = Detection {
            file: "a.pdf".into(),
            notes: vec![],
            candidates: vec![],
            scales: vec![got],
        };
        let m = score_measure(&key, &d);
        assert!(m.found && m.items_exact && m.anchors_right);
        assert_eq!(m.items_text_right, 2, "one OCR slip");
        assert_eq!(
            (
                m.reversed_suggested,
                m.reversed_total,
                m.suggestions_right,
                m.suggestions_total
            ),
            (1, 1, 1, 1)
        );
        let r = report(
            &[ArticleResult {
                file: "a.pdf".into(),
                has_key: true,
                candidates: 1,
                top: None,
                measures: vec![m],
                error: None,
            }],
            "now",
        );
        assert!(r.contains("Measure found in the article: 100%"));
        assert!(r.contains("Item text matching the source: 67%"));
    }

    #[test]
    fn keys_round_trip() {
        let dir = std::env::temp_dir().join(format!("ps-keys-{}", std::process::id()));
        let article = dir.join("art.pdf");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&article, b"%PDF").unwrap();
        let s = scale(&["a b c", "d e f"], &[]);
        save_key(&dir, &article, Some(s.clone()), s.clone(), "t").unwrap();
        save_key(&dir, &article, None, s.clone(), "t2").unwrap();
        let k = load_key(&dir, "art.pdf").unwrap();
        assert_eq!(
            k.confirmed.len(),
            1,
            "confirming the same measure again replaces it"
        );
        assert!(k.first_guess.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
