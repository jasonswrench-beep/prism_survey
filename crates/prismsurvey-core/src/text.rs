//! Small text helpers shared by the importer and the self-check: words, light stems, wording similarity and edit
//! distance.

use std::collections::HashSet;

/// Words of a text for matching: lower case letters and digits, apostrophes kept inside words, quotes and dashes ignored.
pub fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .replace(['\u{2019}', '\u{2018}'], "'")
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|w| w.trim_matches('\''))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// How alike two wordings are: 1.0 when identical after spacing and punctuation fixes, otherwise the share of words in
/// common (never quite 1.0 unless identical).
pub fn wording_match(a: &str, b: &str) -> f64 {
    let wa = words(a);
    let wb = words(b);
    if wa.is_empty() || wb.is_empty() {
        return 0.0;
    }
    if wa == wb {
        return 1.0;
    }
    let sa: HashSet<&String> = wa.iter().collect();
    let sb: HashSet<&String> = wb.iter().collect();
    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    (inter / union).min(0.99)
}

/// A light stem so that "likes", "liked" and "like" compare equal: common endings cut, then a final e.
pub fn stem(w: &str) -> String {
    let w = w.trim_end_matches("'s");
    let mut s = w.to_string();
    for suffix in ["ingly", "ing", "ies", "ied", "ed", "es", "s"] {
        if s.len() > suffix.len() + 2
            && s.ends_with(suffix)
            && !(suffix == "s" && s.ends_with("ss"))
        {
            s.truncate(s.len() - suffix.len());
            if suffix == "ies" || suffix == "ied" {
                s.push('y');
            }
            break;
        }
    }
    if s.len() > 3 && s.ends_with('e') {
        s.pop();
    }
    s
}

/// The Damerau-Levenshtein distance (adjacent transpositions count as one edit), stopping early above `limit`.
pub fn edit_distance(a: &str, b: &str, limit: usize) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > limit {
        return limit + 1;
    }
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = v;
        }
    }
    d[n][m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_join_inflections() {
        assert_eq!(stem("likes"), stem("liked"));
        assert_eq!(stem("like"), stem("liked"));
        assert_eq!(stem("cultures"), stem("culture"));
        assert_eq!(stem("class"), "class");
    }

    #[test]
    fn transposition_is_one_edit() {
        assert_eq!(edit_distance("dysmoprhia", "dysmorphia", 3), 1);
        assert_eq!(edit_distance("confidance", "confidence", 3), 1);
    }

    #[test]
    fn wording_match_ignores_spacing_and_quotes() {
        assert_eq!(
            wording_match("I don\u{2019}t  like it.", "I don't like it"),
            1.0
        );
        assert!(
            wording_match(
                "I have physically confronted someone.",
                "I have physically confronted someone that I disagreed with."
            ) < 0.75
        );
    }
}
