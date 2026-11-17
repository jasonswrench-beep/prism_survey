//! The scoring self-check (specification, "Scoring self-check"). Every scale is checked for scoring errors before
//! export. Findings warn and never block: each is accepted, fixed, or dismissed with a reason the scale keeps, and any
//! still open print in the researcher copy.
//!
//! Item direction is judged against the other items, never by counting negation words alone (the specification's two
//! guards: a negated item can score with the construct, and an item with no negation can be reversed). Each item's
//! wording gives a polarity (negations and words such as "limited" or "avoid" turn it), and items that mirror each
//! other (the same words with one turned) tie their directions together. A recode list is then read as one of two
//! mappings, "the recoded items are the turned ones" or "the recoded items are the plain ones"; the mapping that fits
//! the most items is taken, and the items that do not fit it are reported, with the evidence.

use crate::instructions::{fit_findings, is_forced_choice};
use crate::model::*;
use crate::scale::{computed_range, filled_items, title_count};
use crate::text::{edit_distance, stem, wording_match, words};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckKind {
    RecodeWording,
    RecodeMissing,
    LateItem,
    ListRepeat,
    DuplicateItem,
    SubscaleCoverage,
    SubscaleNames,
    ItemCount,
    ScoreRange,
    InstructionsFormat,
    LeftoverSource,
    TypingSlip,
    ScoreNames,
    ForcedChoice,
    VariantStructure,
}

impl CheckKind {
    pub fn label(self) -> &'static str {
        match self {
            CheckKind::RecodeWording => "Recode list against wording",
            CheckKind::RecodeMissing => "Recode list missing",
            CheckKind::LateItem => "Late-added item",
            CheckKind::ListRepeat => "Repeats in a list",
            CheckKind::DuplicateItem => "Duplicate items",
            CheckKind::SubscaleCoverage => "One subscale per item",
            CheckKind::SubscaleNames => "Subscale names agree",
            CheckKind::ItemCount => "Item count against title",
            CheckKind::ScoreRange => "Score range",
            CheckKind::InstructionsFormat => "Instructions against format",
            CheckKind::LeftoverSource => "Leftover source",
            CheckKind::TypingSlip => "Typing slips",
            CheckKind::ScoreNames => "Score names agree",
            CheckKind::ForcedChoice => "Forced choice",
            CheckKind::VariantStructure => "Variant structure held",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Probably a scoring error.
    Warning,
    /// Worth knowing; not an error in itself.
    Note,
}

/// A change the researcher can make in one step from a finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Fix {
    SetReversed {
        item: u32,
        reversed: bool,
    },
    /// Replace one repeat in the stated recode list (`with` None removes it).
    RecodeRepeat {
        number: u32,
        with: Option<u32>,
    },
    SyncTitleCount,
    UseComputedRange,
    ClearSource,
    UseHouseInstructions,
    UseTotalName {
        name: String,
    },
    /// Puts back what a variant keeps locked from its parent: points, anchors, reverse items, subscales, recode rule
    /// and scoring.
    RestoreLockedStructure,
}

impl Fix {
    pub fn label(&self) -> String {
        match self {
            Fix::SetReversed {
                item,
                reversed: true,
            } => format!("Mark item {item} as reversed"),
            Fix::SetReversed {
                item,
                reversed: false,
            } => format!("Mark item {item} as not reversed"),
            Fix::RecodeRepeat {
                number,
                with: Some(w),
            } => format!("Change the second {number} to {w}"),
            Fix::RecodeRepeat { number, with: None } => format!("Remove the second {number}"),
            Fix::SyncTitleCount => "Make the title match the item count".into(),
            Fix::UseComputedRange => "Use the computed range".into(),
            Fix::ClearSource => "Remove the source".into(),
            Fix::UseHouseInstructions => "Use house instructions for this format".into(),
            Fix::UseTotalName { name } => format!("Call the total \"{name}\" throughout"),
            Fix::RestoreLockedStructure => "Put back the original's structure".into(),
        }
    }

    /// What the scale does once the fix is made, for the record of a correction from the source.
    pub fn done_label(&self) -> String {
        match self {
            Fix::SetReversed {
                item,
                reversed: true,
            } => format!("item {item} is scored as reversed"),
            Fix::SetReversed {
                item,
                reversed: false,
            } => format!("item {item} is scored as not reversed"),
            Fix::RecodeRepeat {
                number,
                with: Some(w),
            } => format!("the second {number} in the list is read as {w}"),
            Fix::RecodeRepeat { number, with: None } => {
                format!("the second {number} in the list is left out")
            }
            Fix::SyncTitleCount => "the title gives the number of items the scale has".into(),
            Fix::UseComputedRange => "the score range is the one computed from the items".into(),
            Fix::ClearSource => "the source is removed".into(),
            Fix::UseHouseInstructions => "the instructions are rewritten for this format".into(),
            Fix::UseTotalName { name } => format!("the total is called \"{name}\" throughout"),
            Fix::RestoreLockedStructure => "the original's structure is put back".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FindingStatus {
    Open,
    Accepted,
    Dismissed,
    /// The finding is in the original publication and this copy keeps it as printed. It stays visible, with its
    /// source note, rather than closing.
    Published,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Stable for the same problem across runs, so a decision stays with it ("recodeWording:4").
    pub key: String,
    pub check: CheckKind,
    pub check_label: String,
    pub severity: Severity,
    pub message: String,
    /// Items the finding is about.
    pub items: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<Fix>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fix_labels: Vec<String>,
    pub status: FindingStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn finding(
    key: String,
    check: CheckKind,
    severity: Severity,
    message: String,
    items: Vec<u32>,
    fixes: Vec<Fix>,
) -> Finding {
    let fix_labels = fixes.iter().map(Fix::label).collect();
    Finding {
        key,
        check,
        check_label: check.label().to_string(),
        severity,
        message,
        items,
        fixes,
        fix_labels,
        status: FindingStatus::Open,
        reason: None,
    }
}

fn list(n: &[u32]) -> String {
    match n.len() {
        0 => String::new(),
        1 => n[0].to_string(),
        2 => format!("{} and {}", n[0], n[1]),
        _ => format!(
            "{}, and {}",
            n[..n.len() - 1]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            n[n.len() - 1]
        ),
    }
}

fn items_word(n: &[u32]) -> &'static str {
    if n.len() == 1 {
        "Item"
    } else {
        "Items"
    }
}

// ---------------------------------------------------------------- item direction

const NEGATIONS: &[&str] = &[
    "not", "no", "never", "nor", "none", "neither", "nobody", "nothing", "cannot", "without",
    "seldom", "hardly",
];

/// Words that turn an item's direction the way a negation does.
const TURNING: &[&str] = &[
    "avoid",
    "avoids",
    "avoided",
    "avoiding",
    "avoidance",
    "flee",
    "flees",
    "limited",
    "uncomfortable",
    "unclear",
    "unable",
    "unwilling",
    "uninvolved",
    "unwelcome",
    "uneasy",
    "nervous",
    "anxious",
    "anxiety",
    "fear",
    "fears",
    "afraid",
    "worry",
    "worried",
    "worries",
    "tense",
    "dread",
    "waste",
    "derogatory",
    "tease",
    "refuse",
    "refused",
    "lack",
    "lacks",
    "fail",
    "failed",
    "ignore",
    "ignored",
    "difficult",
    "confused",
    "bored",
    "boring",
    "dislike",
    "hate",
];

/// Words that mark an item as plainly worded in the positive direction. An item with no negation, no turning word and
/// none of these is too neutral to judge from its wording alone, unless it mirrors another item.
const POSITIVE_CUES: &[&str] = &[
    "allowed",
    "legal",
    "normal",
    "respect",
    "respects",
    "enjoy",
    "enjoys",
    "enjoying",
    "comfortable",
    "comfortably",
    "clear",
    "clearly",
    "completely",
    "understand",
    "understands",
    "understood",
    "close",
    "friendship",
    "friendships",
    "protection",
    "rights",
    "care",
    "cares",
    "cared",
    "helped",
    "help",
    "helps",
    "satisfied",
    "welcome",
    "calm",
    "relaxed",
    "ease",
    "happily",
    "happy",
    "sensible",
    "trust",
    "support",
    "supportive",
    "agree",
    "pleasing",
    "knowledgeable",
    "comprehend",
    "responsive",
    "communicative",
    "listened",
    "listen",
    "listens",
    "wanted",
    "confident",
    "steady",
    "informed",
];

const STOP: &[&str] = &[
    "the", "a", "an", "to", "of", "and", "or", "i", "me", "my", "is", "was", "were", "be", "are",
    "am", "been", "he", "she", "his", "her", "it", "its", "in", "on", "at", "for", "with", "that",
    "this", "about", "during", "as", "by", "from", "when", "what", "who", "which", "their", "them",
    "they", "our", "us", "we", "you", "your", "would", "do", "does", "did", "have", "has", "had",
    "very", "much", "more", "most", "some", "any", "all", "s", "or", "if", "than", "will", "can",
    "could", "should", "may", "might", "must", "there", "these", "those", "so", "too", "also",
    "often",
];

fn is_negation(w: &str) -> bool {
    NEGATIONS.contains(&w) || w.ends_with("n't")
}

/// What the wording of one item says about its direction.
#[derive(Debug, Clone)]
struct Wording {
    number: u32,
    /// +1 plain, -1 turned (an odd number of negations and turning words).
    polarity: i8,
    /// The wording carries an explicit cue for its polarity.
    cued: bool,
    /// The cue words, for the explanation.
    cues: Vec<String>,
    /// Stems of the content words, without negations, turning words or stop words.
    content: BTreeSet<String>,
}

fn item_text(it: &Item) -> String {
    if it.text.trim().is_empty() {
        format!(
            "{} {}",
            it.left.clone().unwrap_or_default(),
            it.right.clone().unwrap_or_default()
        )
    } else {
        it.text.clone()
    }
}

fn wording(it: &Item) -> Wording {
    let ws = words(&item_text(it));
    let mut turns = 0;
    let mut cues = Vec::new();
    let mut positive = Vec::new();
    let mut content = BTreeSet::new();
    for (k, w) in ws.iter().enumerate() {
        // "or not" ("whether I was informed or not") does not turn the item.
        let idiom = w == "not" && k > 0 && ws[k - 1] == "or";
        if !idiom && (is_negation(w) || TURNING.contains(&w.as_str())) {
            turns += 1;
            cues.push(w.clone());
        } else {
            if POSITIVE_CUES.contains(&w.as_str()) {
                positive.push(w.clone());
            }
            if !STOP.contains(&w.as_str()) {
                content.insert(stem(w));
            }
        }
    }
    let polarity = if turns % 2 == 1 { -1 } else { 1 };
    let (cued, cues) = if polarity == -1 {
        (true, cues)
    } else if !positive.is_empty() {
        (true, positive)
    } else {
        (!cues.is_empty(), cues)
    };
    Wording {
        number: it.number,
        polarity,
        cued,
        cues,
        content,
    }
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    inter / a.union(b).count() as f64
}

/// Items that say the same thing with one of them turned ("I know X" and "I do not know X").
fn mirrors(ws: &[Wording]) -> HashMap<u32, Vec<u32>> {
    let mut out: HashMap<u32, Vec<u32>> = HashMap::new();
    for (i, a) in ws.iter().enumerate() {
        for b in &ws[i + 1..] {
            if a.polarity != b.polarity
                && a.content.len() >= 2
                && b.content.len() >= 2
                && jaccard(&a.content, &b.content) >= 0.6
            {
                out.entry(a.number).or_default().push(b.number);
                out.entry(b.number).or_default().push(a.number);
            }
        }
    }
    out
}

/// One item whose recode status does not fit its wording.
#[derive(Debug, Clone)]
struct Misfit {
    number: u32,
    reversed: bool,
    polarity: i8,
    cues: Vec<String>,
    mirror: Option<u32>,
}

/// Judges one set of items (a subscale, or the whole scale) against the recode list. Returns the misfits, or nothing
/// when the wording cannot decide (too few informative items, or no contrast between the recoded and kept items).
fn direction_misfits(items: &[&Item]) -> Vec<Misfit> {
    let ws: Vec<Wording> = items.iter().map(|i| wording(i)).collect();
    let mir = mirrors(&ws);
    let reversed: HashSet<u32> = items
        .iter()
        .filter(|i| i.reverse.is_reversed())
        .map(|i| i.number)
        .collect();
    let informative: Vec<&Wording> = ws
        .iter()
        .filter(|w| w.cued || mir.contains_key(&w.number))
        .collect();
    if informative.len() < 4 {
        return Vec::new();
    }
    // Mapping A: recoded items are the turned ones (-1). Mapping B: recoded items are the plain ones (+1).
    let fits = |w: &Wording, recoded_polarity: i8| {
        reversed.contains(&w.number) == (w.polarity == recoded_polarity)
    };
    let mut best: Option<(i8, usize)> = None;
    for rp in [-1i8, 1] {
        let misfits = informative.iter().filter(|w| !fits(w, rp)).count();
        // The mapping must be supported on both sides: some recoded and some kept items fit it.
        let rec_ok = informative
            .iter()
            .any(|w| reversed.contains(&w.number) && fits(w, rp));
        let kept_ok = informative
            .iter()
            .any(|w| !reversed.contains(&w.number) && fits(w, rp));
        if !(rec_ok && kept_ok) {
            continue;
        }
        if best.is_none_or(|(_, m)| misfits < m) {
            best = Some((rp, misfits));
        } else if best.is_some_and(|(_, m)| misfits == m) {
            // A tie: the wording cannot say which way the list should run.
            return Vec::new();
        }
    }
    let Some((rp, count)) = best else {
        return Vec::new();
    };
    if count == 0 || count * 100 > informative.len() * 45 {
        return Vec::new();
    }
    informative
        .iter()
        .filter(|w| !fits(w, rp))
        .map(|w| Misfit {
            number: w.number,
            reversed: reversed.contains(&w.number),
            polarity: w.polarity,
            cues: w.cues.clone(),
            mirror: mir.get(&w.number).and_then(|m| m.first().copied()),
        })
        .collect()
}

fn describe_misfit(m: &Misfit, it: &Item) -> String {
    let words_part = if m.cues.is_empty() {
        String::new()
    } else {
        format!(" (\"{}\")", m.cues.join("\", \""))
    };
    let mirror = m
        .mirror
        .map(|x| format!(" It mirrors item {x}."))
        .unwrap_or_default();
    let quote = if it.text.trim().is_empty() {
        String::new()
    } else {
        format!(" \"{}\"", it.text.trim())
    };
    if m.reversed {
        format!(
            "Item {}{quote} is recoded, but its wording{words_part} runs the same way as the items that are not recoded.{mirror}",
            m.number
        )
    } else {
        let way = if m.polarity < 0 { "against" } else { "with" };
        format!(
            "Item {}{quote} is not recoded, but its wording{words_part} runs {way} the construct, like the recoded items.{mirror}",
            m.number
        )
    }
}

/// The sets of items judged together: each subscale that is a list, block or odd/even split, or the whole scale.
fn direction_sets(scale: &Scale) -> Vec<(String, Vec<&Item>)> {
    let count = scale.items.len() as u32;
    let filled: Vec<&Item> = scale
        .items
        .iter()
        .filter(|i| !item_text(i).trim().is_empty())
        .collect();
    let mut sets = Vec::new();
    for s in &scale.subscales {
        let nums: HashSet<u32> = s.items.resolve(count).into_iter().collect();
        let members: Vec<&Item> = filled
            .iter()
            .copied()
            .filter(|i| nums.contains(&i.number))
            .collect();
        if members.len() >= 3 {
            sets.push((s.name.clone(), members));
        }
    }
    let covered: HashSet<u32> = sets
        .iter()
        .flat_map(|(_, m)| m.iter().map(|i| i.number))
        .collect();
    if sets.is_empty() {
        sets.push(("the scale".into(), filled));
    } else {
        let rest: Vec<&Item> = filled
            .iter()
            .copied()
            .filter(|i| !covered.contains(&i.number))
            .collect();
        if rest.len() >= 3 {
            sets.push(("the items in no subscale".into(), rest));
        }
    }
    sets
}

// ---------------------------------------------------------------- the checks

fn check_recode(scale: &Scale, out: &mut Vec<Finding>) {
    // Reverse suggestions waiting for the researcher come first; the direction checks wait for them.
    let waiting: Vec<u32> = scale
        .items
        .iter()
        .filter(|i| i.reverse.state == ReverseState::Suggested)
        .map(|i| i.number)
        .collect();
    if !waiting.is_empty() {
        out.push(finding(
            "reverseSuggestions".into(),
            CheckKind::RecodeWording,
            Severity::Note,
            format!(
                "{} {} {} suggested as reversed and waiting for you: accept or reject each one in the item list.",
                items_word(&waiting),
                list(&waiting),
                if waiting.len() == 1 { "is" } else { "are" }
            ),
            waiting,
            vec![],
        ));
        return;
    }
    let any_reversed = scale.items.iter().any(|i| i.reverse.is_reversed());
    if !any_reversed {
        // Recode list missing: the items run both ways and nothing is recoded.
        let filled: Vec<&Item> = scale
            .items
            .iter()
            .filter(|i| !item_text(i).trim().is_empty())
            .collect();
        let ws: Vec<Wording> = filled.iter().map(|i| wording(i)).collect();
        let turned: Vec<u32> = ws
            .iter()
            .filter(|w| w.polarity < 0)
            .map(|w| w.number)
            .collect();
        let plain = ws.len() - turned.len();
        if ws.len() >= 4 && turned.len() >= 2 && plain >= 2 && turned.len() * 100 >= ws.len() * 15 {
            let (minority, word) = if turned.len() <= plain {
                (turned.clone(), "against")
            } else {
                (
                    ws.iter()
                        .filter(|w| w.polarity > 0)
                        .map(|w| w.number)
                        .collect(),
                    "the other way from",
                )
            };
            out.push(finding(
                "recodeMissing".into(),
                CheckKind::RecodeMissing,
                Severity::Warning,
                format!(
                    "The scale has no recode list, but {} {} {} worded {word} the others and likely need recoding.",
                    items_word(&minority).to_lowercase(),
                    list(&minority),
                    if minority.len() == 1 { "is" } else { "are" }
                ),
                minority.clone(),
                minority.iter().map(|&n| Fix::SetReversed { item: n, reversed: true }).collect(),
            ));
        }
        return;
    }
    let last = scale
        .items
        .iter()
        .filter(|i| !item_text(i).trim().is_empty())
        .map(|i| i.number)
        .max()
        .unwrap_or(0);
    let max_recoded = scale
        .items
        .iter()
        .filter(|i| i.reverse.is_reversed())
        .map(|i| i.number)
        .max()
        .unwrap_or(0);
    let mut seen = HashSet::new();
    for (_set, members) in direction_sets(scale) {
        for m in direction_misfits(&members) {
            if !seen.insert(m.number) {
                continue;
            }
            let Some(it) = scale.items.iter().find(|i| i.number == m.number) else {
                continue;
            };
            let fix = Fix::SetReversed {
                item: m.number,
                reversed: !m.reversed,
            };
            if !m.reversed && m.number == last && m.number > max_recoded {
                let recoded: Vec<u32> = scale
                    .items
                    .iter()
                    .filter(|i| i.reverse.is_reversed())
                    .map(|i| i.number)
                    .collect();
                out.push(finding(
                    format!("lateItem:{}", m.number),
                    CheckKind::LateItem,
                    Severity::Warning,
                    format!(
                        "Item {}, the last item, reads in the same direction as recoded items {} but is not on the recode list. It may have been added after the list was written.",
                        m.number,
                        list(&recoded)
                    ),
                    vec![m.number],
                    vec![fix],
                ));
            } else {
                out.push(finding(
                    format!("recodeWording:{}", m.number),
                    CheckKind::RecodeWording,
                    Severity::Warning,
                    describe_misfit(&m, it),
                    vec![m.number],
                    vec![fix],
                ));
            }
        }
    }
}

fn check_repeats(scale: &Scale, out: &mut Vec<Finding>) {
    let flagged_missing: HashSet<u32> = out
        .iter()
        .filter(|f| matches!(f.check, CheckKind::RecodeWording | CheckKind::LateItem))
        .flat_map(|f| f.fixes.iter())
        .filter_map(|x| match x {
            Fix::SetReversed {
                item,
                reversed: true,
            } => Some(*item),
            _ => None,
        })
        .collect();
    if let Some(stated) = &scale.recode.stated {
        let mut seen = HashSet::new();
        for &n in &stated.numbers {
            if !seen.insert(n) {
                let next = n + 1;
                let likely = (flagged_missing.contains(&next) && !stated.numbers.contains(&next))
                    .then_some(next);
                let hint = likely
                    .map(|m| format!(" Item {m} is not on the list and is worded like the recoded items, so the second {n} was likely meant as {m}."))
                    .unwrap_or_default();
                out.push(finding(
                    format!("listRepeat:recode:{n}"),
                    CheckKind::ListRepeat,
                    Severity::Warning,
                    format!(
                        "The recode list names item {n} twice (\"{}\").{hint}",
                        stated.text
                    ),
                    vec![n],
                    vec![Fix::RecodeRepeat {
                        number: n,
                        with: likely,
                    }],
                ));
            }
        }
    }
    for s in &scale.subscales {
        if let ItemSelector::List { numbers } = &s.items {
            let mut seen = HashSet::new();
            for &n in numbers {
                if !seen.insert(n) {
                    out.push(finding(
                        format!("listRepeat:{}:{n}", s.name),
                        CheckKind::ListRepeat,
                        Severity::Warning,
                        format!("The {} list names item {n} twice.", s.name),
                        vec![n],
                        vec![],
                    ));
                }
            }
        }
    }
}

fn check_duplicates(scale: &Scale, out: &mut Vec<Finding>) {
    let filled: Vec<&Item> = scale
        .items
        .iter()
        .filter(|i| !item_text(i).trim().is_empty())
        .collect();
    for (k, a) in filled.iter().enumerate() {
        for b in &filled[k + 1..] {
            let m = wording_match(&item_text(a), &item_text(b));
            if m >= 1.0 {
                out.push(finding(
                    format!("duplicate:{}:{}", a.number, b.number),
                    CheckKind::DuplicateItem,
                    Severity::Warning,
                    format!("Item {} repeats item {} word for word.", b.number, a.number),
                    vec![a.number, b.number],
                    vec![],
                ));
            } else if m >= 0.9 {
                out.push(finding(
                    format!("nearDuplicate:{}:{}", a.number, b.number),
                    CheckKind::DuplicateItem,
                    Severity::Note,
                    format!("Items {} and {} are nearly the same.", a.number, b.number),
                    vec![a.number, b.number],
                    vec![],
                ));
            }
        }
    }
}

fn check_subscales(scale: &Scale, out: &mut Vec<Finding>) {
    if scale.subscales.is_empty() {
        return;
    }
    let count = scale.items.len() as u32;
    let mut membership: HashMap<u32, Vec<&str>> = HashMap::new();
    for s in &scale.subscales {
        for n in s.items.resolve(count) {
            if n == 0 || n > count {
                out.push(finding(
                    format!("subscaleRange:{}:{n}", s.name),
                    CheckKind::SubscaleCoverage,
                    Severity::Warning,
                    format!(
                        "The {} list names item {n}, but the scale has {count} items.",
                        s.name
                    ),
                    vec![n],
                    vec![],
                ));
                continue;
            }
            membership.entry(n).or_default().push(&s.name);
        }
    }
    let none: Vec<u32> = scale
        .items
        .iter()
        .map(|i| i.number)
        .filter(|n| !membership.contains_key(n))
        .collect();
    if !none.is_empty() {
        out.push(finding(
            "subscaleNone".into(),
            CheckKind::SubscaleCoverage,
            Severity::Warning,
            format!(
                "{} {} in no subscale.",
                items_word(&none),
                list(&none) + if none.len() == 1 { " is" } else { " are" }
            ),
            none,
            vec![],
        ));
    }
    let mut many: Vec<(u32, Vec<&str>)> = membership
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .collect();
    many.sort();
    for (n, subs) in many {
        out.push(finding(
            format!("subscaleMany:{n}"),
            CheckKind::SubscaleCoverage,
            Severity::Warning,
            format!(
                "Item {n} is in more than one subscale: {}.",
                subs.join(", ")
            ),
            vec![n],
            vec![],
        ));
    }
    for s in &scale.subscales {
        for other in &s.other_names {
            out.push(finding(
                format!("subscaleNames:{}:{other}", s.name),
                CheckKind::SubscaleNames,
                Severity::Warning,
                format!(
                    "A subscale is called {} in one place and {other} in another.",
                    s.name
                ),
                vec![],
                vec![],
            ));
        }
    }
}

fn check_counts(scale: &Scale, out: &mut Vec<Finding>) {
    let n = scale.items.len() as u32;
    if let Some(c) = title_count(&scale.title) {
        if c != n {
            out.push(finding(
                "itemCount:title".into(),
                CheckKind::ItemCount,
                Severity::Warning,
                format!("The title says {c} items, but the scale has {n}."),
                vec![],
                vec![Fix::SyncTitleCount],
            ));
        }
    }
    let file = scale
        .provenance
        .first()
        .map(|p| p.file.clone())
        .unwrap_or_default();
    let stem = std::path::Path::new(&file)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Some(c) = title_count(&stem) {
        if c != n {
            out.push(finding(
                "itemCount:file".into(),
                CheckKind::ItemCount,
                Severity::Note,
                format!("The file name ({file}) says {c} items, but the scale has {n}."),
                vec![],
                vec![],
            ));
        }
    }
}

fn fmt(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{}", x as i64)
    } else {
        format!("{x:.2}")
    }
}

fn check_range(scale: &Scale, out: &mut Vec<Finding>) {
    let computed = computed_range(scale);
    match (scale.score_range.stated, computed) {
        (Some(s), Some(c)) if (s.0 - c.0).abs() > 1e-9 || (s.1 - c.1).abs() > 1e-9 => {
            out.push(finding(
                "scoreRange:differs".into(),
                CheckKind::ScoreRange,
                Severity::Warning,
                format!(
                    "The stated range is {} to {}, but {} items scored {} to {} give {} to {}.",
                    fmt(s.0),
                    fmt(s.1),
                    scale.items.len(),
                    scale.response.lowest(),
                    scale.response.highest(),
                    fmt(c.0),
                    fmt(c.1)
                ),
                vec![],
                vec![Fix::UseComputedRange],
            ))
        }
        (None, Some(c)) if filled_items(scale) > 0 => out.push(finding(
            "scoreRange:missing".into(),
            CheckKind::ScoreRange,
            Severity::Note,
            format!(
                "No score range is stated; the scores run from {} to {}.",
                fmt(c.0),
                fmt(c.1)
            ),
            vec![],
            vec![Fix::UseComputedRange],
        )),
        _ => {}
    }
}

fn check_instructions(scale: &Scale, out: &mut Vec<Finding>) {
    for f in fit_findings(&scale.instructions.text, scale.format, &scale.response) {
        let fixes = if scale.instructions.origin == InstructionsOrigin::House {
            vec![]
        } else {
            vec![Fix::UseHouseInstructions]
        };
        out.push(finding(
            format!("instructions:{}", f.words.to_lowercase()),
            CheckKind::InstructionsFormat,
            Severity::Warning,
            format!("\"{}\": {}", f.words, f.message),
            vec![],
            fixes,
        ));
    }
    if is_forced_choice(&scale.response) {
        out.push(finding(
            "forcedChoice".into(),
            CheckKind::ForcedChoice,
            Severity::Note,
            format!(
                "With {} points there is no middle answer: the scale is forced choice.",
                scale.response.points
            ),
            vec![],
            vec![],
        ));
    }
}

fn check_leftover_source(scale: &Scale, out: &mut Vec<Finding>) {
    if let Some(src) = &scale.source {
        if filled_items(scale) == 0 {
            out.push(finding(
                "leftoverSource".into(),
                CheckKind::LeftoverSource,
                Severity::Warning,
                format!(
                    "This blank scale still carries a source from an earlier use: {}",
                    src.apa
                ),
                vec![],
                vec![Fix::ClearSource],
            ));
        }
    }
}

fn check_score_names(scale: &Scale, out: &mut Vec<Finding>) {
    if !scale.scoring.has_total {
        return;
    }
    if let Some(name) = &scale.scoring.total_name {
        for other in &scale.scoring.other_total_names {
            out.push(finding(
                format!("scoreNames:{}", other.to_lowercase()),
                CheckKind::ScoreNames,
                Severity::Warning,
                format!("The total is called the {name} in one place and the {other} in another."),
                vec![],
                vec![Fix::UseTotalName { name: name.clone() }],
            ));
        }
    }
}

/// Verbs whose base form ends in "ed", so "did not need" is not a slip.
const ED_BASE: &[&str] = &[
    "need", "feed", "seed", "speed", "bleed", "heed", "proceed", "succeed", "exceed", "shed",
    "wed", "breed", "embed", "shred", "red",
];

fn slips_in(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let t = text.trim();
    if t.is_empty() {
        return out;
    }
    // A stray mark at the start, or two marks with only space between.
    if t.starts_with(['.', ',', ';', ':'])
        || t.contains(". .")
        || t.contains(",,")
        || t.contains(" .") && t.contains(". .")
    {
        out.push((
            t.chars().take(12).collect(),
            "starts with or contains a stray mark".into(),
        ));
    }
    // An item number typed into the text.
    if let Some(first) = t.split_whitespace().next() {
        if first.ends_with('.')
            && first[..first.len() - 1].chars().all(|c| c.is_ascii_digit())
            && first.len() > 1
        {
            out.push((first.to_string(), "starts with a stray item number".into()));
        }
    }
    let ws: Vec<String> = t
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                .to_lowercase()
        })
        .collect();
    for k in 0..ws.len() {
        // "did not used", "does not cares".
        let aux = ws[k].as_str();
        if matches!(
            aux,
            "did"
                | "didn't"
                | "does"
                | "doesn't"
                | "do"
                | "don't"
                | "to"
                | "will"
                | "would"
                | "can"
                | "could"
                | "should"
                | "might"
                | "must"
        ) {
            let mut j = k + 1;
            if ws.get(j).is_some_and(|w| w == "not") {
                j += 1;
            }
            if let Some(v) = ws.get(j) {
                let ed_slip = v.len() > 3
                    && v.ends_with("ed")
                    && !ED_BASE.contains(&v.as_str())
                    && aux != "to";
                let s_slip = matches!(aux, "does" | "doesn't")
                    && v.len() > 3
                    && v.ends_with('s')
                    && !v.ends_with("ss")
                    && !v.ends_with("us")
                    && !v.ends_with("is");
                if ed_slip || s_slip {
                    let phrase = ws[k..=j].join(" ");
                    out.push((phrase.clone(), format!("\"{phrase}\" reads as a slip")));
                }
            }
        }
        // A word typed twice ("the the").
        if k + 1 < ws.len()
            && !ws[k].is_empty()
            && ws[k] == ws[k + 1]
            && !matches!(ws[k].as_str(), "had" | "that" | "very")
        {
            out.push((
                format!("{} {}", ws[k], ws[k]),
                format!("\"{}\" is typed twice", ws[k]),
            ));
        }
    }
    out
}

fn check_typing(scale: &Scale, out: &mut Vec<Finding>) {
    let mut places: Vec<(String, String, Vec<u32>)> = Vec::new(); // (where, text, items)
    places.push(("the title".into(), scale.title.clone(), vec![]));
    for it in &scale.items {
        places.push((
            format!("item {}", it.number),
            it.text.clone(),
            vec![it.number],
        ));
        if let Some(l) = &it.left {
            places.push((format!("item {}", it.number), l.clone(), vec![it.number]));
        }
        if let Some(r) = &it.right {
            places.push((format!("item {}", it.number), r.clone(), vec![it.number]));
        }
    }
    for s in &scale.subscales {
        places.push((
            format!("the {} subscale name", s.name),
            s.name.clone(),
            vec![],
        ));
    }
    if let Some(n) = &scale.scoring.total_name {
        places.push(("the total's name".into(), n.clone(), vec![]));
    }
    for n in &scale.scoring.other_total_names {
        places.push(("the total's other name".into(), n.clone(), vec![]));
    }
    for (k, s) in scale.scoring.stated.iter().enumerate() {
        places.push((format!("scoring line {}", k + 1), s.clone(), vec![]));
    }
    let mut seen = HashSet::new();
    for (place, text, items) in &places {
        for (what, why) in slips_in(text) {
            let key = format!("slip:{place}:{what}");
            if seen.insert(key.clone()) {
                out.push(finding(
                    key,
                    CheckKind::TypingSlip,
                    Severity::Warning,
                    format!(
                        "In {place}, {why}. It is not fixed silently; change it if it is a slip."
                    ),
                    items.clone(),
                    vec![],
                ));
            }
        }
    }
    // Spelling: a long word used once that is one or two letters away from a word used elsewhere in the scale.
    let mut vocab: HashMap<String, usize> = HashMap::new();
    for (_, text, _) in &places {
        for w in words(text) {
            if w.chars().all(char::is_alphabetic) {
                *vocab.entry(w).or_default() += 1;
            }
        }
    }
    for (place, text, items) in &places {
        for w in words(text) {
            let uses = vocab.get(&w).copied().unwrap_or(0);
            if w.chars().count() < 6 || !w.chars().all(char::is_alphabetic) || uses > 2 {
                continue;
            }
            let limit = if w.chars().count() >= 9 { 2 } else { 1 };
            let near = vocab
                .iter()
                .filter(|(o, n)| *o != &w && o.chars().count() >= 6 && **n >= uses)
                .map(|(o, _)| o)
                .find(|o| {
                    let d = edit_distance(&w, o, limit);
                    if d == 0 || d > limit {
                        return false;
                    }
                    // Different endings are inflections ("communicated" and "communicator"), not slips.
                    let a: Vec<char> = w.chars().collect();
                    let b: Vec<char> = o.chars().collect();
                    // A slip keeps the first two letters ("letter" and "better" are different words).
                    if a.len() < 2 || b.len() < 2 || a[..2] != b[..2] {
                        return false;
                    }
                    let common_end = a.len().min(b.len()).saturating_sub(3);
                    a.len() == b.len()
                        && a[..common_end] != b[..common_end]
                        && a[a.len() - 2..] == b[b.len() - 2..]
                });
            if let Some(o) = near {
                let key = format!("spelling:{w}");
                if seen.insert(key.clone()) {
                    out.push(finding(
                        key,
                        CheckKind::TypingSlip,
                        Severity::Warning,
                        format!("In {place}, \"{w}\" looks like a misspelling of \"{o}\", which the scale uses elsewhere."),
                        items.clone(),
                        vec![],
                    ));
                }
            }
        }
    }
}

/// Variant structure held (specification, "Change the context"): a variant keeps its parent's item count, point
/// count, anchors, reverse items, subscales and scoring; each item maps to one original item; and no old target word
/// is left in items, title or instructions. Whether reversed items still read against the construct is the recode
/// check's work, which runs on every scale.
fn check_variant(scale: &Scale, out: &mut Vec<Finding>) {
    let Some(vo) = &scale.variant_of else { return };
    let parent = vo.parent_title.trim();
    let restore = || vec![Fix::RestoreLockedStructure];
    let warn = |key: &str, msg: String, items: Vec<u32>, fixes: Vec<Fix>| {
        finding(
            key.to_string(),
            CheckKind::VariantStructure,
            Severity::Warning,
            msg,
            items,
            fixes,
        )
    };
    if let Some(l) = &vo.locked {
        let n = scale.items.len() as u32;
        if n != l.item_count {
            out.push(warn(
                "variant:count",
                format!("The variant has {n} items and the original ({parent}) has {}. A variant keeps its parent's items one for one; for a different set of items, copy the scale as a new scale.", l.item_count),
                vec![],
                vec![],
            ));
        }
        if scale.response.points != l.response.points {
            out.push(warn(
                "variant:points",
                format!(
                    "The variant has {} points and the original has {}.",
                    scale.response.points, l.response.points
                ),
                vec![],
                restore(),
            ));
        } else if scale.response.labels != l.response.labels
            || scale.response.values != l.response.values
        {
            out.push(warn(
                "variant:anchors",
                "The variant's anchors or point values differ from the original's.".into(),
                vec![],
                restore(),
            ));
        }
        let original_of = |num: u32| {
            vo.item_map
                .iter()
                .find(|e| e.number == num)
                .map(|e| e.original_number)
                .unwrap_or(num)
        };
        let now: BTreeSet<u32> = scale
            .items
            .iter()
            .filter(|i| i.reverse.is_reversed())
            .map(|i| original_of(i.number))
            .collect();
        let was: BTreeSet<u32> = l.reversed.iter().copied().collect();
        if now != was {
            let differ: Vec<u32> = scale
                .items
                .iter()
                .filter(|i| {
                    now.contains(&original_of(i.number)) != was.contains(&original_of(i.number))
                })
                .map(|i| i.number)
                .collect();
            out.push(warn(
                "variant:reverse",
                format!(
                    "The reverse items differ from the original's: {} {}.",
                    items_word(&differ),
                    list(&differ)
                ),
                differ,
                restore(),
            ));
        }
        let shape = |subs: &[Subscale], count: u32| -> Vec<(String, Vec<u32>)> {
            subs.iter()
                .map(|s| (s.name.trim().to_lowercase(), s.items.resolve(count)))
                .collect()
        };
        if shape(&scale.subscales, n) != shape(&l.subscales, l.item_count) {
            out.push(warn(
                "variant:subscales",
                "The variant's subscales differ from the original's.".into(),
                vec![],
                restore(),
            ));
        }
        if scale.recode != l.recode || scale.scoring != l.scoring {
            out.push(warn(
                "variant:scoring",
                "The variant's recode or scoring rule differs from the original's.".into(),
                vec![],
                restore(),
            ));
        }
    }
    // Each item maps to one original item.
    let mut seen = HashSet::new();
    let unmapped: Vec<u32> = scale
        .items
        .iter()
        .filter(|i| {
            vo.item_map
                .iter()
                .find(|e| e.number == i.number)
                .is_none_or(|e| !seen.insert(e.original_number))
        })
        .map(|i| i.number)
        .collect();
    if !unmapped.is_empty() {
        out.push(warn(
            "variant:map",
            format!(
                "{} {} {} not map to one item of the original.",
                if unmapped.len() == 1 { "Item" } else { "Items" },
                list(&unmapped),
                if unmapped.len() == 1 { "does" } else { "do" }
            ),
            unmapped,
            vec![],
        ));
    }
    // No old target word left.
    let old = vo.old_target.trim();
    for it in &scale.items {
        let text = [
            Some(it.text.as_str()),
            it.left.as_deref(),
            it.right.as_deref(),
            it.lead_in.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
        let left = crate::variant::targets_left(&text, vo);
        if left.is_empty() {
            continue;
        }
        let flag = vo
            .item_map
            .iter()
            .find(|e| e.number == it.number)
            .and_then(|e| e.flagged.clone());
        let words = left
            .iter()
            .map(|w| format!("\"{w}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let (severity, msg) = match flag {
            Some(r) => (
                Severity::Note,
                format!(
                    "Item {} still names {words}: it was flagged as not rewordable ({r}).",
                    it.number
                ),
            ),
            None => (
                Severity::Warning,
                format!(
                    "Item {} still names {words}, the old target \"{old}\".",
                    it.number
                ),
            ),
        };
        out.push(finding(
            format!("variant:target:{}", it.number),
            CheckKind::VariantStructure,
            severity,
            msg,
            vec![it.number],
            vec![],
        ));
    }
    for (key, what, text) in [
        ("variant:target:title", "The title", scale.title.as_str()),
        (
            "variant:target:instructions",
            "The instructions",
            scale.instructions.text.as_str(),
        ),
    ] {
        let left = crate::variant::targets_left(text, vo);
        if !left.is_empty() {
            let words = left
                .iter()
                .map(|w| format!("\"{w}\""))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(warn(
                key,
                format!("{what} still names {words}, the old target \"{old}\"."),
                vec![],
                vec![],
            ));
        }
    }
}

#[cfg(test)]
mod variant_tests {
    use super::*;
    use crate::scale::new_blank;
    use crate::variant::{draft, Request};

    fn parent() -> Scale {
        let mut s = new_blank(Format::LikertBlankLine);
        s.title = "Communication Apprehension with Physicians Scale".into();
        s.items = vec![
            Item::new(1, "I am calm when I talk with my physician."),
            Item::new(2, "Talking with a physician makes me tense."),
            Item::new(3, "I look forward to seeing a physician."),
        ];
        s.items[1].reverse = ReverseFlag {
            state: ReverseState::Confirmed,
            reason: None,
        };
        s
    }

    fn variant() -> Scale {
        let r = Request {
            old_target: "physician".into(),
            new_target: "AI".into(),
            ..Default::default()
        };
        draft(&parent(), &r).unwrap().scale
    }

    fn vkeys(s: &Scale) -> Vec<String> {
        self_check(s)
            .into_iter()
            .filter(|f| f.check == CheckKind::VariantStructure)
            .map(|f| f.key)
            .collect()
    }

    #[test]
    fn a_fresh_variant_holds_its_structure() {
        assert!(vkeys(&variant()).is_empty(), "{:?}", vkeys(&variant()));
    }

    #[test]
    fn structure_changes_and_leftover_targets_are_found_and_put_back() {
        let mut v = variant();
        v.response = crate::formats::with_points(&v.response, 7);
        v.items[1].reverse = ReverseFlag::default();
        v.items[2].text = "I look forward to seeing a physician.".into();
        let k = vkeys(&v);
        for want in ["variant:points", "variant:reverse", "variant:target:3"] {
            assert!(k.contains(&want.to_string()), "{k:?}");
        }
        apply_fix(&mut v, &Fix::RestoreLockedStructure);
        let k = vkeys(&v);
        assert_eq!(
            k,
            vec!["variant:target:3".to_string()],
            "only the wording is left"
        );
        assert!(v.items[1].reverse.is_reversed());
        // A flagged item that keeps the old target is a note, not a warning.
        v.variant_of.as_mut().unwrap().item_map[2].flagged = Some("no counterpart".into());
        let f = self_check(&v)
            .into_iter()
            .find(|f| f.key == "variant:target:3")
            .unwrap();
        assert_eq!(f.severity, Severity::Note);
        v.items.pop();
        assert!(vkeys(&v).contains(&"variant:count".to_string()));
    }
}

/// Runs every check. Decisions already made on the scale are applied to the findings they belong to.
pub fn self_check(scale: &Scale) -> Vec<Finding> {
    let mut out = Vec::new();
    check_recode(scale, &mut out);
    check_repeats(scale, &mut out);
    check_duplicates(scale, &mut out);
    check_subscales(scale, &mut out);
    check_counts(scale, &mut out);
    check_range(scale, &mut out);
    check_instructions(scale, &mut out);
    check_leftover_source(scale, &mut out);
    check_score_names(scale, &mut out);
    check_typing(scale, &mut out);
    check_variant(scale, &mut out);
    for f in &mut out {
        if let Some(d) = scale.check_decisions.iter().rev().find(|d| d.key == f.key) {
            f.status = match d.status {
                DecisionStatus::Accepted => FindingStatus::Accepted,
                DecisionStatus::Dismissed => FindingStatus::Dismissed,
            };
            f.reason = Some(d.reason.clone()).filter(|r| !r.is_empty());
        }
        if let Some(n) = scale
            .source_notes
            .iter()
            .find(|n| n.key == f.key && n.handling == SourceNoteHandling::Kept)
        {
            f.status = FindingStatus::Published;
            f.reason = Some(note_text(n));
        }
    }
    out
}

/// One line for a source note, as it prints under the scale in the researcher copy and shows on the confirm screen:
/// "As published (page 7): the original lists item 4 as reverse scored. Kept as printed."
pub fn note_text(n: &SourceNote) -> String {
    let page = n
        .page
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            if p.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                format!(" (page {p})")
            } else {
                format!(" ({p})")
            }
        })
        .unwrap_or_default();
    let original = n.original.trim().trim_end_matches('.');
    match n.handling {
        SourceNoteHandling::Kept => format!("As published{page}: {original}. Kept as printed."),
        SourceNoteHandling::Corrected => {
            let change = n
                .change
                .as_deref()
                .unwrap_or("it differs")
                .trim()
                .trim_end_matches('.');
            format!(
                "Corrected from the source{page}: {original}. In this copy, {}.",
                lower_first(change)
            )
        }
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn finding_for(scale: &Scale, key: &str) -> Result<Finding, String> {
    self_check(scale)
        .into_iter()
        .find(|f| f.key == key)
        .ok_or_else(|| "That finding is no longer reported for this scale.".to_string())
}

fn new_note(
    f: &Finding,
    original: &str,
    page: Option<&str>,
    handling: SourceNoteHandling,
    now: &str,
) -> Result<SourceNote, String> {
    let original = original.trim();
    if original.is_empty() {
        return Err(
            "Say what the original publication prints; the note is kept with the scale.".into(),
        );
    }
    Ok(SourceNote {
        key: f.key.clone(),
        subject: f.check_label.clone(),
        items: f.items.clone(),
        original: original.to_string(),
        page: page
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string),
        handling,
        change: None,
        noted_at: now.to_string(),
    })
}

/// Marks a finding as published: the scale stays exactly as the source prints it, and the note shows beside the item
/// and under the scale in the researcher copy. Replaces any earlier decision or note on the same finding.
pub fn keep_as_published(
    scale: &mut Scale,
    key: &str,
    original: &str,
    page: Option<&str>,
    now: &str,
) -> Result<(), String> {
    let f = finding_for(scale, key)?;
    let note = new_note(&f, original, page, SourceNoteHandling::Kept, now)?;
    scale.check_decisions.retain(|d| d.key != key);
    scale.source_notes.retain(|n| n.key != key);
    scale.source_notes.push(note);
    Ok(())
}

/// Corrects a finding that sits in the original publication: the fix is made, and the correction is recorded as a
/// change from the source with the same note, so the scoring key shows what the original printed and what this copy
/// scores.
pub fn correct_from_source(
    scale: &mut Scale,
    key: &str,
    fix: &Fix,
    original: &str,
    page: Option<&str>,
    now: &str,
) -> Result<(), String> {
    let f = finding_for(scale, key)?;
    if !f.fixes.contains(fix) {
        return Err("That fix does not belong to this finding.".into());
    }
    let mut note = new_note(&f, original, page, SourceNoteHandling::Corrected, now)?;
    note.change = Some(fix.done_label());
    apply_fix(scale, fix);
    scale.check_decisions.retain(|d| d.key != key);
    scale.source_notes.retain(|n| n.key != key);
    scale.source_notes.push(note);
    Ok(())
}

/// Removes the source note on a finding. A note kept as published reopens its finding; removing the record of a
/// correction does not undo the correction.
pub fn remove_source_note(scale: &mut Scale, key: &str) {
    scale.source_notes.retain(|n| n.key != key);
}

/// Findings still open: not accepted, dismissed or kept as published. These print in the researcher copy (findings kept
/// as published print there too, as source notes).
pub fn open_findings(scale: &Scale) -> Vec<Finding> {
    self_check(scale)
        .into_iter()
        .filter(|f| f.status == FindingStatus::Open)
        .collect()
}

/// Records a decision on a finding. A dismissal needs a reason; an acceptance may have one.
pub fn decide(
    scale: &mut Scale,
    key: &str,
    status: Option<DecisionStatus>,
    reason: &str,
    now: &str,
) -> Result<(), String> {
    scale.check_decisions.retain(|d| d.key != key);
    // A new decision, or reopening, replaces a "kept as published" note; the record of a correction stays.
    scale
        .source_notes
        .retain(|n| !(n.key == key && n.handling == SourceNoteHandling::Kept));
    let Some(status) = status else { return Ok(()) };
    if status == DecisionStatus::Dismissed && reason.trim().is_empty() {
        return Err(
            "Say why the finding does not apply; the reason is kept with the scale.".into(),
        );
    }
    scale.check_decisions.push(CheckDecision {
        key: key.to_string(),
        status,
        reason: reason.trim().to_string(),
        decided_at: now.to_string(),
    });
    Ok(())
}

/// Makes the change a fix describes.
pub fn apply_fix(scale: &mut Scale, fix: &Fix) {
    match fix {
        Fix::SetReversed { item, reversed } => {
            if let Some(it) = scale.items.iter_mut().find(|i| i.number == *item) {
                it.reverse = if *reversed {
                    ReverseFlag {
                        state: ReverseState::Confirmed,
                        reason: Some("set from a self-check finding".into()),
                    }
                } else {
                    ReverseFlag {
                        state: ReverseState::NotReversed,
                        reason: None,
                    }
                };
            }
        }
        Fix::RecodeRepeat { number, with } => {
            if let Some(stated) = &mut scale.recode.stated {
                if let Some(pos) = stated
                    .numbers
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| *n == number)
                    .nth(1)
                    .map(|(i, _)| i)
                {
                    match with {
                        Some(w) => stated.numbers[pos] = *w,
                        None => {
                            stated.numbers.remove(pos);
                        }
                    }
                }
            }
            if let Some(w) = with {
                apply_fix(
                    scale,
                    &Fix::SetReversed {
                        item: *w,
                        reversed: true,
                    },
                );
            }
        }
        Fix::SyncTitleCount => {
            scale.count_suffix = true;
            crate::scale::sync_title_count(scale);
        }
        Fix::UseComputedRange => {
            scale.score_range.stated = computed_range(scale);
            scale.score_range.stated_text = None;
        }
        Fix::ClearSource => scale.source = None,
        Fix::UseHouseInstructions => crate::scale::use_house_instructions(scale, None),
        Fix::UseTotalName { name } => {
            scale.scoring.total_name = Some(name.clone());
            scale.scoring.other_total_names.clear();
        }
        Fix::RestoreLockedStructure => {
            let Some(vo) = scale.variant_of.clone() else {
                return;
            };
            let Some(l) = vo.locked else { return };
            scale.response = l.response;
            scale.subscales = l.subscales;
            scale.recode = l.recode;
            scale.scoring = l.scoring;
            for it in &mut scale.items {
                let original = vo
                    .item_map
                    .iter()
                    .find(|e| e.number == it.number)
                    .map(|e| e.original_number)
                    .unwrap_or(it.number);
                let want = l.reversed.contains(&original);
                if want != it.reverse.is_reversed() {
                    it.reverse = if want {
                        ReverseFlag {
                            state: ReverseState::Confirmed,
                            reason: Some(format!("as in {}", vo.parent_title)),
                        }
                    } else {
                        ReverseFlag::default()
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::new_blank;

    fn scale(texts: &[&str], recoded: &[u32]) -> Scale {
        let mut s = new_blank(Format::LikertBlankLine);
        s.items = texts
            .iter()
            .enumerate()
            .map(|(k, t)| {
                let mut it = Item::new(k as u32 + 1, *t);
                if recoded.contains(&(k as u32 + 1)) {
                    it.reverse = ReverseFlag {
                        state: ReverseState::Confirmed,
                        reason: None,
                    };
                }
                it
            })
            .collect();
        s
    }

    fn keys(s: &Scale) -> Vec<String> {
        self_check(s).into_iter().map(|f| f.key).collect()
    }

    #[test]
    fn a_recoded_item_that_reads_with_the_construct_is_found() {
        let s = scale(
            &[
                "The lesson was explained clearly to me.",
                "The teacher completely explained the lesson.",
                "The lesson involved was not explained to me.",
                "The teacher completely explained the lesson involved.",
                "The teacher did not explain the lesson clearly.",
                "The lesson involved was explained to me.",
            ],
            &[3, 4, 5],
        );
        let k = keys(&s);
        assert!(k.contains(&"recodeWording:4".to_string()), "{k:?}");
        assert!(
            !k.iter()
                .any(|x| x == "recodeWording:2" || x == "recodeWording:3"),
            "{k:?}"
        );
    }

    #[test]
    fn guards_a_negated_item_with_the_construct_and_a_reversed_item_without_negation() {
        // Like Ethnocentrism item 5: "don't respect" is negated but scores with the construct.
        let a = scale(
            &[
                "My town is generally right about things.",
                "I respect differences between my town and other towns.",
                "Other towns should try to be like my town.",
                "People from elsewhere make me anxious.",
                "I don't respect the habits of other towns.",
                "Other towns should model themselves after my town.",
                "I do not feel the need to respect other towns.",
                "I respect towns that do things differently.",
            ],
            &[2, 8],
        );
        assert!(
            !keys(&a).iter().any(|k| k.starts_with("recodeWording")),
            "{:?}",
            keys(&a)
        );
        // Like Crisis Knowledge item 6: "is limited" has no negation word but is correctly reversed.
        let b = scale(
            &[
                "I know the details of the storm.",
                "I do not feel knowledgeable about the storm.",
                "The details of the storm are very clear to me.",
                "I do not know enough about the storm.",
                "My knowledge of the storm is limited.",
                "I completely understand the details of the storm.",
            ],
            &[2, 4, 5],
        );
        assert!(
            !keys(&b).iter().any(|k| k.starts_with("recodeWording")),
            "{:?}",
            keys(&b)
        );
    }

    #[test]
    fn missing_recode_list() {
        let s = scale(
            &[
                "I am careful to avoid shouting at people.",
                "When I get upset, I shout.",
                "When I am annoyed, I yell at the television.",
                "I would never shout to win an argument.",
                "I raise my voice to get my way.",
                "I yell when I lose.",
            ],
            &[],
        );
        let f: Vec<Finding> = self_check(&s)
            .into_iter()
            .filter(|f| f.check == CheckKind::RecodeMissing)
            .collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].items, vec![1, 4]);
    }

    #[test]
    fn slips_are_found() {
        assert!(!slips_in(". . I can roll a kayak.").is_empty());
        assert!(!slips_in("The guide did not used a map.").is_empty());
        assert!(slips_in("The guide did not need a map.").is_empty());
        assert!(!slips_in("5. Extrovert").is_empty());
        let mut s = scale(&["I feel confident in a kayak."], &[]);
        s.title = "Kayak Confidence Scale".into();
        s.scoring.total_name = Some("composite kayak confidance score".into());
        assert!(
            keys(&s).contains(&"spelling:confidance".to_string()),
            "{:?}",
            keys(&s)
        );
    }

    fn communicated() -> Scale {
        scale(
            &[
                "The lesson was explained clearly to me.",
                "The teacher completely explained the lesson.",
                "The lesson involved was not explained to me.",
                "The teacher completely explained the lesson involved.",
                "The teacher did not explain the lesson clearly.",
                "The lesson involved was explained to me.",
            ],
            &[3, 4, 5],
        )
    }

    #[test]
    fn a_finding_kept_as_published_stays_visible_with_its_note() {
        // Like Crisis Communicated item 4: a researcher replicating the study keeps the published recode.
        let mut s = communicated();
        let key = "recodeWording:4";
        assert!(
            keep_as_published(&mut s, key, " ", None, "t").is_err(),
            "a note needs the original's wording"
        );
        keep_as_published(
            &mut s,
            key,
            "The original lists item 4 as reverse scored.",
            Some("7"),
            "t",
        )
        .unwrap();
        assert!(
            s.items[3].reverse.state == ReverseState::Confirmed,
            "the scale is unchanged"
        );
        let f = self_check(&s).into_iter().find(|f| f.key == key).unwrap();
        assert_eq!(f.status, FindingStatus::Published);
        assert_eq!(
            f.reason.as_deref(),
            Some("As published (page 7): The original lists item 4 as reverse scored. Kept as printed.")
        );
        assert!(open_findings(&s).iter().all(|f| f.key != key));
        assert_eq!(s.source_notes[0].items, vec![4]);
        // Reopening removes the note and the finding is open again.
        decide(&mut s, key, None, "", "t").unwrap();
        assert!(s.source_notes.is_empty());
        assert!(open_findings(&s).iter().any(|f| f.key == key));
    }

    #[test]
    fn a_correction_from_the_source_is_recorded_with_its_note() {
        // A researcher building a new study corrects it; the key says what the original printed and what this copy scores.
        let mut s = communicated();
        let key = "recodeWording:4";
        let fix = self_check(&s)
            .into_iter()
            .find(|f| f.key == key)
            .unwrap()
            .fixes[0]
            .clone();
        let other = Fix::ClearSource;
        assert!(correct_from_source(&mut s, key, &other, "x", None, "t").is_err());
        correct_from_source(
            &mut s,
            key,
            &fix,
            "the original lists item 4 as reverse scored",
            Some("p. 7"),
            "t",
        )
        .unwrap();
        assert_eq!(s.items[3].reverse.state, ReverseState::NotReversed);
        assert!(
            self_check(&s).iter().all(|f| f.key != key),
            "the finding is fixed"
        );
        assert_eq!(
            note_text(&s.source_notes[0]),
            "Corrected from the source (p. 7): the original lists item 4 as reverse scored. In this copy, item 4 is scored as not reversed."
        );
        // Deciding something else on the same key later keeps the record of the correction.
        decide(&mut s, key, None, "", "t").unwrap();
        assert_eq!(s.source_notes.len(), 1);
    }

    #[test]
    fn decisions_stick_and_dismissals_need_a_reason() {
        let mut s = scale(&["a b c", "a b c"], &[]);
        let key = "duplicate:1:2";
        assert!(keys(&s).contains(&key.to_string()));
        assert!(decide(&mut s, key, Some(DecisionStatus::Dismissed), " ", "t").is_err());
        decide(
            &mut s,
            key,
            Some(DecisionStatus::Dismissed),
            "Asked twice on purpose.",
            "t",
        )
        .unwrap();
        let f = self_check(&s).into_iter().find(|f| f.key == key).unwrap();
        assert_eq!(f.status, FindingStatus::Dismissed);
        assert!(open_findings(&s).iter().all(|f| f.key != key));
        decide(&mut s, key, None, "", "t").unwrap();
        assert_eq!(
            self_check(&s)
                .into_iter()
                .find(|f| f.key == key)
                .unwrap()
                .status,
            FindingStatus::Open
        );
    }

    #[test]
    fn fixes_change_the_scale() {
        let mut s = scale(&["a", "b"], &[]);
        s.recode.stated = Some(StatedList {
            numbers: vec![1, 1],
            text: "Recode 1, 1".into(),
        });
        apply_fix(
            &mut s,
            &Fix::RecodeRepeat {
                number: 1,
                with: Some(2),
            },
        );
        assert_eq!(s.recode.stated.as_ref().unwrap().numbers, vec![1, 2]);
        assert!(s.items[1].reverse.is_reversed());
    }
}
