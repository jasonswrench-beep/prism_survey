//! Making and changing scales: a new blank scale from a format, switching format or point count with the instructions
//! following along, keeping a title's item-count suffix in step, and the computed score range.

use crate::formats::{default_response, with_points};
use crate::instructions::{self, FitFinding};
use crate::model::*;
use regex::Regex;
use std::sync::OnceLock;

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// A new scale from one of the three blank formats, with the template's number of empty lines.
pub fn new_blank(format: Format) -> Scale {
    let response = default_response(format);
    let instructions = instructions::house(format, &response, None);
    let items = (1..=format.template_lines() as u32)
        .map(|n| {
            let mut it = Item::new(n, "");
            if format == Format::SemanticDifferential {
                it.left = Some(String::new());
                it.right = Some(String::new());
            }
            it
        })
        .collect();
    Scale {
        pscale_version: SCHEMA_VERSION,
        id: new_id(),
        title: String::new(),
        short_name: None,
        tags: Vec::new(),
        count_suffix: false,
        format,
        response,
        instructions,
        items,
        recode: RecodeRule::default(),
        subscales: Vec::new(),
        scoring: Scoring::default(),
        score_range: ScoreRange::default(),
        source: None,
        adapted_from: None,
        source_placement: SourcePlacement::UnderScale,
        // A scale typed by the user stays on this machine (specification, "Licensing and the shipped library").
        licence: Licence {
            status: LicenceStatus::LocalOnly,
            note: None,
        },
        development: None,
        provenance: Vec::new(),
        history: Vec::new(),
        unparsed: Vec::new(),
        check_decisions: Vec::new(),
        directory_entry: None,
        source_notes: Vec::new(),
        variant_of: None,
    }
}

/// What a change of format or point count did to the instructions.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeOutcome {
    /// True when house instructions were rebuilt.
    pub instructions_rewritten: bool,
    /// For verbatim or edited instructions: the words that no longer fit.
    pub findings: Vec<FitFinding>,
}

fn after_response_change(scale: &mut Scale) -> ChangeOutcome {
    if scale.instructions.origin == InstructionsOrigin::House {
        scale.instructions = instructions::house(
            scale.format,
            &scale.response,
            scale.instructions.house_stem.as_deref(),
        );
        ChangeOutcome {
            instructions_rewritten: true,
            findings: Vec::new(),
        }
    } else {
        ChangeOutcome {
            instructions_rewritten: false,
            findings: instructions::fit_findings(
                &scale.instructions.text,
                scale.format,
                &scale.response,
            ),
        }
    }
}

/// Switches format. A semantic differential gets left and right adjective slots; leaving one keeps the pair text as the
/// item text so nothing typed is lost.
pub fn set_format(scale: &mut Scale, format: Format) -> ChangeOutcome {
    if scale.format == format {
        return ChangeOutcome {
            instructions_rewritten: false,
            findings: Vec::new(),
        };
    }
    let was_sd = scale.format == Format::SemanticDifferential;
    scale.format = format;
    for it in &mut scale.items {
        if format == Format::SemanticDifferential {
            it.left.get_or_insert_with(String::new);
            it.right.get_or_insert_with(String::new);
        } else if was_sd {
            if it.text.trim().is_empty() {
                let l = it.left.take().unwrap_or_default();
                let r = it.right.take().unwrap_or_default();
                if !l.is_empty() || !r.is_empty() {
                    it.text = format!("{l} / {r}");
                }
            } else {
                it.left = None;
                it.right = None;
            }
        }
    }
    after_response_change(scale)
}

pub fn set_points(scale: &mut Scale, points: u8) -> ChangeOutcome {
    scale.response = with_points(&scale.response, points);
    if let RecodeKind::Table { .. } = scale.recode.rule {
        // A stated table belongs to the old point count; the default reflection fits any count.
        scale.recode.rule = RecodeKind::Reflect;
    }
    after_response_change(scale)
}

pub fn set_response(scale: &mut Scale, response: ResponseScale) -> ChangeOutcome {
    scale.response = response;
    after_response_change(scale)
}

/// Records a hand edit of the instructions: from now on they are never rewritten.
pub fn edit_instructions(scale: &mut Scale, text: String) {
    if text != scale.instructions.text {
        scale.instructions.text = text;
        scale.instructions.origin = InstructionsOrigin::Edited;
    }
}

/// Goes back to house instructions for the current format.
pub fn use_house_instructions(scale: &mut Scale, stem: Option<String>) {
    scale.instructions = instructions::house(scale.format, &scale.response, stem.as_deref());
}

fn suffix_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"-(\d+)\b").unwrap())
}

/// The item-count suffix in a title, such as 16 in "Homophobia-16 Scale".
pub fn title_count(title: &str) -> Option<u32> {
    suffix_re().captures(title).and_then(|c| c[1].parse().ok())
}

/// Items that have text (a blank template's empty lines are not counted).
pub fn filled_items(scale: &Scale) -> usize {
    scale
        .items
        .iter()
        .filter(|i| {
            !i.text.trim().is_empty() || i.left.as_deref().is_some_and(|s| !s.trim().is_empty())
        })
        .count()
}

/// Keeps the item-count suffix in step with the item count, when the scale asks for that.
pub fn sync_title_count(scale: &mut Scale) {
    if !scale.count_suffix {
        return;
    }
    let n = scale.items.len();
    if let Some(m) = suffix_re().find(&scale.title) {
        let new = format!("-{n}");
        if m.as_str() != new {
            scale.title.replace_range(m.range(), &new);
        }
    }
}

/// Renumbers items 1 to n in their current order.
pub fn renumber(scale: &mut Scale) {
    for (i, it) in scale.items.iter_mut().enumerate() {
        it.number = i as u32 + 1;
    }
}

/// Drops empty template lines left at the end of a scale.
pub fn trim_empty_tail(scale: &mut Scale) {
    while scale.items.last().is_some_and(|i| {
        i.text.trim().is_empty()
            && i.left.as_deref().unwrap_or("").trim().is_empty()
            && i.right.as_deref().unwrap_or("").trim().is_empty()
    }) {
        scale.items.pop();
    }
}

/// The range the total can take, computed from the items, points and scoring rule. `None` when there is no total.
pub fn computed_range(scale: &Scale) -> Option<(f64, f64)> {
    if !scale.scoring.has_total {
        return None;
    }
    let lo = scale.response.lowest() as f64;
    let hi = scale.response.highest() as f64;
    let count = scale.items.len() as u32;
    match scale.scoring.total_from {
        TotalFrom::Items => match scale.scoring.total_method {
            ScoreMethod::Sum => Some((lo * count as f64, hi * count as f64)),
            ScoreMethod::Mean => Some((lo, hi)),
        },
        TotalFrom::Subscales => {
            if scale.subscales.is_empty() {
                return None;
            }
            let parts: Vec<(f64, f64)> = scale
                .subscales
                .iter()
                .map(|s| {
                    let k = s.items.resolve(count).len() as f64;
                    match scale.scoring.subscale_method {
                        ScoreMethod::Sum => (lo * k, hi * k),
                        ScoreMethod::Mean => (lo, hi),
                    }
                })
                .collect();
            let (a, b) = parts
                .iter()
                .fold((0.0, 0.0), |acc, p| (acc.0 + p.0, acc.1 + p.1));
            match scale.scoring.total_method {
                ScoreMethod::Sum => Some((a, b)),
                ScoreMethod::Mean => Some((a / parts.len() as f64, b / parts.len() as f64)),
            }
        }
    }
}

/// The scored value of an answer on a reversed item.
pub fn recode_value(scale: &Scale, answer: i32) -> i32 {
    match &scale.recode.rule {
        RecodeKind::Reflect => scale.response.lowest() + scale.response.highest() - answer,
        RecodeKind::Table { pairs } => pairs
            .iter()
            .find(|p| p.0 == answer)
            .map(|p| p.1)
            .unwrap_or(answer),
    }
}

/// The recode table to print: each answer and what it scores once reversed.
pub fn recode_table(scale: &Scale) -> Vec<(i32, i32)> {
    scale
        .response
        .values
        .iter()
        .map(|&v| (v, recode_value(scale, v)))
        .collect()
}

/// The numbers of the items marked reversed (confirmed), in order.
pub fn reversed_numbers(scale: &Scale) -> Vec<u32> {
    scale
        .items
        .iter()
        .filter(|i| i.reverse.is_reversed())
        .map(|i| i.number)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_templates_have_his_line_counts() {
        assert_eq!(new_blank(Format::LikertBlankLine).items.len(), 40);
        assert_eq!(new_blank(Format::SemanticDifferential).items.len(), 30);
        assert_eq!(new_blank(Format::SemanticDifferential).response.points, 7);
    }

    #[test]
    fn house_instructions_follow_a_format_switch() {
        let mut s = new_blank(Format::LikertBlankLine);
        let out = set_format(&mut s, Format::SemanticDifferential);
        assert!(out.instructions_rewritten);
        assert!(s.instructions.text.contains("Circle the number"));
        // The point count stays at 5, so the intensity sentences are the 5-point ones.
        assert!(s
            .instructions
            .text
            .contains("Numbers \"1\" and \"5\" indicate a strong feeling."));
    }

    #[test]
    fn edited_instructions_are_flagged_not_rewritten() {
        let mut s = new_blank(Format::SemanticDifferential);
        let original = s.instructions.text.clone();
        edit_instructions(&mut s, format!("{original} Thank you."));
        let out = set_points(&mut s, 5);
        assert!(!out.instructions_rewritten);
        assert!(s.instructions.text.starts_with(&original));
        assert!(
            out.findings.iter().any(|f| f.words.contains('7')),
            "{:?}",
            out.findings
        );
    }

    #[test]
    fn title_suffix_follows_item_count() {
        let mut s = new_blank(Format::LikertBlankLine);
        s.title = "Homophobia-15 Scale".into();
        s.count_suffix = true;
        s.items.truncate(16);
        sync_title_count(&mut s);
        assert_eq!(s.title, "Homophobia-16 Scale");
        assert_eq!(title_count(&s.title), Some(16));
    }

    #[test]
    fn computed_range_for_ten_five_point_items_is_10_to_50() {
        let mut s = new_blank(Format::LikertBlankLine);
        s.items.truncate(10);
        assert_eq!(computed_range(&s), Some((10.0, 50.0)));
    }

    #[test]
    fn total_from_four_blocks_of_five_is_20_to_100() {
        let mut s = new_blank(Format::LikertBlankLine);
        s.items.truncate(20);
        s.scoring.total_from = TotalFrom::Subscales;
        for b in 0..4 {
            s.subscales.push(Subscale {
                name: format!("Block {b}"),
                items: ItemSelector::Block {
                    first: b * 5 + 1,
                    last: b * 5 + 5,
                },
                other_names: vec![],
            });
        }
        assert_eq!(computed_range(&s), Some((20.0, 100.0)));
    }

    #[test]
    fn default_recode_reflects() {
        let s = new_blank(Format::LikertBlankLine);
        assert_eq!(
            recode_table(&s),
            vec![(1, 5), (2, 4), (3, 3), (4, 2), (5, 1)]
        );
    }
}
