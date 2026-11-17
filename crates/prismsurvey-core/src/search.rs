//! Search over the library (specification addendum, section 2): titles, short names, tags, subscale names, item text,
//! sources and instructions, with word stems, small spelling slips forgiven, "quoted phrases", and fields named in the
//! query (`tag:health`, `author:wrench`, `year:2001`, `title:anxiety`, `item:nervous`). Filters narrow by format,
//! point count, item count, licence and tag. Everything runs on this computer.

use crate::model::{Format, LicenceStatus, Scale};
use crate::text::{edit_distance, stem, words};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The searchable fields and how much a match in each counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    Title,
    Tags,
    Subscales,
    Items,
    Source,
    Instructions,
    /// An add-on entry's purpose, format, sample, reliability, validity and critique.
    Description,
}

impl Field {
    fn weight(self) -> f64 {
        match self {
            Field::Title => 4.0,
            Field::Tags => 3.0,
            Field::Subscales => 2.0,
            Field::Source => 1.5,
            Field::Description => 1.2,
            Field::Items => 1.0,
            Field::Instructions => 0.5,
        }
    }
    fn named(s: &str) -> Option<Field> {
        Some(match s {
            "title" | "name" => Field::Title,
            "tag" | "tags" => Field::Tags,
            "subscale" | "subscales" | "factor" => Field::Subscales,
            "item" | "items" => Field::Items,
            "source" | "author" | "journal" | "year" | "citation" => Field::Source,
            "instructions" => Field::Instructions,
            "purpose" | "about" | "description" => Field::Description,
            _ => return None,
        })
    }
}

/// One searchable document: a library scale or an add-on entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Doc {
    /// Where the document comes from: a library file path, or an add-on entry id.
    pub key: String,
    pub title: String,
    pub format: Option<Format>,
    pub points: Option<u8>,
    pub items: usize,
    pub tags: Vec<String>,
    pub licence: Option<LicenceStatus>,
    /// "library" or the add-on's name.
    pub collection: String,
    pub fields: Vec<(Field, String)>,
}

impl Doc {
    pub fn from_scale(key: &str, s: &Scale) -> Doc {
        let mut fields = vec![(Field::Title, s.title.clone())];
        if let Some(n) = &s.short_name {
            fields.push((Field::Title, n.clone()));
        }
        fields.extend(s.tags.iter().map(|t| (Field::Tags, t.clone())));
        for sub in &s.subscales {
            fields.push((Field::Subscales, sub.name.clone()));
            fields.extend(
                sub.other_names
                    .iter()
                    .map(|n| (Field::Subscales, n.clone())),
            );
        }
        for it in &s.items {
            let t = if it.text.trim().is_empty() {
                format!(
                    "{} {}",
                    it.left.clone().unwrap_or_default(),
                    it.right.clone().unwrap_or_default()
                )
            } else {
                it.text.clone()
            };
            fields.push((Field::Items, t));
        }
        for c in [&s.source, &s.adapted_from].into_iter().flatten() {
            fields.push((Field::Source, c.apa.clone()));
        }
        fields.push((Field::Instructions, s.instructions.text.clone()));
        Doc {
            key: key.to_string(),
            title: s.title.clone(),
            format: Some(s.format),
            points: Some(s.response.points),
            items: s.items.len(),
            tags: s.tags.clone(),
            licence: Some(s.licence.status),
            collection: "library".into(),
            fields,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Filters {
    pub format: Option<Format>,
    pub points: Option<u8>,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub licence: Option<LicenceStatus>,
    pub tag: Option<String>,
    pub collection: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub key: String,
    pub title: String,
    pub collection: String,
    pub score: f64,
    /// The text that matched best, shortened around the match.
    pub snippet: String,
    pub field: Field,
}

/// The stems a query word stands for, each with how much a match counts (1.0 for the word itself).
type Alternatives = Vec<(String, f64)>;

/// A query: words (each may be limited to a field), and quoted phrases.
#[derive(Debug, Default)]
struct Query {
    /// (field, stem, the word as typed)
    terms: Vec<(Option<Field>, String, String)>,
    phrases: Vec<(Option<Field>, Vec<String>)>,
}

fn parse(q: &str) -> Query {
    let mut out = Query::default();
    let mut rest = q.to_string();
    // Quoted phrases first, with an optional field in front ("item:\"feel nervous\"").
    let re = regex::Regex::new(r#"(?:(\w+):)?"([^"]+)""#).unwrap();
    for c in re.captures_iter(q) {
        let field = c
            .get(1)
            .and_then(|f| Field::named(&f.as_str().to_lowercase()));
        let ws: Vec<String> = words(&c[2]).iter().map(|w| stem(w)).collect();
        if !ws.is_empty() {
            out.phrases.push((field, ws));
        }
        rest = rest.replace(&c[0], " ");
    }
    for tok in rest.split_whitespace() {
        let (field, word) = match tok.split_once(':') {
            Some((f, w)) if Field::named(&f.to_lowercase()).is_some() => {
                (Field::named(&f.to_lowercase()), w)
            }
            _ => (None, tok),
        };
        for w in words(word) {
            out.terms.push((field, stem(&w), w));
        }
    }
    out
}

fn passes(d: &Doc, f: &Filters) -> bool {
    f.format.is_none_or(|x| d.format == Some(x))
        && f.points.is_none_or(|x| d.points == Some(x))
        && f.min_items.is_none_or(|x| d.items >= x)
        && f.max_items.is_none_or(|x| d.items <= x)
        && f.licence.is_none_or(|x| d.licence == Some(x))
        && f.tag.as_ref().is_none_or(|t| {
            d.tags
                .iter()
                .any(|x| crate::tags::key(x) == crate::tags::key(t))
        })
        && f.collection.as_ref().is_none_or(|c| &d.collection == c)
}

fn snippet(text: &str, hit: &str) -> String {
    let lower = text.to_lowercase();
    let at = lower.find(hit).unwrap_or(0);
    let start = text[..at]
        .char_indices()
        .rev()
        .nth(60)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let s: String = text[start..].chars().take(160).collect();
    let mut out = if start > 0 {
        format!("…{}", s.trim_start())
    } else {
        s
    };
    if text.len() > start + out.len() {
        out.push('…');
    }
    out
}

/// Ranks documents for a query. An empty query lists every document that passes the filters, by title.
pub fn search(docs: &[Doc], query: &str, filters: &Filters) -> Vec<Hit> {
    let q = parse(query);
    let pool: Vec<&Doc> = docs.iter().filter(|d| passes(d, filters)).collect();
    if q.terms.is_empty() && q.phrases.is_empty() {
        let mut all: Vec<Hit> = pool
            .iter()
            .map(|d| Hit {
                key: d.key.clone(),
                title: d.title.clone(),
                collection: d.collection.clone(),
                score: 0.0,
                snippet: String::new(),
                field: Field::Title,
            })
            .collect();
        all.sort_by_key(|h| h.title.to_lowercase());
        return all;
    }
    // Each document's fields as stemmed words, and how many documents hold each stem (for rarity).
    let tokenised: Vec<Vec<(Field, Vec<String>, &String)>> = pool
        .iter()
        .map(|d| {
            d.fields
                .iter()
                .map(|(f, t)| (*f, words(t).iter().map(|w| stem(w)).collect::<Vec<_>>(), t))
                .collect()
        })
        .collect();
    let mut df: HashMap<&str, usize> = HashMap::new();
    for doc in &tokenised {
        let mut seen = HashSet::new();
        for (_, ws, _) in doc {
            for w in ws {
                if seen.insert(w.as_str()) {
                    *df.entry(w.as_str()).or_default() += 1;
                }
            }
        }
    }
    // Whole words as written in the library, each with its stem, so a slip is judged against real words.
    let mut raw: HashMap<String, String> = HashMap::new();
    for d in &pool {
        for (_, t) in &d.fields {
            for w in words(t) {
                raw.entry(w.clone()).or_insert_with(|| stem(&w));
            }
        }
    }
    let n = pool.len().max(1) as f64;
    // Each query word stands for itself, or for close spellings when it is not in the library at all.
    let expand = |t: &str, typed: &str| -> Vec<(String, f64)> {
        if df.contains_key(t) || typed.chars().count() < 4 {
            return vec![(t.to_string(), 1.0)];
        }
        let limit = if typed.chars().count() >= 8 { 2 } else { 1 };
        let mut close: Vec<(String, f64)> = Vec::new();
        for (w, st) in &raw {
            if edit_distance(typed, w, limit) <= limit && !close.iter().any(|(c, _)| c == st) {
                close.push((st.clone(), 0.7));
            }
        }
        if close.is_empty() {
            close.push((t.to_string(), 1.0));
        }
        close
    };
    let terms: Vec<(Option<Field>, Alternatives)> = q
        .terms
        .iter()
        .map(|(f, t, typed)| (*f, expand(t, typed)))
        .collect();
    let mut hits = Vec::new();
    'docs: for (d, doc) in pool.iter().zip(&tokenised) {
        let mut score = 0.0;
        let mut best: Option<(f64, Field, String, String)> = None;
        // Every term and phrase must match somewhere (AND).
        for (field, alts) in &terms {
            let mut term_score = 0.0;
            for (f, ws, text) in doc {
                if field.is_some_and(|x| x != *f) {
                    continue;
                }
                for (alt, factor) in alts {
                    let tf = ws.iter().filter(|w| *w == alt).count();
                    if tf > 0 {
                        let idf = (1.0 + n / (*df.get(alt.as_str()).unwrap_or(&1) as f64)).ln();
                        let s = f.weight() * (1.0 + (tf as f64).ln()) * idf * factor;
                        term_score += s;
                        if best.as_ref().is_none_or(|b| s > b.0) {
                            best = Some((s, *f, (*text).clone(), alt.clone()));
                        }
                    }
                }
            }
            if term_score == 0.0 {
                continue 'docs;
            }
            score += term_score;
        }
        for (field, phrase) in &q.phrases {
            let mut found = false;
            for (f, ws, text) in doc {
                if field.is_some_and(|x| x != *f) {
                    continue;
                }
                if ws.windows(phrase.len()).any(|w| w == phrase.as_slice()) {
                    found = true;
                    let s = f.weight() * 3.0;
                    score += s;
                    if best.as_ref().is_none_or(|b| s > b.0) {
                        best = Some((s, *f, (*text).clone(), phrase[0].clone()));
                    }
                }
            }
            if !found {
                continue 'docs;
            }
        }
        let (_, field, text, word) =
            best.unwrap_or((0.0, Field::Title, d.title.clone(), String::new()));
        hits.push(Hit {
            key: d.key.clone(),
            title: d.title.clone(),
            collection: d.collection.clone(),
            score: (score * 100.0).round() / 100.0,
            snippet: snippet(&text, &word),
            field,
        });
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.cmp(&b.title))
    });
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Citation, Item};
    use crate::scale::new_blank;

    fn doc(key: &str, title: &str, items: &[&str], tags: &[&str], source: &str) -> Doc {
        let mut s = new_blank(Format::LikertBlankLine);
        s.title = title.into();
        s.items = items
            .iter()
            .enumerate()
            .map(|(k, t)| Item::new(k as u32 + 1, *t))
            .collect();
        s.tags = tags.iter().map(|t| t.to_string()).collect();
        s.source = Some(Citation {
            apa: source.into(),
            doi: None,
            reference_manager_id: None,
        });
        Doc::from_scale(key, &s)
    }

    fn docs() -> Vec<Doc> {
        vec![
            doc(
                "a",
                "Communication Anxiety Scale",
                &[
                    "I feel nervous when I speak in class.",
                    "Talking makes me tense.",
                ],
                &["Instructional"],
                "Wrench, J. S. (2001). Anxiety. Journal A, 1, 1.",
            ),
            doc(
                "b",
                "Library Comfort Scale",
                &["I feel at ease in a library.", "Libraries make me tense."],
                &["Leisure"],
                "Example, A. (2024). Libraries. Journal B, 2, 2.",
            ),
            doc(
                "c",
                "Garden Joy Index",
                &["I enjoy weeding.", "Gardening relaxes me."],
                &["Leisure", "Pilot study"],
                "Gardener, B. (1999). Soil. Journal C, 3, 3.",
            ),
        ]
    }

    #[test]
    fn titles_rank_above_items() {
        let h = search(&docs(), "anxiety", &Filters::default());
        assert_eq!(h[0].key, "a");
        assert_eq!(h.len(), 1);
        let h = search(&docs(), "tense", &Filters::default());
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn stems_and_slips_are_forgiven() {
        assert_eq!(
            search(&docs(), "libraries", &Filters::default())[0].key,
            "b"
        );
        assert_eq!(search(&docs(), "gardning", &Filters::default())[0].key, "c");
        assert_eq!(search(&docs(), "libary", &Filters::default())[0].key, "b");
        assert_eq!(search(&docs(), "weding", &Filters::default())[0].key, "c");
    }

    #[test]
    fn fields_phrases_and_filters() {
        assert_eq!(
            search(&docs(), "author:wrench", &Filters::default())[0].key,
            "a"
        );
        assert!(search(&docs(), "title:nervous", &Filters::default()).is_empty());
        assert_eq!(
            search(&docs(), "\"feel at ease\"", &Filters::default())[0].key,
            "b"
        );
        let only_leisure = Filters {
            tag: Some("leisure".into()),
            ..Default::default()
        };
        let h = search(&docs(), "", &only_leisure);
        assert_eq!(
            h.iter().map(|h| h.key.as_str()).collect::<Vec<_>>(),
            vec!["c", "b"]
        );
        assert_eq!(search(&docs(), "tense", &only_leisure).len(), 1);
    }

    #[test]
    fn every_word_must_match() {
        assert!(search(&docs(), "garden nervous", &Filters::default()).is_empty());
        let h = search(&docs(), "garden weeding", &Filters::default());
        assert_eq!(h.len(), 1);
        assert!(
            h[0].snippet.to_lowercase().contains("garden")
                || h[0].snippet.to_lowercase().contains("weed")
        );
    }
}
