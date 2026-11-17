//! A study (specification, "Study builder and Word export"): an ordered stack of scales, with optional pieces between
//! them, exported as one Word document that drops into a larger questionnaire. Each scale keeps its own title,
//! instructions, anchors and numbering. A study is saved as one `.pstudy` JSON file that carries its own copy of each
//! scale (so it opens anywhere) and the library file it came from (so it can be brought up to date).

use crate::model::{HistoryEntry, Scale, SourcePlacement};
use crate::pscale::{now_rfc3339, write_atomic, PscaleError};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const STUDY_VERSION: u32 = 1;
pub const STUDY_EXTENSION: &str = "pstudy";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Numbering {
    /// Each scale starts again at 1, as in the study template.
    #[default]
    RestartEach,
    /// Numbers run on through the study.
    RunOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TitleMode {
    /// The scale's title, as in the stored files.
    #[default]
    Show,
    /// A neutral label instead ("Section 3"), for the respondent copy.
    Neutral,
    Hide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StudyOptions {
    pub numbering: Numbering,
    pub titles: TitleMode,
    /// The word before a neutral label's number.
    pub neutral_word: String,
    /// Where sources print: under each scale, in a References section at the end, or not at all.
    pub sources: SourcePlacement,
    pub font: String,
    pub size_pt: u32,
    /// A page break before each scale after the first.
    pub break_between_scales: bool,
}

impl Default for StudyOptions {
    fn default() -> Self {
        StudyOptions {
            numbering: Numbering::RestartEach,
            titles: TitleMode::Show,
            neutral_word: "Section".into(),
            sources: SourcePlacement::UnderScale,
            font: "Times New Roman".into(),
            size_pt: 12,
            break_between_scales: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
// A study holds a handful of parts, so a scale held in place costs nothing worth boxing it for.
#[allow(clippy::large_enum_variant)]
pub enum Part {
    Scale {
        scale: Scale,
        /// The library file this copy came from, when it came from one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        library_path: Option<String>,
    },
    PageBreak,
    /// A short sentence between scales ("The next questions ask about your workplace.").
    Transition {
        text: String,
    },
    /// A block the researcher types: demographic questions, one per line.
    Demographics {
        title: String,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Study {
    pub pstudy_version: u32,
    pub id: String,
    pub title: String,
    pub parts: Vec<Part>,
    #[serde(default)]
    pub options: StudyOptions,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

impl Study {
    pub fn new(title: &str) -> Study {
        Study {
            pstudy_version: STUDY_VERSION,
            id: crate::scale::new_id(),
            title: title.into(),
            parts: Vec::new(),
            options: StudyOptions::default(),
            history: Vec::new(),
        }
    }

    pub fn scales(&self) -> impl Iterator<Item = &Scale> {
        self.parts.iter().filter_map(|p| match p {
            Part::Scale { scale, .. } => Some(scale),
            _ => None,
        })
    }

    /// The item number each scale's first item prints as (1 for every scale unless numbering runs on).
    pub fn number_offsets(&self) -> Vec<u32> {
        let mut out = Vec::new();
        let mut next = 0u32;
        for s in self.scales() {
            out.push(match self.options.numbering {
                Numbering::RestartEach => 0,
                Numbering::RunOn => next,
            });
            next += s.items.len() as u32;
        }
        out
    }

    /// Total items, and a rough time to complete (about seven seconds an item, a common rule of thumb).
    pub fn length(&self) -> (usize, u32) {
        let items: usize = self.scales().map(|s| s.items.len()).sum();
        (items, (items as u32 * 7).div_ceil(60))
    }
}

pub fn study_from_json(text: &str) -> Result<Study, PscaleError> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let version = value
        .get("pstudyVersion")
        .and_then(|v| v.as_u64())
        .ok_or(PscaleError::NoVersion)? as u32;
    if version > STUDY_VERSION {
        return Err(PscaleError::TooNew(version));
    }
    Ok(serde_json::from_value(value)?)
}

pub fn load_study(path: &Path) -> Result<Study, PscaleError> {
    let text = std::fs::read_to_string(path)?;
    study_from_json(text.trim_start_matches('\u{feff}'))
}

pub fn save_study(study: &mut Study, path: &Path, summary: &str) -> Result<(), PscaleError> {
    study.pstudy_version = STUDY_VERSION;
    let version = study.history.last().map(|h| h.version + 1).unwrap_or(1);
    study.history.push(HistoryEntry {
        version,
        saved_at: now_rfc3339(),
        summary: summary.into(),
    });
    write_atomic(
        path,
        serde_json::to_string_pretty(study)
            .expect("a study always serialises")
            .as_bytes(),
    )
}

/// Brings each scale up to date from its library file, when that file is newer. Returns the titles brought up to date.
pub fn refresh_from_library(study: &mut Study) -> Vec<String> {
    let mut changed = Vec::new();
    for p in &mut study.parts {
        if let Part::Scale {
            scale,
            library_path: Some(path),
        } = p
        {
            if let Ok(fresh) = crate::pscale::load(Path::new(path)) {
                let newer = fresh.history.last().map(|h| h.saved_at.clone())
                    > scale.history.last().map(|h| h.saved_at.clone());
                if newer {
                    changed.push(fresh.title.clone());
                    *scale = fresh;
                }
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Format;
    use crate::scale::new_blank;

    fn scale(n: usize) -> Scale {
        let mut s = new_blank(Format::LikertBlankLine);
        s.items.truncate(n);
        s
    }

    #[test]
    fn numbering_restarts_or_runs_on() {
        let mut st = Study::new("Spring study");
        st.parts.push(Part::Scale {
            scale: scale(10),
            library_path: None,
        });
        st.parts.push(Part::PageBreak);
        st.parts.push(Part::Scale {
            scale: scale(5),
            library_path: None,
        });
        st.parts.push(Part::Scale {
            scale: scale(3),
            library_path: None,
        });
        assert_eq!(st.number_offsets(), vec![0, 0, 0]);
        st.options.numbering = Numbering::RunOn;
        assert_eq!(st.number_offsets(), vec![0, 10, 15]);
        assert_eq!(st.length(), (18, 3));
    }

    #[test]
    fn a_study_round_trips() {
        let dir = std::env::temp_dir().join(format!("ps-study-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.pstudy");
        let mut st = Study::new("A");
        st.parts.push(Part::Transition {
            text: "Next, some questions about you.".into(),
        });
        save_study(&mut st, &path, "Created").unwrap();
        assert_eq!(load_study(&path).unwrap(), st);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
