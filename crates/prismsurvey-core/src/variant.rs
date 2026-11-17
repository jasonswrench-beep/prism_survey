//! Change the context (specification, "Change the context"): a linked variant of a scale reworded for a new target,
//! such as communication apprehension with physicians turned into communication apprehension with AIs. The structure
//! stays locked; only the target words and the wording around them change.
//!
//! This is the by-hand mode, which needs no model: the target words found in items, title and instructions are
//! replaced, the researcher accepts, edits or flags each item, and the variant structure check runs before saving.

use crate::model::*;
use crate::scale::{new_id, reversed_numbers};
use serde::{Deserialize, Serialize};

/// The target pairs of a variant, longest first so a phrase ("my doctor") wins over a word inside it ("doctor").
pub fn pairs(old_target: &str, new_target: &str, extra: &[Replacement]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> =
        std::iter::once((old_target.to_string(), new_target.to_string()))
            .chain(extra.iter().map(|r| (r.from.clone(), r.to.clone())))
            .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
            .filter(|(a, _)| !a.is_empty())
            .collect();
    out.sort_by_key(|p| std::cmp::Reverse(p.0.chars().count()));
    out.dedup_by(|a, b| a.0.eq_ignore_ascii_case(&b.0));
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

fn plural(w: &str) -> String {
    let lower = w.to_lowercase();
    if ["s", "x", "z", "ch", "sh"]
        .iter()
        .any(|e| lower.ends_with(e))
    {
        format!("{w}es")
    } else if lower.ends_with('y')
        && !lower
            .chars()
            .rev()
            .nth(1)
            .is_some_and(|c| "aeiou".contains(c))
        && w.chars().any(|c| c.is_lowercase())
    {
        format!("{}ies", &w[..w.len() - 1])
    } else {
        format!("{w}s")
    }
}

fn starts_with_vowel_letter(s: &str) -> bool {
    s.chars().next().is_some_and(|c| "aeiouAEIOU".contains(c))
}

/// Matches `needle` at byte `at` of `hay`, ignoring case, and returns the matched length in bytes.
fn match_at(hay: &str, at: usize, needle: &str) -> Option<usize> {
    let mut h = hay[at..].char_indices();
    let mut len = 0;
    for nc in needle.chars() {
        let (i, hc) = h.next()?;
        if !hc.to_lowercase().eq(nc.to_lowercase()) {
            return None;
        }
        len = i + hc.len_utf8();
    }
    Some(len)
}

/// Replaces the target words in `text`. Matching ignores case and keeps to whole words; a plural ("physicians"), a
/// possessive ("physician's") and a capital at the start are carried over, and "a" or "an" before the new target is
/// set to fit it. Returns the new text and whether anything was replaced.
pub fn replace_targets(text: &str, pairs: &[(String, String)]) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut changed = false;
    while i < text.len() {
        let prev_ok = text[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word_char(c));
        let mut done = false;
        if prev_ok {
            for (from, to) in pairs {
                let Some(len) = match_at(text, i, from) else {
                    continue;
                };
                let rest = &text[i + len..];
                // A suffix the match may carry: possessive, plural, plural possessive.
                let suffixes = ["’s", "'s", "s'", "s’", "es", "s", ""];
                let Some(suffix) = suffixes.iter().find(|sfx| {
                    rest.starts_with(**sfx)
                        && rest[sfx.len()..]
                            .chars()
                            .next()
                            .is_none_or(|c| !is_word_char(c))
                }) else {
                    continue;
                };
                let matched = &text[i..i + len];
                let mut new = to.clone();
                let first_upper = matched.chars().next().is_some_and(char::is_uppercase);
                if first_upper && new.chars().next().is_some_and(char::is_lowercase) {
                    let mut c = new.chars();
                    new = c
                        .next()
                        .map(|f| f.to_uppercase().collect::<String>())
                        .unwrap_or_default()
                        + c.as_str();
                }
                let new = match *suffix {
                    "s" | "es" => plural(&new),
                    "s'" | "s’" => format!("{}'", plural(&new)),
                    "'s" | "’s" => {
                        if new.ends_with('s') {
                            format!("{new}'")
                        } else {
                            format!("{new}{suffix}")
                        }
                    }
                    _ => new,
                };
                fix_article(&mut out, &new);
                out.push_str(&new);
                i += len + suffix.len();
                changed = true;
                done = true;
                break;
            }
        }
        if !done {
            let c = text[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    (out, changed)
}

/// Sets "a" or "an" just before a replacement to fit the new word.
fn fix_article(out: &mut String, next: &str) {
    let trimmed = out.trim_end_matches(' ');
    if trimmed.len() == out.len() {
        return;
    }
    let spaces = out.len() - trimmed.len();
    let word_start = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| !is_word_char(*c))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    let word = &trimmed[word_start..];
    let want_an = starts_with_vowel_letter(next);
    let replacement = match word {
        "a" | "an" => Some(if want_an { "an" } else { "a" }),
        "A" | "An" => Some(if want_an { "An" } else { "A" }),
        _ => None,
    };
    if let Some(r) = replacement {
        let tail = " ".repeat(spaces);
        out.truncate(word_start);
        out.push_str(r);
        out.push_str(&tail);
    }
}

/// The target words of a variant that are still in `text`, as written there.
pub fn targets_left(text: &str, v: &VariantOf) -> Vec<String> {
    let from: Vec<(String, String)> = pairs(&v.old_target, &v.new_target, &v.replacements)
        .into_iter()
        // A target the new wording itself contains ("doctor" to "AI doctor") is not a leftover.
        .filter(|(f, t)| !t.to_lowercase().contains(&f.to_lowercase()))
        .map(|(f, _)| (f.clone(), f))
        .collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let prev_ok = text[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word_char(c));
        if prev_ok {
            for (f, _) in &from {
                if let Some(len) = match_at(text, i, f) {
                    let rest = &text[i + len..];
                    let ok = ["’s", "'s", "s'", "es", "s", ""].iter().any(|sfx| {
                        rest.starts_with(sfx)
                            && rest[sfx.len()..]
                                .chars()
                                .next()
                                .is_none_or(|c| !is_word_char(c))
                    });
                    if ok {
                        let w = text[i..i + len].to_string();
                        if !found.contains(&w) {
                            found.push(w);
                        }
                    }
                }
            }
        }
        i += text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
    }
    found
}

pub fn locked_structure(s: &Scale) -> LockedStructure {
    LockedStructure {
        item_count: s.items.len() as u32,
        response: s.response.clone(),
        reversed: reversed_numbers(s),
        subscales: s.subscales.clone(),
        recode: s.recode.clone(),
        scoring: s.scoring.clone(),
    }
}

/// One item on the side-by-side screen: the original beside the proposed wording.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftRow {
    pub number: u32,
    pub original: String,
    pub proposed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_left: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_right: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<String>,
    /// Whether a target word was found and replaced. An item without one is for the researcher to reword or keep.
    pub found: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    /// The variant as drafted, not yet saved.
    pub scale: Scale,
    pub rows: Vec<DraftRow>,
    pub title_changed: bool,
    pub instructions_changed: bool,
}

/// What the researcher asks for when starting a variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub old_target: String,
    pub new_target: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub replacements: Vec<Replacement>,
}

/// Drafts a variant of a saved scale: the target words are replaced in items, title and instructions, and everything
/// else is kept and locked. The parent's source notes and Comm Directory link carry into the variant.
pub fn draft(parent: &Scale, req: &Request) -> Result<Draft, String> {
    let old = req.old_target.trim();
    let new = req.new_target.trim();
    if old.is_empty() || new.is_empty() {
        return Err("Name the old target and the new one.".into());
    }
    if old.eq_ignore_ascii_case(new) {
        return Err("The new target is the same as the old one.".into());
    }
    let replacements: Vec<Replacement> = req
        .replacements
        .iter()
        .filter(|r| !r.from.trim().is_empty())
        .map(|r| Replacement {
            from: r.from.trim().to_string(),
            to: r.to.trim().to_string(),
        })
        .collect();
    let ps = pairs(old, new, &replacements);
    let mut v = parent.clone();
    v.id = new_id();
    v.history.clear();
    v.check_decisions.clear();
    v.provenance.clear();
    v.unparsed.clear();
    v.development = None;

    let mut rows = Vec::new();
    for it in &mut v.items {
        let (text, a) = replace_targets(&it.text, &ps);
        let (left, b) = match &it.left {
            Some(l) => {
                let (x, c) = replace_targets(l, &ps);
                (Some(x), c)
            }
            None => (None, false),
        };
        let (right, c) = match &it.right {
            Some(r) => {
                let (x, c) = replace_targets(r, &ps);
                (Some(x), c)
            }
            None => (None, false),
        };
        rows.push(DraftRow {
            number: it.number,
            original: it.text.clone(),
            proposed: text.clone(),
            original_left: it.left.clone(),
            original_right: it.right.clone(),
            left: left.clone(),
            right: right.clone(),
            found: a || b || c,
        });
        it.text = text;
        it.left = left;
        it.right = right;
        it.source_text = None;
        if let Some(l) = &it.lead_in {
            it.lead_in = Some(replace_targets(l, &ps).0);
        }
    }

    let (title, title_changed) = replace_targets(&parent.title, &ps);
    v.title = if title_changed {
        format!("{title} (adapted)")
    } else {
        format!("{} (adapted for {new})", parent.title)
    };
    v.short_name = parent
        .short_name
        .as_ref()
        .map(|s| replace_targets(s, &ps).0);
    let (instr, instructions_changed) = replace_targets(&parent.instructions.text, &ps);
    if instructions_changed {
        v.instructions.text = instr;
        v.instructions.origin = InstructionsOrigin::Edited;
    }

    // Keeps the parent's source and names the parent as the work it is adapted from.
    let mut adapted = parent.title.trim().to_string();
    if let Some(a) = &parent.adapted_from {
        adapted = format!("{adapted}, itself adapted from {}", a.apa.trim());
    }
    v.adapted_from = Some(Citation {
        apa: adapted,
        doi: None,
        reference_manager_id: None,
    });
    v.licence.note = Some(match &parent.licence.note {
        Some(n) => format!("Follows the original scale's licence. {n}"),
        None => "Follows the original scale's licence.".to_string(),
    });

    v.variant_of = Some(VariantOf {
        parent_id: parent.id.clone(),
        parent_title: parent.title.clone(),
        parent_version: parent.history.last().map(|h| h.version).unwrap_or(0),
        old_target: old.to_string(),
        new_target: new.to_string(),
        context: req
            .context
            .as_ref()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty()),
        replacements,
        item_map: parent
            .items
            .iter()
            .map(|it| ItemMapEntry {
                number: it.number,
                original_number: it.number,
                original_text: original_wording(it),
                flagged: None,
            })
            .collect(),
        locked: Some(locked_structure(parent)),
        declined: Vec::new(),
    });
    Ok(Draft {
        scale: v,
        rows,
        title_changed,
        instructions_changed,
    })
}

/// An item's wording as one line: the text, or for a semantic differential its pair.
pub fn original_wording(it: &Item) -> String {
    match (&it.left, &it.right) {
        (Some(l), Some(r)) if !l.trim().is_empty() || !r.trim().is_empty() => {
            if it.text.trim().is_empty() {
                format!("{} / {}", l.trim(), r.trim())
            } else {
                format!("{}: {} / {}", it.text.trim(), l.trim(), r.trim())
            }
        }
        _ => it.text.clone(),
    }
}

/// The researcher's decision on one item of the side-by-side screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowDecision {
    pub number: u32,
    pub text: String,
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    /// Set when the item cannot be reworded without changing what it asks: the reason. The item keeps its original
    /// wording.
    #[serde(default)]
    pub flagged: Option<String>,
}

/// Puts the researcher's accepted, edited or flagged wording into the drafted variant.
pub fn finish(mut v: Scale, decisions: &[RowDecision]) -> Result<Scale, String> {
    let Some(vo) = v.variant_of.as_mut() else {
        return Err("This scale is not a variant.".into());
    };
    if decisions.len() != v.items.len() {
        return Err("Every item needs a decision: accept, edit, or flag it.".into());
    }
    for d in decisions {
        let Some(entry) = vo.item_map.iter_mut().find(|e| e.number == d.number) else {
            return Err(format!("Item {} is not in the item map.", d.number));
        };
        let Some(it) = v.items.iter_mut().find(|i| i.number == d.number) else {
            return Err(format!("Item {} is not in the variant.", d.number));
        };
        // A flagged item comes with the original wording, which it keeps.
        match d.flagged.as_deref().map(str::trim) {
            Some("") => {
                return Err(format!(
                    "Say why item {} cannot be reworded; the reason is kept with the variant.",
                    d.number
                ))
            }
            Some(reason) => entry.flagged = Some(reason.to_string()),
            None => entry.flagged = None,
        }
        it.text = d.text.clone();
        if it.left.is_some() {
            it.left = d.left.clone().or(it.left.take());
        }
        if it.right.is_some() {
            it.right = d.right.clone().or(it.right.take());
        }
    }
    Ok(v)
}

/// A change in the parent since the variant was made, which the researcher may carry into the variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ParentChange {
    /// The parent's wording of an item changed. `proposed` is the new wording with the target words replaced.
    ItemText {
        key: String,
        number: u32,
        was: String,
        now: String,
        proposed: String,
    },
    /// The parent's reverse flag on an item changed.
    Reverse {
        key: String,
        number: u32,
        reversed: bool,
    },
    /// The parent has a source note the variant lacks.
    SourceNote {
        key: String,
        note: SourceNote,
        text: String,
    },
    /// The parent's points, anchors, subscales or scoring changed.
    Structure { key: String, what: Vec<String> },
    /// The parent now has a different number of items; a variant cannot follow that item by item.
    ItemCount { key: String, was: u32, now: u32 },
}

impl ParentChange {
    pub fn key(&self) -> &str {
        match self {
            ParentChange::ItemText { key, .. }
            | ParentChange::Reverse { key, .. }
            | ParentChange::SourceNote { key, .. }
            | ParentChange::Structure { key, .. }
            | ParentChange::ItemCount { key, .. } => key,
        }
    }
}

/// A short, stable fingerprint of a text, for change keys (FNV-1a).
fn fingerprint(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:08x}", h as u32)
}

fn structure_differences(locked: &LockedStructure, parent: &Scale) -> Vec<String> {
    let mut what = Vec::new();
    if locked.response.points != parent.response.points {
        what.push(format!(
            "points ({} to {})",
            locked.response.points, parent.response.points
        ));
    } else if locked.response != parent.response {
        what.push("anchors".to_string());
    }
    if locked.subscales != parent.subscales {
        what.push("subscales".to_string());
    }
    if locked.recode != parent.recode {
        what.push("recode rule".to_string());
    }
    if locked.scoring != parent.scoring {
        what.push("scoring".to_string());
    }
    what
}

/// The parent's changes since the variant was made, leaving out those the researcher declined. Never applied unasked.
pub fn parent_changes(variant: &Scale, parent: &Scale) -> Vec<ParentChange> {
    let Some(vo) = &variant.variant_of else {
        return Vec::new();
    };
    if vo.parent_id != parent.id {
        return Vec::new();
    }
    let mut out = Vec::new();
    let locked = vo
        .locked
        .clone()
        .unwrap_or_else(|| locked_structure(variant));
    let now_count = parent.items.len() as u32;
    if now_count != locked.item_count {
        out.push(ParentChange::ItemCount {
            key: format!("count:{now_count}"),
            was: locked.item_count,
            now: now_count,
        });
    }
    let ps = pairs(&vo.old_target, &vo.new_target, &vo.replacements);
    for e in &vo.item_map {
        let Some(p) = parent.items.iter().find(|i| i.number == e.original_number) else {
            continue;
        };
        let now = original_wording(p);
        if now != e.original_text {
            out.push(ParentChange::ItemText {
                key: format!("text:{}:{}", e.number, fingerprint(&now)),
                number: e.number,
                was: e.original_text.clone(),
                now: now.clone(),
                proposed: replace_targets(&p.text, &ps).0,
            });
        }
        let was_rev = locked.reversed.contains(&e.original_number);
        let now_rev = p.reverse.is_reversed();
        if was_rev != now_rev {
            out.push(ParentChange::Reverse {
                key: format!("reverse:{}:{now_rev}", e.number),
                number: e.number,
                reversed: now_rev,
            });
        }
    }
    let what = structure_differences(&locked, parent);
    if !what.is_empty() {
        let fp = fingerprint(&serde_json::to_string(&locked_structure(parent)).unwrap_or_default());
        out.push(ParentChange::Structure {
            key: format!("structure:{fp}"),
            what,
        });
    }
    for n in &parent.source_notes {
        if !variant
            .source_notes
            .iter()
            .any(|m| m.key == n.key && m.original == n.original)
        {
            out.push(ParentChange::SourceNote {
                key: format!("note:{}:{}", n.key, fingerprint(&n.original)),
                note: n.clone(),
                text: crate::selfcheck::note_text(n),
            });
        }
    }
    out.retain(|c| !vo.declined.iter().any(|d| d == c.key()));
    out
}

/// Carries one change from the parent into the variant. The variant then records the parent's current version.
pub fn carry(variant: &mut Scale, parent: &Scale, change: &ParentChange) -> Result<(), String> {
    let version = parent.history.last().map(|h| h.version).unwrap_or(0);
    let fallback = locked_structure(variant);
    let Some(vo) = variant.variant_of.as_mut() else {
        return Err("This scale is not a variant.".into());
    };
    let mut locked = vo.locked.clone().unwrap_or(fallback);
    match change {
        ParentChange::ItemText {
            number,
            now,
            proposed,
            ..
        } => {
            let entry = vo
                .item_map
                .iter_mut()
                .find(|e| e.number == *number)
                .ok_or("That item is not in the item map.")?;
            entry.original_text = now.clone();
            let original_number = entry.original_number;
            let p = parent.items.iter().find(|i| i.number == original_number);
            let ps = pairs(&vo.old_target, &vo.new_target, &vo.replacements);
            if let Some(it) = variant.items.iter_mut().find(|i| i.number == *number) {
                it.text = proposed.clone();
                if let Some(p) = p {
                    if let Some(l) = &p.left {
                        it.left = Some(replace_targets(l, &ps).0);
                    }
                    if let Some(r) = &p.right {
                        it.right = Some(replace_targets(r, &ps).0);
                    }
                }
            }
        }
        ParentChange::Reverse {
            number, reversed, ..
        } => {
            let original_number = vo
                .item_map
                .iter()
                .find(|e| e.number == *number)
                .map(|e| e.original_number)
                .unwrap_or(*number);
            if let Some(it) = variant.items.iter_mut().find(|i| i.number == *number) {
                it.reverse = if *reversed {
                    ReverseFlag {
                        state: ReverseState::Confirmed,
                        reason: Some(format!("carried from {}", vo.parent_title)),
                    }
                } else {
                    ReverseFlag::default()
                };
            }
            locked.reversed.retain(|n| *n != original_number);
            if *reversed {
                locked.reversed.push(original_number);
                locked.reversed.sort_unstable();
            }
        }
        ParentChange::SourceNote { note, .. } => {
            variant.source_notes.retain(|n| n.key != note.key);
            variant.source_notes.push(note.clone());
        }
        ParentChange::Structure { .. } => {
            variant.response = parent.response.clone();
            variant.subscales = parent.subscales.clone();
            variant.recode = parent.recode.clone();
            variant.scoring = parent.scoring.clone();
            let p = locked_structure(parent);
            locked.response = p.response;
            locked.subscales = p.subscales;
            locked.recode = p.recode;
            locked.scoring = p.scoring;
        }
        ParentChange::ItemCount { now, .. } => {
            return Err(format!(
                "The original now has {now} items. A variant keeps its parent's items one for one, so make a new variant from the original instead."
            ));
        }
    }
    vo.locked = Some(locked);
    vo.parent_version = vo.parent_version.max(version);
    Ok(())
}

/// Records that the researcher chose not to carry a change, so it is not offered again.
pub fn decline(variant: &mut Scale, key: &str) {
    if let Some(vo) = variant.variant_of.as_mut() {
        if !vo.declined.iter().any(|d| d == key) {
            vo.declined.push(key.to_string());
        }
    }
}

/// The titles of the scales a variant comes from, nearest first, from the scales in a library (`(id, title, parent id)`).
pub fn chain(
    start_parent: Option<&str>,
    scales: &[(String, String, Option<String>)],
) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut next = start_parent.map(str::to_string);
    while let Some(id) = next {
        if !seen.insert(id.clone()) {
            break;
        }
        match scales.iter().find(|(i, _, _)| *i == id) {
            Some((_, title, parent)) => {
                out.push(title.clone());
                next = parent.clone();
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::new_blank;

    fn ps(pairs_: &[(&str, &str)]) -> Vec<(String, String)> {
        let (a, b) = pairs_[0];
        let extra: Vec<Replacement> = pairs_[1..]
            .iter()
            .map(|(f, t)| Replacement {
                from: f.to_string(),
                to: t.to_string(),
            })
            .collect();
        pairs(a, b, &extra)
    }

    #[test]
    fn target_words_are_replaced_with_plurals_capitals_and_articles() {
        let p = ps(&[("physician", "AI"), ("my doctor", "the AI I use")]);
        let r = |t: &str| replace_targets(t, &p).0;
        assert_eq!(
            r("I am tense talking with a physician."),
            "I am tense talking with an AI."
        );
        assert_eq!(r("Physicians make me nervous."), "AIs make me nervous.");
        assert_eq!(
            r("My physician's questions worry me."),
            "My AI's questions worry me."
        );
        assert_eq!(
            r("I avoid talking to my doctor."),
            "I avoid talking to the AI I use."
        );
        assert_eq!(r("My doctor listens."), "The AI I use listens.");
        assert_eq!(
            r("Metaphysicians argue."),
            "Metaphysicians argue.",
            "whole words only"
        );
        assert!(!replace_targets("Nothing here.", &p).1);
        let q = ps(&[("teacher", "coach")]);
        assert_eq!(
            replace_targets("An teacher and the teachers.", &q).0,
            "A coach and the coaches."
        );
    }

    fn parent() -> Scale {
        let mut s = new_blank(Format::LikertBlankLine);
        s.title = "Communication Apprehension with Physicians Scale".into();
        s.items = vec![
            Item::new(1, "I am calm when I talk with my physician."),
            Item::new(2, "Talking with a physician makes me tense."),
            Item::new(3, "I look forward to appointments."),
        ];
        s.items[1].reverse = ReverseFlag {
            state: ReverseState::Confirmed,
            reason: None,
        };
        s.source = Some(Citation {
            apa: "Author, A. (2001). A scale. Journal, 1(2), 3-4.".into(),
            doi: None,
            reference_manager_id: None,
        });
        s.licence.status = LicenceStatus::Open;
        s.directory_entry = Some("ch3-17".into());
        s.source_notes.push(SourceNote {
            key: "k".into(),
            subject: "Typing slips".into(),
            items: vec![1],
            original: "the original prints this".into(),
            page: None,
            handling: SourceNoteHandling::Kept,
            change: None,
            noted_at: "t".into(),
        });
        s.history.push(HistoryEntry {
            version: 3,
            saved_at: "t".into(),
            summary: "Saved".into(),
        });
        s
    }

    fn req() -> Request {
        Request {
            old_target: "physician".into(),
            new_target: "AI".into(),
            context: Some("a chatbot used for health questions".into()),
            replacements: vec![],
        }
    }

    #[test]
    fn a_draft_keeps_the_structure_and_links_the_parent() {
        let p = parent();
        let d = draft(&p, &req()).unwrap();
        let v = &d.scale;
        assert_ne!(v.id, p.id);
        assert_eq!(
            v.title,
            "Communication Apprehension with AIs Scale (adapted)"
        );
        assert_eq!(v.items[0].text, "I am calm when I talk with my AI.");
        assert_eq!(v.items[1].text, "Talking with an AI makes me tense.");
        assert!(d.rows[0].found && d.rows[1].found && !d.rows[2].found);
        assert!(v.items[1].reverse.is_reversed());
        assert_eq!(v.source, p.source, "the parent's source is kept");
        assert_eq!(v.adapted_from.as_ref().unwrap().apa, p.title);
        assert_eq!(
            v.licence.status,
            LicenceStatus::Open,
            "the licence follows the original"
        );
        assert_eq!(
            v.source_notes, p.source_notes,
            "source notes carry into the variant"
        );
        assert!(v.history.is_empty());
        assert_eq!(
            v.directory_entry, p.directory_entry,
            "the Comm Directory link is kept"
        );
        let vo = v.variant_of.as_ref().unwrap();
        assert_eq!(vo.parent_id, p.id);
        assert_eq!(vo.parent_version, 3);
        assert_eq!(
            vo.item_map[1].original_text,
            "Talking with a physician makes me tense."
        );
        assert_eq!(vo.locked.as_ref().unwrap().reversed, vec![2]);
        assert!(draft(
            &p,
            &Request {
                new_target: " ".into(),
                ..req()
            }
        )
        .is_err());
    }

    #[test]
    fn finishing_takes_each_items_decision() {
        let p = parent();
        let d = draft(&p, &req()).unwrap();
        let mut ds: Vec<RowDecision> = d
            .rows
            .iter()
            .map(|r| RowDecision {
                number: r.number,
                text: r.proposed.clone(),
                left: None,
                right: None,
                flagged: None,
            })
            .collect();
        assert!(
            finish(d.scale.clone(), &ds[..2]).is_err(),
            "every item needs a decision"
        );
        ds[0].text = "I am calm when I use the AI.".into();
        ds[2].flagged = Some(" ".into());
        assert!(
            finish(d.scale.clone(), &ds).is_err(),
            "a flag needs a reason"
        );
        ds[2].flagged = Some("Appointments have no counterpart for an AI.".into());
        let v = finish(d.scale, &ds).unwrap();
        assert_eq!(v.items[0].text, "I am calm when I use the AI.");
        assert_eq!(v.items[2].text, "I look forward to appointments.");
        assert!(v.variant_of.unwrap().item_map[2].flagged.is_some());
    }

    #[test]
    fn parent_changes_are_offered_and_carried_one_by_one() {
        let mut p = parent();
        let mut v = draft(&p, &req()).unwrap().scale;
        assert!(parent_changes(&v, &p).is_empty());
        p.items[0].text = "I am relaxed when I talk with my physician.".into();
        p.items[2].reverse = ReverseFlag {
            state: ReverseState::Confirmed,
            reason: None,
        };
        p.history.push(HistoryEntry {
            version: 4,
            saved_at: "t".into(),
            summary: "Edited".into(),
        });
        let ch = parent_changes(&v, &p);
        assert_eq!(ch.len(), 2, "{ch:?}");
        assert_eq!(
            v.items[0].text, "I am calm when I talk with my AI.",
            "never applied unasked"
        );
        let text = ch
            .iter()
            .find(|c| matches!(c, ParentChange::ItemText { .. }))
            .unwrap()
            .clone();
        carry(&mut v, &p, &text).unwrap();
        assert_eq!(v.items[0].text, "I am relaxed when I talk with my AI.");
        assert_eq!(v.variant_of.as_ref().unwrap().parent_version, 4);
        let rev = parent_changes(&v, &p);
        assert_eq!(rev.len(), 1);
        decline(&mut v, rev[0].key());
        assert!(
            parent_changes(&v, &p).is_empty(),
            "a declined change is not offered again"
        );
        p.items.push(Item::new(4, "New."));
        let ch = parent_changes(&v, &p);
        let count = ch
            .iter()
            .find(|c| matches!(c, ParentChange::ItemCount { .. }))
            .unwrap();
        assert!(carry(&mut v, &p, count).is_err());
    }

    #[test]
    fn leftovers_and_chains() {
        let v = draft(&parent(), &req()).unwrap().scale;
        let vo = v.variant_of.as_ref().unwrap();
        assert_eq!(
            targets_left("Ask the Physician first.", vo),
            vec!["Physician".to_string()]
        );
        assert!(targets_left("Ask the AI first.", vo).is_empty());
        let scales = vec![
            ("c".to_string(), "C".to_string(), Some("b".to_string())),
            ("b".to_string(), "B".to_string(), Some("a".to_string())),
            ("a".to_string(), "A".to_string(), None),
        ];
        assert_eq!(chain(Some("b"), &scales), vec!["B", "A"]);
    }
}
