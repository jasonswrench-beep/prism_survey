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

/// Five-point label sets offered on the form as a starting point (Jason's figure of Likert-type anchors, decision 75),
/// lowest first, as printed. Choosing one fills the labels; the scale keeps them as its own (custom) labels.
pub const COMMON_ANCHORS: &[[&str; 5]] = &[
    [
        "Strongly disagree",
        "Disagree",
        "Neutral",
        "Agree",
        "Strongly Agree",
    ],
    ["Never", "Rarely", "Sometimes", "Often", "Always"],
    [
        "Definitely not possible",
        "Very unlikely",
        "Don’t know",
        "Somewhat likely",
        "Extremely likely",
    ],
    [
        "Not at all",
        "To a little extent",
        "To some extent",
        "To a great extent",
        "To a very great extent",
    ],
    [
        "Not concerned at all",
        "Not really concerned",
        "Somewhat concerned",
        "Concerned",
        "Very concerned",
    ],
    [
        "Definitely false",
        "Possibly false",
        "Don’t know",
        "Possibly true",
        "Definitely true",
    ],
    ["None", "Very mild", "Mild", "Moderate", "Severe"],
    [
        "Lacking or ineffective",
        "Below average",
        "Average",
        "Above average",
        "Excellent",
    ],
    [
        "Very unrealistic",
        "Unrealistic",
        "Not sure",
        "Realistic",
        "Very Realistic",
    ],
    ["NO", "no", "?", "yes", "YES"],
    ["Poor", "Fair", "Good", "Very good", "Excellent"],
    [
        "Not important",
        "Somewhat important",
        "Important",
        "Very important",
        "Extremely important",
    ],
    [
        "Substantially worse",
        "Somewhat worse",
        "About the same",
        "Somewhat better",
        "Substantially better",
    ],
    [
        "Very dissatisfied",
        "Dissatisfied",
        "Neither dissatisfied nor satisfied",
        "Satisfied",
        "Very satisfied",
    ],
    ["1–20%", "21–40%", "41–60%", "61–80%", "81–100%"],
    [
        "Definitely did not use",
        "Rarely used",
        "Used moderately",
        "Above average use",
        "Great use",
    ],
    [
        "Absolutely do not agree",
        "Do not agree",
        "Neither agree nor disagree",
        "Agree",
        "Absolutely Agree",
    ],
    [
        "Not at all like me",
        "Not really like me",
        "Neither like me nor dislike me",
        "Somewhat like me",
        "Definitely like me",
    ],
    ["F", "D", "C", "B", "A"],
    ["Not at all", "Somewhat", "Moderately", "Quite", "Extremely"],
    [
        "No Chance",
        "Very little chance",
        "Not sure",
        "Good chance",
        "Certain to happen",
    ],
    [
        "Far below expected level",
        "Below expected level",
        "At expected level",
        "Above expected level",
        "Far above expected level",
    ],
    [
        "Extremely negative",
        "Moderately negative",
        "Neither positive nor negative",
        "Moderately positive",
        "Extremely positive",
    ],
    [
        "Have not",
        "Have slightly",
        "Have moderately",
        "Have mostly",
        "Have fully",
    ],
    ["Very bad", "Rather bad", "Neutral", "Good", "Very good"],
    [
        "Never",
        "Occasionally (once or twice altogether)",
        "Regularly (once or twice a year)",
        "Often (several times a year)",
        "Very often (about weekly)",
    ],
    [
        "Not at all",
        "Slightly",
        "Moderately",
        "Markedly",
        "Extremely",
    ],
    [
        "Very much a loner",
        "Moderately a loner",
        "Average",
        "Somewhat socially sought after",
        "Very socially sought after",
    ],
    [
        "None",
        "Slightly",
        "No opinion",
        "Moderately",
        "Extensively",
    ],
    [
        "Definitely would not do this",
        "Might not do this",
        "Not sure whether I would do this",
        "Might do this",
        "Would definitely do this",
    ],
    [
        "I reject this",
        "I am inclined to reject this",
        "I neither reject nor accept this",
        "I accept this as important",
        "I accept this as of the greatest importance",
    ],
    [
        "Extremely Conservative",
        "Conservative",
        "Moderate: Middle of the road",
        "Progressive",
        "Extremely progressive",
    ],
    ["– –", "–", "0", "+", "++"],
    [
        "Infrequently",
        "Fairly infrequently",
        "No opinion",
        "Fairly frequently",
        "Frequently",
    ],
];

/// A response scale with labels the person chose (a common set from the form, or their own), kept as a custom set.
/// When the labels fit the point count every point is labelled; otherwise the first and last label become the ends.
pub fn labelled_scale(points: u8, labels: &[String]) -> ResponseScale {
    let points = points.clamp(MIN_POINTS, MAX_POINTS);
    let given: Vec<Option<String>> = labels
        .iter()
        .map(|l| Some(l.trim().to_string()).filter(|l| !l.is_empty()))
        .collect();
    let (label_mode, labels) = if given.len() == points as usize {
        (LabelMode::Every, given)
    } else {
        let mut v = vec![None; points as usize];
        v[0] = given.first().cloned().flatten();
        v[points as usize - 1] = given.last().cloned().flatten();
        (LabelMode::EndsOnly, v)
    };
    let abbreviations = labels
        .iter()
        .map(|l| l.as_deref().map(abbreviate).unwrap_or_default())
        .collect();
    ResponseScale {
        points,
        anchor_set: AnchorSet::Custom,
        label_mode,
        labels,
        abbreviations,
        values: (1..=points as i32).collect(),
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
    fn the_common_sets_have_five_labels_each_and_no_empty_ones() {
        assert_eq!(COMMON_ANCHORS.len(), 34);
        for set in COMMON_ANCHORS {
            assert!(set.iter().all(|l| !l.trim().is_empty()), "{set:?}");
        }
    }

    #[test]
    fn a_chosen_set_fills_every_point_at_five_and_the_ends_otherwise() {
        let set: Vec<String> = COMMON_ANCHORS[11].iter().map(|s| s.to_string()).collect();
        let r = labelled_scale(5, &set);
        assert_eq!(r.anchor_set, AnchorSet::Custom);
        assert_eq!(r.label_mode, LabelMode::Every);
        assert_eq!(r.labels[2].as_deref(), Some("Important"));
        assert_eq!(r.abbreviations[0], "NI");
        let r = labelled_scale(7, &set);
        assert_eq!(r.label_mode, LabelMode::EndsOnly);
        assert_eq!(r.labels[0].as_deref(), Some("Not important"));
        assert_eq!(r.labels[6].as_deref(), Some("Extremely important"));
        assert!(r.labels[3].is_none());
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
