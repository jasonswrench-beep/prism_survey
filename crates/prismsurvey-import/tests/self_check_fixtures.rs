//! The synthetic stored scales in tests/fixtures/self-check recreate, in new words, each worked example of the
//! specification's scoring self-check. Each is imported and checked here: the self-check must report every worked
//! example, and must stay quiet on the two guards.

use prismsurvey_core::scale::reversed_numbers;
use prismsurvey_core::selfcheck::{self_check, CheckKind, Finding};
use prismsurvey_core::*;
use prismsurvey_import::import_path;
use std::path::PathBuf;

fn import(name: &str) -> Scale {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/self-check")
        .join(name);
    let mut v = import_path(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(v.len(), 1, "{name}");
    v.remove(0).scale
}

fn findings(s: &Scale, kind: CheckKind) -> Vec<Finding> {
    self_check(s)
        .into_iter()
        .filter(|f| f.check == kind)
        .collect()
}

fn flagged_items(s: &Scale, kinds: &[CheckKind]) -> Vec<u32> {
    let mut v: Vec<u32> = self_check(s)
        .into_iter()
        .filter(|f| kinds.contains(&f.check))
        .flat_map(|f| f.items)
        .collect();
    v.sort();
    v.dedup();
    v
}

const DIRECTION: &[CheckKind] = &[
    CheckKind::RecodeWording,
    CheckKind::LateItem,
    CheckKind::RecodeMissing,
];

#[test]
fn recode_against_wording() {
    let s = import("01-recode-against-wording.txt");
    assert_eq!(reversed_numbers(&s), vec![3, 4, 5]);
    // Item 4 is recoded but reads with the construct.
    assert_eq!(flagged_items(&s, DIRECTION), vec![4]);
}

#[test]
fn recode_swapped_on_part_of_an_odd_even_scale() {
    let s = import("02-recode-swapped-odd-even.txt");
    assert_eq!(reversed_numbers(&s), vec![3, 4, 7, 8, 9, 10]);
    // 9 and 10 are plain but recoded; 11 and 12 are negated but not.
    assert_eq!(flagged_items(&s, DIRECTION), vec![9, 10, 11, 12]);
}

#[test]
fn recode_missing() {
    let s = import("03-recode-missing.txt");
    let f = findings(&s, CheckKind::RecodeMissing);
    assert_eq!(f.len(), 1, "{:?}", self_check(&s));
    assert_eq!(f[0].items, vec![1, 4, 6]);
}

#[test]
fn late_added_item() {
    let s = import("04-late-added-item.txt");
    let f = findings(&s, CheckKind::LateItem);
    assert_eq!(f.len(), 1, "{:?}", self_check(&s));
    assert_eq!(f[0].items, vec![7]);
}

#[test]
fn repeats_in_a_list() {
    let s = import("05-repeats-in-a-list.txt");
    let f = findings(&s, CheckKind::ListRepeat);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].message.contains("likely meant as 8"),
        "{}",
        f[0].message
    );
    assert!(flagged_items(&s, DIRECTION).contains(&8));
}

#[test]
fn duplicate_items() {
    let s = import("06-duplicate-items.txt");
    let keys: Vec<String> = findings(&s, CheckKind::DuplicateItem)
        .into_iter()
        .map(|f| f.key)
        .collect();
    assert_eq!(keys, vec!["duplicate:1:5", "duplicate:2:6"]);
}

#[test]
fn one_subscale_per_item_passes() {
    let s = import("07-one-subscale-per-item-pass.txt");
    assert!(findings(&s, CheckKind::SubscaleCoverage).is_empty());
    assert!(
        flagged_items(&s, DIRECTION).is_empty(),
        "{:?}",
        self_check(&s)
    );
}

#[test]
fn subscale_names_agree() {
    let s = import("08-subscale-names-agree.txt");
    let f = findings(&s, CheckKind::SubscaleNames);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("Warm") && f[0].message.contains("Friendly"));
}

#[test]
fn item_count_against_title_and_file_name() {
    let s = import("09-Garden Worry-5.txt");
    let keys: Vec<String> = findings(&s, CheckKind::ItemCount)
        .into_iter()
        .map(|f| f.key)
        .collect();
    assert_eq!(keys, vec!["itemCount:file"]);
}

#[test]
fn score_range_stated_and_missing() {
    assert!(findings(&import("10-score-range-pass.txt"), CheckKind::ScoreRange).is_empty());
    let f = findings(&import("11-score-range-missing.txt"), CheckKind::ScoreRange);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("8 to 40"));
}

#[test]
fn instructions_against_format() {
    let f = findings(
        &import("12-instructions-against-format.txt"),
        CheckKind::InstructionsFormat,
    );
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("blanks"));
}

#[test]
fn leftover_source() {
    assert_eq!(
        findings(&import("13-leftover-source.txt"), CheckKind::LeftoverSource).len(),
        1
    );
}

#[test]
fn typing_slips() {
    let s = import("14-typing-slips.txt");
    let msgs: Vec<String> = findings(&s, CheckKind::TypingSlip)
        .into_iter()
        .map(|f| f.message)
        .collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("item 2") && m.contains("stray mark")),
        "{msgs:?}"
    );
    assert!(msgs.iter().any(|m| m.contains("did not used")), "{msgs:?}");
    assert!(
        msgs.iter().any(|m| m.contains("\"confidance\"")),
        "{msgs:?}"
    );
}

#[test]
fn score_names_agree() {
    let f = findings(&import("15-score-names-agree.txt"), CheckKind::ScoreNames);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("Overall Stage Fright Score"));
}

#[test]
fn guards_raise_no_false_alarm() {
    // A negated item that scores in the construct's direction is correctly left off the list.
    let a = import("16-guard-negation-not-reversed.txt");
    assert_eq!(reversed_numbers(&a), vec![2, 5]);
    assert!(
        flagged_items(&a, DIRECTION).is_empty(),
        "{:?}",
        self_check(&a)
    );
    // An item with no negation word is correctly reversed.
    let b = import("17-guard-reversed-without-negation.txt");
    assert_eq!(reversed_numbers(&b), vec![2, 4]);
    assert!(
        flagged_items(&b, DIRECTION).is_empty(),
        "{:?}",
        self_check(&b)
    );
}
