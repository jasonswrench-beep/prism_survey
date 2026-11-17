//! Recommendations for a longer questionnaire (specification addendum, section 3): from the scales already in a study,
//! measures that would work well beside them, each with its reasons and no claim about the validity of the
//! combination. Three of the four agreed bases work from the rules alone; the optional model (phase 8) adds the fourth.
//!
//! - Construct fit: a candidate shares tags or categories with the study (a related area) but is not a near-copy of a
//!   scale already in it (the same construct measured twice).
//! - Respondent burden: the candidate's items and minutes against the length the researcher sets, and a preference for
//!   the format and point count most of the study already uses, so respondents switch less often.
//! - The researcher's own studies: measures stacked together before with a scale in this study.

use crate::model::Format;
use crate::text::wording_match;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// What the recommender needs to know about one measure (a library scale or a Comm Directory entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub key: String,
    pub title: String,
    pub collection: String,
    pub tags: Vec<String>,
    pub items: usize,
    pub format: Option<Format>,
    pub points: Option<u8>,
    /// Item wording, when known, to tell a near-copy apart.
    #[serde(default)]
    pub item_text: Vec<String>,
    /// Other keys for the same measure (a library scale built from a directory entry is known by both).
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl Measure {
    fn keys(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.key.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// The most minutes the whole questionnaire should take.
    pub max_minutes: u32,
    pub limit: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_minutes: 20,
            limit: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub key: String,
    pub title: String,
    pub collection: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// About seven seconds an item.
pub fn minutes(items: usize) -> f64 {
    items as f64 * 7.0 / 60.0
}

fn key(s: &str) -> String {
    crate::tags::key(s)
}

/// Words that say what kind of instrument a title names, not what it measures.
const GENERIC: &[&str] = &[
    "scale",
    "scales",
    "measure",
    "measures",
    "inventory",
    "index",
    "questionnaire",
    "instrument",
    "test",
    "short",
    "form",
    "revised",
    "version",
    "brief",
    "the",
    "of",
    "a",
    "an",
    "for",
    "and",
    "in",
    "to",
];

fn title_core(t: &str) -> HashSet<String> {
    crate::text::words(t)
        .into_iter()
        .filter(|w| !GENERIC.contains(&w.as_str()))
        .collect()
}

/// Two titles name the same construct: their working words match closely, or one title's working words all appear in
/// the other's ("Communication Apprehension Scale" and its short form).
pub fn same_construct(a: &str, b: &str) -> bool {
    let (ca, cb) = (title_core(a), title_core(b));
    if ca.is_empty() || cb.is_empty() {
        return wording_match(a, b) >= 0.7;
    }
    let inter = ca.intersection(&cb).count();
    let small = ca.len().min(cb.len());
    (small >= 2 && inter == small) || inter as f64 / ca.union(&cb).count() as f64 >= 0.7
}

/// Ranks candidates for a study. `history` lists the measures of the researcher's earlier studies, one list per study.
pub fn recommend(
    study: &[Measure],
    candidates: &[Measure],
    history: &[Vec<String>],
    settings: &Settings,
) -> Vec<Recommendation> {
    let study_tags: HashSet<String> = study
        .iter()
        .flat_map(|m| m.tags.iter().map(|t| key(t)))
        .collect();
    let study_keys: HashSet<&str> = study.iter().flat_map(|m| m.keys()).collect();
    let used: f64 = study.iter().map(|m| minutes(m.items)).sum();
    let left = settings.max_minutes as f64 - used;
    // The format and point count most of the study uses.
    let mut formats: Vec<(Format, u8, usize)> = Vec::new();
    for m in study {
        if let (Some(f), Some(p)) = (m.format, m.points) {
            match formats.iter_mut().find(|x| x.0 == f && x.1 == p) {
                Some(x) => x.2 += 1,
                None => formats.push((f, p, 1)),
            }
        }
    }
    let usual = formats.iter().max_by_key(|x| x.2).map(|x| (x.0, x.1));
    let mut out = Vec::new();
    for c in candidates {
        if c.keys().any(|k| study_keys.contains(k)) {
            continue;
        }
        let mut score = 0.0;
        let mut reasons = Vec::new();
        // A near-copy of a scale already in the study measures the same thing twice.
        let copy_of = study.iter().find(|m| {
            same_construct(&m.title, &c.title) || {
                let shared = c
                    .item_text
                    .iter()
                    .filter(|t| m.item_text.iter().any(|u| wording_match(t, u) >= 0.8))
                    .count();
                !c.item_text.is_empty() && shared * 2 >= c.item_text.len()
            }
        });
        if copy_of.is_some() {
            continue;
        }
        let shared: Vec<&String> = c
            .tags
            .iter()
            .filter(|t| study_tags.contains(&key(t)))
            .collect();
        if !shared.is_empty() {
            score += 2.0 + shared.len() as f64;
            reasons.push(format!(
                "in the same area as the study ({})",
                shared
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        // Earlier studies that held this measure beside one already chosen.
        let together = history
            .iter()
            .filter(|h| {
                c.keys().any(|k| h.iter().any(|x| x == k))
                    && h.iter().any(|k| study_keys.contains(k.as_str()))
            })
            .count();
        if together > 0 {
            score += 2.5 * together as f64;
            reasons.push(format!(
                "you used it beside a scale in this study in {together} earlier stud{}",
                if together == 1 { "y" } else { "ies" }
            ));
        }
        if score == 0.0 {
            continue;
        }
        let mins = minutes(c.items);
        if c.items > 0 {
            if mins <= left {
                score += 1.0;
                reasons.push(format!(
                    "{} items, about {:.0} minute{}, within the {} minutes set",
                    c.items,
                    mins.ceil(),
                    if mins.ceil() == 1.0 { "" } else { "s" },
                    settings.max_minutes
                ));
            } else {
                score -= 2.0;
                reasons.push(format!(
                    "{} items would take the questionnaire past {} minutes",
                    c.items, settings.max_minutes
                ));
            }
        }
        if let (Some(u), Some(f), Some(p)) = (usual, c.format, c.points) {
            if (f, p) == u {
                score += 1.0;
                reasons.push("the same format and point count as most of the study".into());
            } else {
                reasons.push("a different format or point count from most of the study, so respondents switch".into());
            }
        }
        out.push(Recommendation {
            key: c.key.clone(),
            title: c.title.clone(),
            collection: c.collection.clone(),
            score,
            reasons,
        });
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.cmp(&b.title))
    });
    out.truncate(settings.limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(key: &str, title: &str, tags: &[&str], items: usize) -> Measure {
        Measure {
            key: key.into(),
            title: title.into(),
            collection: "library".into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            items,
            format: Some(Format::LikertBlankLine),
            points: Some(5),
            item_text: vec![],
            aliases: vec![],
        }
    }

    #[test]
    fn related_measures_rank_with_reasons_and_copies_are_left_out() {
        let study = vec![m(
            "a",
            "Communication Apprehension Scale",
            &["Apprehension", "Instructional"],
            24,
        )];
        let cands = vec![
            m("b", "Willingness to Communicate", &["Apprehension"], 20),
            m("c", "Garden Joy Index", &["Leisure"], 6),
            m(
                "d",
                "Communication Apprehension Scale Short Form",
                &["Apprehension"],
                10,
            ),
            m("e", "Teacher Clarity", &["Instructional"], 200),
        ];
        let r = recommend(&study, &cands, &[], &Settings::default());
        let keys: Vec<&str> = r.iter().map(|x| x.key.as_str()).collect();
        assert_eq!(keys[0], "b");
        assert!(
            !keys.contains(&"c"),
            "no shared area, no reason to suggest it"
        );
        assert!(!keys.contains(&"d"), "a near-copy of a chosen scale");
        let e = r.iter().find(|x| x.key == "e").unwrap();
        assert!(e.reasons.iter().any(|x| x.contains("past 20 minutes")));
        assert!(r[0].reasons.iter().any(|x| x.contains("same area")));
    }

    #[test]
    fn earlier_studies_count() {
        let study = vec![m("a", "Scale A", &[], 10)];
        let cands = vec![m("b", "Scale B", &[], 10), m("c", "Scale C", &[], 10)];
        let history = vec![vec!["a".to_string(), "c".to_string()]];
        let r = recommend(&study, &cands, &history, &Settings::default());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].key, "c");
        assert!(r[0].reasons[0].contains("1 earlier study"));
    }

    #[test]
    fn same_construct_titles() {
        assert!(same_construct(
            "Communication Apprehension Scale",
            "Communication Apprehension Scale Short Form"
        ));
        assert!(same_construct(
            "Personal Report of Communication Apprehension",
            "Personal Report of Communication Apprehension-24"
        ));
        assert!(!same_construct(
            "Communication Apprehension Scale",
            "Writing Apprehension Test"
        ));
        assert!(!same_construct(
            "Teacher Clarity Scale",
            "Teacher Immediacy Scale"
        ));
    }
}
