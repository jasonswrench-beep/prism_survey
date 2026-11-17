//! The researcher's own tags (specification addendum, section 1). There is no fixed list. Tags are kept as typed;
//! two tags that differ only in capital letters or spacing count as one, and the first spelling is kept.

use crate::model::Scale;

/// A tag with its spacing tidied: ends trimmed, runs of spaces made one. Empty when nothing is left.
pub fn tidy(tag: &str) -> String {
    tag.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The form two tags are compared in.
pub fn key(tag: &str) -> String {
    tidy(tag).to_lowercase()
}

/// Adds a tag unless the scale already has it (in any capitalisation). Returns true when added.
pub fn add(scale: &mut Scale, tag: &str) -> bool {
    let t = tidy(tag);
    if t.is_empty() || scale.tags.iter().any(|x| key(x) == key(&t)) {
        return false;
    }
    scale.tags.push(t);
    true
}

pub fn remove(scale: &mut Scale, tag: &str) -> bool {
    let before = scale.tags.len();
    scale.tags.retain(|x| key(x) != key(tag));
    scale.tags.len() != before
}

/// Renames a tag on a scale, merging it into the new name when the scale has both. Returns true when anything changed.
pub fn rename(scale: &mut Scale, from: &str, to: &str) -> bool {
    let to = tidy(to);
    if to.is_empty() {
        return false;
    }
    let Some(pos) = scale.tags.iter().position(|x| key(x) == key(from)) else {
        return false;
    };
    if scale
        .tags
        .iter()
        .enumerate()
        .any(|(i, x)| i != pos && key(x) == key(&to))
    {
        scale.tags.remove(pos);
    } else {
        scale.tags[pos] = to;
    }
    true
}

/// Tidies a whole tag list: spacing fixed, empties and repeats dropped, first spelling kept.
pub fn normalise(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = tidy(t);
        if !t.is_empty() && !out.iter().any(|x| key(x) == key(&t)) {
            out.push(t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Format;
    use crate::scale::new_blank;

    #[test]
    fn tags_are_free_form_and_case_blind() {
        let mut s = new_blank(Format::LikertBlankLine);
        assert!(add(&mut s, "  Health   Communication "));
        assert!(!add(&mut s, "health communication"));
        assert!(add(&mut s, "Dissertation 2027"));
        assert_eq!(s.tags, vec!["Health Communication", "Dissertation 2027"]);
        assert!(remove(&mut s, "DISSERTATION 2027"));
        assert_eq!(s.tags, vec!["Health Communication"]);
    }

    #[test]
    fn rename_merges_when_both_are_present() {
        let mut s = new_blank(Format::LikertBlankLine);
        add(&mut s, "Health");
        add(&mut s, "Health Communication");
        assert!(rename(&mut s, "health", "Health Communication"));
        assert_eq!(s.tags, vec!["Health Communication"]);
        assert_eq!(
            normalise(&["a".into(), " A ".into(), "".into(), "b".into()]),
            vec!["a", "b"]
        );
    }
}
