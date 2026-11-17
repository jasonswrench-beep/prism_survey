//! House instructions (specification, "Instructions rewritten to match"). House instructions are built from a template
//! with slots, so they always fit the format and point count. Instructions kept verbatim from a source, or edited by hand,
//! are never rewritten; a change of format or point count raises a fit finding instead.

use crate::model::{Format, Instructions, InstructionsOrigin, ResponseScale};
use regex::Regex;
use std::sync::OnceLock;

/// The opening sentence a new scale of each format starts with, from the stored templates.
pub fn default_stem(format: Format) -> &'static str {
    match format {
        Format::LikertBlankLine => {
            "Read the following questions and select the answer that corresponds with your perception. Do not be concerned if some of the items appear similar."
        }
        Format::LikertBoxGrid => "Indicate the degree to which each of these statements applies to you.",
        Format::SemanticDifferential => "Below are several oppositely worded adjective pairs.",
    }
}

/// Intensity words for the pairs of a semantic differential, from the ends inward, by how many pairs there are.
fn intensity_words(pairs: usize) -> &'static [&'static str] {
    match pairs {
        0 => &[],
        1 => &["a strong feeling"],
        2 => &["a strong feeling", "a fairly weak feeling"],
        3 => &[
            "a very strong feeling",
            "a strong feeling",
            "a fairly weak feeling",
        ],
        4 => &[
            "a very strong feeling",
            "a strong feeling",
            "a moderate feeling",
            "a fairly weak feeling",
        ],
        _ => &[
            "an extremely strong feeling",
            "a very strong feeling",
            "a strong feeling",
            "a moderate feeling",
            "a fairly weak feeling",
        ],
    }
}

/// The intensity sentences of a semantic differential: "Numbers "1" and "7" indicate a very strong feeling." and so on,
/// pairing the points from the ends inward, then the undecided sentence when there is a middle point.
pub fn intensity_sentences(response: &ResponseScale) -> Vec<String> {
    let v = &response.values;
    let n = v.len();
    let pairs = n / 2;
    let words = intensity_words(pairs);
    let mut out = Vec::new();
    for i in 0..pairs {
        out.push(format!(
            "Numbers \"{}\" and \"{}\" indicate {}.",
            v[i],
            v[n - 1 - i],
            words[i.min(words.len() - 1)]
        ));
    }
    if n % 2 == 1 {
        out.push(format!("Number \"{}\" indicates you are undecided or do not understand the adjectives themselves.", v[n / 2]));
    }
    out
}

/// The box grid key: "SD = Strongly Disagree, D = Disagree, N = Neutral, A = Agree, and SA = Strongly Agree."
pub fn box_grid_key(response: &ResponseScale) -> String {
    let parts: Vec<String> = response
        .labels
        .iter()
        .zip(&response.abbreviations)
        .filter_map(|(l, a)| l.as_ref().map(|l| format!("{a} = {l}")))
        .collect();
    match parts.len() {
        0 => String::new(),
        1 => format!("{}.", parts[0]),
        _ => format!(
            "{}, and {}.",
            parts[..parts.len() - 1].join(", "),
            parts[parts.len() - 1]
        ),
    }
}

/// The format sentences that follow the stem.
fn format_sentences(format: Format, response: &ResponseScale) -> Vec<String> {
    match format {
        Format::LikertBlankLine => vec![
            "Please use the scale below to rate the degree to which each statement applies to you."
                .into(),
        ],
        Format::LikertBoxGrid => vec![
            "For each statement, place an \"x\" in the box that corresponds to your response."
                .into(),
            "Place only one x on each line.".into(),
        ],
        Format::SemanticDifferential => {
            let mut s = vec![
                "Circle the number between the adjectives which best represents your beliefs."
                    .to_string(),
            ];
            s.extend(intensity_sentences(response));
            s.push("There are no right or wrong answers.".into());
            s
        }
    }
}

/// Builds house instructions for a format and response scale.
pub fn house_text(format: Format, response: &ResponseScale, stem: Option<&str>) -> String {
    let stem = stem
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default_stem(format));
    let mut parts = vec![stem.to_string()];
    parts.extend(format_sentences(format, response));
    parts.join(" ")
}

pub fn house(format: Format, response: &ResponseScale, stem: Option<&str>) -> Instructions {
    Instructions {
        text: house_text(format, response, stem),
        origin: InstructionsOrigin::House,
        house_stem: stem.map(str::to_string),
    }
}

/// True when the scale has no middle point (an even point count), so it is forced choice.
pub fn is_forced_choice(response: &ResponseScale) -> bool {
    response.points.is_multiple_of(2)
}

/// One reason the instructions no longer fit the format or point count.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitFinding {
    /// The words in the instructions that do not fit.
    pub words: String,
    pub message: String,
}

fn quoted_numbers() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r#"(?i)\bnumbers?\s+["“]?(\d+)["”]?(?:\s+and\s+["“]?(\d+)["”]?)?"#).unwrap()
    })
}

fn point_scale() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)\b(\d+)[- ]point\b").unwrap())
}

fn word(w: &str) -> Regex {
    Regex::new(&format!(r"(?i)\b{w}\b")).unwrap()
}

/// Checks instruction words and numbers against the format and point count. Used for verbatim or hand-edited
/// instructions after a change, and by the self-check.
pub fn fit_findings(text: &str, format: Format, response: &ResponseScale) -> Vec<FitFinding> {
    let mut out = Vec::new();
    let lo = response.lowest();
    let hi = response.highest();
    for c in quoted_numbers().captures_iter(text) {
        for m in [c.get(1), c.get(2)].into_iter().flatten() {
            if let Ok(n) = m.as_str().parse::<i32>() {
                if n < lo || n > hi {
                    out.push(FitFinding {
                        words: c.get(0).unwrap().as_str().to_string(),
                        message: format!(
                            "The instructions mention {n}, but the scale runs from {lo} to {hi}."
                        ),
                    });
                    break;
                }
            }
        }
    }
    for c in point_scale().captures_iter(text) {
        if let Ok(n) = c[1].parse::<u8>() {
            if n != response.points {
                out.push(FitFinding {
                    words: c[0].to_string(),
                    message: format!(
                        "The instructions describe a {n}-point scale, but the scale has {} points.",
                        response.points
                    ),
                });
            }
        }
    }
    let blank = word("blanks?");
    let boxes = word(r#"(?:box(?:es)?|an\s+["“]?x["”]?)"#);
    let circle = word("circle");
    let adjectives = word("adjectives?");
    let found = |r: &Regex| r.find(text).map(|m| m.as_str().to_string());
    match format {
        Format::LikertBlankLine => {
            if let Some(w) = found(&boxes) {
                out.push(FitFinding {
                    words: w,
                    message: "The instructions mention a box, but this is a blank-line scale."
                        .into(),
                });
            }
            if let Some(w) = found(&circle) {
                out.push(FitFinding {
                    words: w,
                    message: "The instructions ask to circle, but this is a blank-line scale."
                        .into(),
                });
            }
        }
        Format::LikertBoxGrid => {
            if let Some(w) = found(&blank) {
                out.push(FitFinding {
                    words: w,
                    message: "The instructions mention blanks, but this is a box grid.".into(),
                });
            }
            if let Some(w) = found(&circle) {
                out.push(FitFinding {
                    words: w,
                    message: "The instructions ask to circle, but this is a box grid.".into(),
                });
            }
        }
        Format::SemanticDifferential => {
            if let Some(w) = found(&blank) {
                out.push(FitFinding {
                    words: w,
                    message:
                        "The instructions mention blanks, but this is a semantic differential."
                            .into(),
                });
            }
            if let Some(w) = found(&boxes) {
                out.push(FitFinding {
                    words: w,
                    message: "The instructions mention a box, but this is a semantic differential."
                        .into(),
                });
            }
        }
    }
    if format != Format::SemanticDifferential {
        if let Some(w) = found(&adjectives) {
            out.push(FitFinding {
                words: w,
                message: "The instructions mention adjectives, but this is a Likert scale.".into(),
            });
        }
    }
    if response.points.is_multiple_of(2) {
        if let Some(w) = found(&word("(?:undecided|neutral)")) {
            out.push(FitFinding {
                words: w,
                message: "The instructions mention a middle answer, but an even point count has no middle.".into(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::response_scale;
    use crate::model::{AnchorSet, LabelMode};

    fn r(points: u8) -> ResponseScale {
        response_scale(points, AnchorSet::Agreement, LabelMode::Every)
    }

    #[test]
    fn seven_point_semantic_differential_uses_his_wording() {
        let s = intensity_sentences(&r(7));
        assert_eq!(
            s,
            [
                "Numbers \"1\" and \"7\" indicate a very strong feeling.",
                "Numbers \"2\" and \"6\" indicate a strong feeling.",
                "Numbers \"3\" and \"5\" indicate a fairly weak feeling.",
                "Number \"4\" indicates you are undecided or do not understand the adjectives themselves.",
            ]
        );
    }

    #[test]
    fn five_point_pairs_strong_then_fairly_weak() {
        let s = intensity_sentences(&r(5));
        assert_eq!(s[0], "Numbers \"1\" and \"5\" indicate a strong feeling.");
        assert_eq!(
            s[1],
            "Numbers \"2\" and \"4\" indicate a fairly weak feeling."
        );
        assert_eq!(s[2], "Number \"3\" indicates you are undecided or do not understand the adjectives themselves.");
    }

    #[test]
    fn nine_points_add_moderate() {
        let s = intensity_sentences(&r(9));
        assert_eq!(s.len(), 5);
        assert!(s[2].contains("\"3\" and \"7\" indicate a moderate feeling"));
        assert!(s[3].contains("\"4\" and \"6\" indicate a fairly weak feeling"));
    }

    #[test]
    fn even_points_drop_the_undecided_sentence() {
        let s = intensity_sentences(&r(6));
        assert_eq!(s.len(), 3);
        assert!(s.iter().all(|x| !x.contains("undecided")));
        assert!(is_forced_choice(&r(6)));
    }

    #[test]
    fn box_grid_key_is_rebuilt_from_anchors() {
        assert_eq!(box_grid_key(&r(5)), "SD = Strongly Disagree, D = Disagree, N = Neutral, A = Agree, and SA = Strongly Agree.");
    }

    #[test]
    fn house_text_fits_its_own_format() {
        for f in Format::ALL {
            for p in 2..=11 {
                let resp = r(p);
                let t = house_text(f, &resp, None);
                assert!(fit_findings(&t, f, &resp).is_empty(), "{f:?} {p}: {t}");
            }
        }
    }

    #[test]
    fn seven_point_wording_on_five_points_is_flagged() {
        let t = house_text(Format::SemanticDifferential, &r(7), None);
        let f = fit_findings(&t, Format::SemanticDifferential, &r(5));
        assert!(f.iter().any(|f| f.words.contains('7')), "{f:?}");
    }

    #[test]
    fn blanks_on_a_box_grid_are_flagged() {
        let t = "Indicate the degree by filling in your response in the blanks: place an \"x\" in the box.";
        let f = fit_findings(t, Format::LikertBoxGrid, &r(5));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].words, "blanks");
    }
}
