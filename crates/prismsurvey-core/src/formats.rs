//! The response scale settings shared by the three formats (specification, "Settings for every scale"): point count,
//! anchor sets, labels on every point or the ends only, and the abbreviations a box grid prints.

use crate::model::{AnchorSet, Format, LabelMode, ResponseScale};

pub const MIN_POINTS: u8 = 2;
pub const MAX_POINTS: u8 = 11;

/// Agreement labels for a given point count, lowest first. Point counts with no common full set get end labels only.
fn agreement_labels(points: u8) -> Option<Vec<&'static str>> {
    Some(match points {
        2 => vec!["Disagree", "Agree"],
        3 => vec!["Disagree", "Neutral", "Agree"],
        4 => vec!["Strongly Disagree", "Disagree", "Agree", "Strongly Agree"],
        5 => vec![
            "Strongly Disagree",
            "Disagree",
            "Neutral",
            "Agree",
            "Strongly Agree",
        ],
        6 => vec![
            "Strongly Disagree",
            "Disagree",
            "Slightly Disagree",
            "Slightly Agree",
            "Agree",
            "Strongly Agree",
        ],
        7 => vec![
            "Strongly Disagree",
            "Disagree",
            "Slightly Disagree",
            "Neutral",
            "Slightly Agree",
            "Agree",
            "Strongly Agree",
        ],
        _ => return None,
    })
}

/// Frequency labels (the Sexual Communication wording at five points).
fn frequency_labels(points: u8) -> Option<Vec<&'static str>> {
    Some(match points {
        2 => vec!["Never", "Very Often"],
        3 => vec!["Never", "Occasionally", "Very Often"],
        4 => vec!["Never", "Rarely", "Often", "Very Often"],
        5 => vec!["Never", "Rarely", "Occasionally", "Often", "Very Often"],
        6 => vec![
            "Never",
            "Very Rarely",
            "Rarely",
            "Often",
            "Very Often",
            "Always",
        ],
        7 => vec![
            "Never",
            "Very Rarely",
            "Rarely",
            "Occasionally",
            "Often",
            "Very Often",
            "Always",
        ],
        _ => return None,
    })
}

fn end_labels(set: AnchorSet) -> (&'static str, &'static str) {
    match set {
        AnchorSet::Frequency => ("Never", "Very Often"),
        _ => ("Strongly Disagree", "Strongly Agree"),
    }
}

/// The short form of a label for a box grid header: initials, with "Slightly" written "Sl" so it stays distinct.
pub fn abbreviate(label: &str) -> String {
    let words: Vec<&str> = label.split_whitespace().collect();
    match words.as_slice() {
        [] => String::new(),
        [one] => one
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default(),
        many => many
            .iter()
            .map(|w| {
                if w.eq_ignore_ascii_case("slightly") {
                    "Sl".to_string()
                } else if w.eq_ignore_ascii_case("very") {
                    "V".to_string()
                } else {
                    w.chars()
                        .next()
                        .map(|c| c.to_uppercase().collect())
                        .unwrap_or_default()
                }
            })
            .collect(),
    }
}

/// Builds the response scale for an anchor set. A full label set is used when one exists for the point count and every
/// point is wanted; otherwise the ends only are labelled.
pub fn response_scale(points: u8, set: AnchorSet, mode: LabelMode) -> ResponseScale {
    let points = points.clamp(MIN_POINTS, MAX_POINTS);
    let full = match set {
        AnchorSet::Agreement => agreement_labels(points),
        AnchorSet::Frequency => frequency_labels(points),
        AnchorSet::Custom => None,
    };
    let (mode, labels): (LabelMode, Vec<Option<String>>) = match (mode, full) {
        (LabelMode::Every, Some(full)) => (
            LabelMode::Every,
            full.into_iter().map(|s| Some(s.to_string())).collect(),
        ),
        _ => {
            let (lo, hi) = end_labels(set);
            let mut v = vec![None; points as usize];
            v[0] = Some(lo.to_string());
            v[points as usize - 1] = Some(hi.to_string());
            (LabelMode::EndsOnly, v)
        }
    };
    let abbreviations = labels
        .iter()
        .map(|l| l.as_deref().map(abbreviate).unwrap_or_default())
        .collect();
    ResponseScale {
        points,
        anchor_set: set,
        label_mode: mode,
        labels,
        abbreviations,
        values: (1..=points as i32).collect(),
    }
}

/// Changes the point count of an existing response scale, keeping a custom set's end labels.
pub fn with_points(current: &ResponseScale, points: u8) -> ResponseScale {
    if current.anchor_set != AnchorSet::Custom {
        return response_scale(points, current.anchor_set, current.label_mode);
    }
    let points = points.clamp(MIN_POINTS, MAX_POINTS);
    let lo = current.labels.first().cloned().flatten();
    let hi = current.labels.last().cloned().flatten();
    let mut labels = vec![None; points as usize];
    labels[0] = lo;
    labels[points as usize - 1] = hi;
    let abbreviations = labels
        .iter()
        .map(|l| l.as_deref().map(abbreviate).unwrap_or_default())
        .collect();
    ResponseScale {
        points,
        anchor_set: AnchorSet::Custom,
        label_mode: LabelMode::EndsOnly,
        labels,
        abbreviations,
        values: (1..=points as i32).collect(),
    }
}

/// Recognises a full label list as one of the built-in anchor sets.
pub fn classify_labels(labels: &[String]) -> AnchorSet {
    let n = labels.len() as u8;
    let same = |set: Option<Vec<&str>>| {
        set.is_some_and(|s| {
            s.len() == labels.len()
                && s.iter()
                    .zip(labels)
                    .all(|(a, b)| a.eq_ignore_ascii_case(b.trim()))
        })
    };
    if same(agreement_labels(n)) {
        AnchorSet::Agreement
    } else if same(frequency_labels(n)) {
        AnchorSet::Frequency
    } else {
        AnchorSet::Custom
    }
}

/// Words that mark a line as an anchor row in a stored scale or an article.
pub const ANCHOR_WORDS: &[&str] = &[
    "strongly disagree",
    "disagree",
    "neutral",
    "agree",
    "strongly agree",
    "slightly disagree",
    "slightly agree",
    "undecided",
    "never",
    "rarely",
    "occasionally",
    "sometimes",
    "often",
    "very often",
    "always",
    "not at all",
    "very much",
    "extremely",
];

pub fn is_anchor_word(s: &str) -> bool {
    let t = s
        .trim()
        .trim_end_matches(['.', ',', ';'])
        .to_ascii_lowercase();
    !t.is_empty() && ANCHOR_WORDS.contains(&t.as_str())
}

/// The format a new scale of this kind starts with.
pub fn default_response(format: Format) -> ResponseScale {
    response_scale(
        format.default_points(),
        AnchorSet::Agreement,
        LabelMode::Every,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_point_agreement_has_his_abbreviations() {
        let r = response_scale(5, AnchorSet::Agreement, LabelMode::Every);
        assert_eq!(r.abbreviations, vec!["SD", "D", "N", "A", "SA"]);
        assert_eq!(r.values, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn odd_sizes_without_a_set_fall_back_to_end_labels() {
        let r = response_scale(9, AnchorSet::Agreement, LabelMode::Every);
        assert_eq!(r.label_mode, LabelMode::EndsOnly);
        assert_eq!(r.labels[0].as_deref(), Some("Strongly Disagree"));
        assert_eq!(r.labels[8].as_deref(), Some("Strongly Agree"));
        assert!(r.labels[4].is_none());
    }

    #[test]
    fn frequency_labels_match_sexual_communication() {
        let r = response_scale(5, AnchorSet::Frequency, LabelMode::Every);
        let l: Vec<_> = r.labels.iter().map(|l| l.clone().unwrap()).collect();
        assert_eq!(
            l,
            ["Never", "Rarely", "Occasionally", "Often", "Very Often"]
        );
        assert_eq!(classify_labels(&l), AnchorSet::Frequency);
    }

    #[test]
    fn point_count_is_clamped() {
        assert_eq!(
            response_scale(1, AnchorSet::Agreement, LabelMode::Every).points,
            2
        );
        assert_eq!(
            response_scale(20, AnchorSet::Agreement, LabelMode::Every).points,
            11
        );
    }
}
