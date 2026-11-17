//! The scale record (specification, "Scale data model"). One `Scale` holds everything a stored scale file carries, plus where
//! each field came from. It is saved as one `.pscale` JSON file (see `pscale.rs` and `docs/PSCALE-FORMAT.md`).

use serde::{Deserialize, Serialize};

/// The `.pscale` schema version written into every file. Raise it, and add a step to `pscale::migrate`, whenever a field
/// changes meaning or a required field is added.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scale {
    /// Always `SCHEMA_VERSION` when written by this build.
    pub pscale_version: u32,
    /// A random identifier that stays with the scale across saves and copies.
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    /// When true, a number in the title such as the 16 in "Homophobia-16 Scale" is kept equal to the item count.
    #[serde(default)]
    pub count_suffix: bool,
    /// The researcher's own tags, in the order added. No fixed list (specification addendum, section 1).
    #[serde(default)]
    pub tags: Vec<String>,
    pub format: Format,
    pub response: ResponseScale,
    pub instructions: Instructions,
    pub items: Vec<Item>,
    pub recode: RecodeRule,
    #[serde(default)]
    pub subscales: Vec<Subscale>,
    pub scoring: Scoring,
    #[serde(default)]
    pub score_range: ScoreRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Citation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapted_from: Option<Citation>,
    /// Where sources print in an export. Default: under the scale, as in the stored files.
    #[serde(default)]
    pub source_placement: SourcePlacement,
    pub licence: Licence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub development: Option<DevelopmentRecord>,
    #[serde(default)]
    pub provenance: Vec<Provenance>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    /// Lines from an imported file that no rule understood, kept so the confirm form can show them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unparsed: Vec<UnparsedLine>,
    /// The researcher's decisions on self-check findings: accepted (kept as it is) or dismissed, with the reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub check_decisions: Vec<CheckDecision>,
    /// The Comm Add On entry this scale was built from, when it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_entry: Option<String>,
    /// Self-check findings that sit in the original publication, each with what the original says and whether this
    /// copy keeps it as published or corrects it (specification, "Source notes: kept as published").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_notes: Vec<SourceNote>,
    /// For a variant reworded for a new target: the parent scale, the version it branched from, the old and new
    /// target, and the item map back to the original (specification, "Change the context").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_of: Option<VariantOf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceNoteHandling {
    /// This copy keeps the item text, recode list or range exactly as the source prints it.
    Kept,
    /// This copy corrects it; the scoring key shows both what the original printed and what this copy scores.
    Corrected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceNote {
    /// The key of the self-check finding the note is about (see `selfcheck::Finding::key`).
    pub key: String,
    /// The field or check the note is about ("Recode list against wording").
    pub subject: String,
    /// Items the note is about.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<u32>,
    /// What the original publication says, in the researcher's words.
    pub original: String,
    /// The page of the original, as the researcher gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    pub handling: SourceNoteHandling,
    /// For a correction: what this copy does instead ("item 4 is scored as not reversed").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
    /// UTC, RFC 3339.
    pub noted_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantOf {
    /// The parent scale's `id`.
    pub parent_id: String,
    pub parent_title: String,
    /// The parent's saved version (its `history` number) the variant branched from.
    pub parent_version: u32,
    /// The target the parent names ("physician") and the one the variant names ("AI").
    pub old_target: String,
    pub new_target: String,
    /// An optional line of context ("a chatbot used for health questions").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Further wording to replace beside the target itself ("my doctor" to "the AI I use").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replacements: Vec<Replacement>,
    /// Each variant item beside its original number and wording.
    #[serde(default)]
    pub item_map: Vec<ItemMapEntry>,
    /// The parent's structure when the variant was made, which the variant keeps locked: the self-check compares the
    /// variant with it, so the check needs no access to the parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<LockedStructure>,
    /// Changes in the parent the researcher chose not to carry into the variant, so they are not offered again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declined: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemMapEntry {
    /// The variant item's number.
    pub number: u32,
    pub original_number: u32,
    pub original_text: String,
    /// Set when the item could not be reworded without changing what it asks, with the researcher's reason. The item
    /// keeps the original wording and the self-check keeps it in view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flagged: Option<String>,
}

/// What a variant keeps locked from its parent (specification, "Change the context"): item count and order, points
/// and anchors, reverse items, subscales, the recode rule and the scoring rule. The score range follows from these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockedStructure {
    pub item_count: u32,
    pub response: ResponseScale,
    pub reversed: Vec<u32>,
    #[serde(default)]
    pub subscales: Vec<Subscale>,
    pub recode: RecodeRule,
    pub scoring: Scoring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecisionStatus {
    /// The finding is right, and the scale is kept as it is (for example, a slip kept word for word from the source).
    Accepted,
    /// The finding does not apply.
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDecision {
    /// The finding's key (see `selfcheck::Finding::key`).
    pub key: String,
    pub status: DecisionStatus,
    #[serde(default)]
    pub reason: String,
    /// UTC, RFC 3339.
    pub decided_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    /// Anchor row, then one line per item: "_____1. item". The respondent writes a number in the blank.
    LikertBlankLine,
    /// Header row of abbreviations (SD, D, N, A, SA) and one row per item with a box under each.
    LikertBoxGrid,
    /// Item number, left adjective, the numbers 1 to n, right adjective. The respondent circles one number.
    SemanticDifferential,
}

impl Format {
    pub const ALL: [Format; 3] = [
        Format::LikertBlankLine,
        Format::LikertBoxGrid,
        Format::SemanticDifferential,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Format::LikertBlankLine => "Likert blank-line",
            Format::LikertBoxGrid => "Likert box grid",
            Format::SemanticDifferential => "Semantic differential",
        }
    }

    pub fn default_points(self) -> u8 {
        match self {
            Format::SemanticDifferential => 7,
            _ => 5,
        }
    }

    /// The number of blank lines a new scale starts with (the stored templates).
    pub fn template_lines(self) -> usize {
        match self {
            Format::SemanticDifferential => 30,
            _ => 40,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AnchorSet {
    #[default]
    Agreement,
    Frequency,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LabelMode {
    /// A label for every point.
    #[default]
    Every,
    /// Labels on the two ends only.
    EndsOnly,
}

/// The response points: how many, what each is called and what each scores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseScale {
    pub points: u8,
    pub anchor_set: AnchorSet,
    pub label_mode: LabelMode,
    /// One entry per point, lowest first. `None` for a point with no label (ends-only labelling).
    pub labels: Vec<Option<String>>,
    /// Short forms for a box grid header (SD, D, N, A, SA), one per point.
    #[serde(default)]
    pub abbreviations: Vec<String>,
    /// The number each point scores, lowest first. Usually 1 to `points`.
    pub values: Vec<i32>,
}

impl ResponseScale {
    pub fn lowest(&self) -> i32 {
        self.values.iter().copied().min().unwrap_or(1)
    }
    pub fn highest(&self) -> i32 {
        self.values
            .iter()
            .copied()
            .max()
            .unwrap_or(self.points as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstructionsOrigin {
    /// Built from the house template; rewritten whenever the format, point count or anchors change.
    House,
    /// Kept word for word from the source; never rewritten.
    Verbatim,
    /// Edited by hand; never rewritten again.
    Edited,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instructions {
    pub text: String,
    pub origin: InstructionsOrigin,
    /// The opening sentence of house instructions (what the scale is about). The format-specific sentences follow it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_stem: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ReverseState {
    #[default]
    NotReversed,
    /// Suggested by the tool (or found in the source) and waiting for the researcher.
    Suggested,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReverseFlag {
    pub state: ReverseState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ReverseFlag {
    pub fn is_reversed(&self) -> bool {
        self.state == ReverseState::Confirmed
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub number: u32,
    /// Verbatim item text. For a semantic differential this is empty or a short label; the pair is in `left`/`right`.
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<String>,
    #[serde(default)]
    pub reverse: ReverseFlag,
    /// The text as it stood in the source, kept once the item is edited away from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    /// Text printed just above this item, as a prompt for the block it starts ("Now answer as a sailor would"). Used
    /// when a source asks the same items again under a new perspective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead_in: Option<String>,
}

impl Item {
    pub fn new(number: u32, text: impl Into<String>) -> Self {
        Item {
            number,
            text: text.into(),
            left: None,
            right: None,
            reverse: ReverseFlag::default(),
            source_text: None,
            lead_in: None,
        }
    }
    pub fn changed_from_source(&self) -> bool {
        self.source_text.as_deref().is_some_and(|s| s != self.text)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum RecodeKind {
    /// The default: (lowest + highest) minus the answer.
    Reflect,
    /// An explicit table, answer to scored value.
    Table { pairs: Vec<(i32, i32)> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecodeRule {
    pub rule: RecodeKind,
    /// The recode list as the source states it, repeats and all ("17, 17"), so the self-check can see slips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stated: Option<StatedList>,
}

impl Default for RecodeRule {
    fn default() -> Self {
        RecodeRule {
            rule: RecodeKind::Reflect,
            stated: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatedList {
    pub numbers: Vec<u32>,
    /// The line it came from, word for word.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ItemSelector {
    /// Item numbers as listed, repeats kept.
    List {
        numbers: Vec<u32>,
    },
    Odd,
    Even,
    /// A block of consecutive items, inclusive.
    Block {
        first: u32,
        last: u32,
    },
}

impl ItemSelector {
    /// The item numbers this selects from a scale with `count` items, in order, without repeats.
    pub fn resolve(&self, count: u32) -> Vec<u32> {
        let mut out: Vec<u32> = match self {
            ItemSelector::List { numbers } => numbers.clone(),
            ItemSelector::Odd => (1..=count).filter(|n| n % 2 == 1).collect(),
            ItemSelector::Even => (1..=count).filter(|n| n % 2 == 0).collect(),
            ItemSelector::Block { first, last } => (*first..=*last).collect(),
        };
        let mut seen = std::collections::HashSet::new();
        out.retain(|n| seen.insert(*n));
        out
    }

    pub fn describe(&self) -> String {
        match self {
            ItemSelector::List { numbers } => numbers
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            ItemSelector::Odd => "odd items".into(),
            ItemSelector::Even => "even items".into(),
            ItemSelector::Block { first, last } => format!("{first}-{last}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscale {
    pub name: String,
    pub items: ItemSelector,
    /// Other names the source uses for this subscale (for example in a factor table), kept for the self-check.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ScoreMethod {
    #[default]
    Sum,
    Mean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TotalFrom {
    #[default]
    Items,
    Subscales,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scoring {
    pub subscale_method: ScoreMethod,
    pub total_method: ScoreMethod,
    pub total_from: TotalFrom,
    /// Whether the scale has a total score at all (some report subscales only).
    pub has_total: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_name: Option<String>,
    /// Every other name the source gives the total, kept for the self-check ("Score names agree").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_total_names: Vec<String>,
    /// The scoring sentences as the source states them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stated: Vec<String>,
    /// How the source says to read the scores, line by line ("3-5 Very calm"). Printed in the researcher copy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interpretation: Vec<String>,
}

impl Default for Scoring {
    fn default() -> Self {
        Scoring {
            subscale_method: ScoreMethod::Sum,
            total_method: ScoreMethod::Sum,
            total_from: TotalFrom::Items,
            has_total: true,
            total_name: None,
            other_total_names: Vec::new(),
            stated: Vec::new(),
            interpretation: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScoreRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stated: Option<(f64, f64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stated_text: Option<String>,
    /// The range the tool computes for the total, refreshed on every save.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computed: Option<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    /// The reference as APA 7 text. Kept even when linked, so a scale opens without Prism Reference Manager.
    pub apa: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// The item's identifier in Prism Reference Manager, once linked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_manager_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SourcePlacement {
    #[default]
    UnderScale,
    References,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LicenceStatus {
    Open,
    Permissioned,
    LocalOnly,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Licence {
    pub status: LicenceStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The development item pool and its factor loadings, mapped to the final numbering by wording, never by number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentRecord {
    /// Factor names in column order, as the table's header gives them.
    pub factors: Vec<String>,
    /// Other names given for the factors (for example "Factor II - Democratic Factor" below the table).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub factor_labels: Vec<String>,
    pub items: Vec<DevelopmentItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentItem {
    pub number: u32,
    pub text: String,
    pub loadings: Vec<Option<f64>>,
    /// True when the source marks the item as kept (bold), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marked_kept: Option<bool>,
    /// The final item it became, matched by wording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_number: Option<u32>,
    /// How closely the wording matched (1.0 = identical after spacing fixes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_score: Option<f64>,
}

/// Where one field came from. `region` is (x, y, width, height) on the page, for journal pages (phase 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// The field, such as "title", "instructions", "anchors", "item:3", "recode", "subscale:Technical", "source".
    pub field: String,
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// The paragraph (line) in the file, counting from 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<(f64, f64, f64, f64)>,
    /// 0 to 1.
    pub confidence: f64,
    /// What rule found it, in plain words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub version: u32,
    /// UTC, RFC 3339.
    pub saved_at: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnparsedLine {
    pub line: u32,
    pub text: String,
}
