//! Reads the Directory of Communication Related Mental Measures (a Word file) into entries for the Comm Add On. Each
//! entry starts at "Test Name:" and runs through its labelled parts (Purpose, Number of items, Format, Original
//! Sample, Reliability, Validity, Critique, Source, Related Research, Categories); a part's text can run over several
//! paragraphs. The chapter comes from the "Chapter One" headings, and the measure's name in ordinary capitals from the
//! book's index.

use crate::document::{Block, Document};
use prismsurvey_core::directory::{
    apa7, tidy_categories, Directory, Entry, IntroBlock, IntroKind, DIRECTORY_VERSION,
};
use regex::Regex;
use std::collections::HashMap;

const LABELS: &[&str] = &[
    "Test Name",
    "Purpose",
    "Number of items",
    "Format",
    "Original Sample",
    "Reliability",
    "Validity",
    "Critique",
    "Source",
    "Related Research",
    "Categories",
];

fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(60).collect()
}

/// "ADULT COMMUNICATION BEHAVIORS (ACB)" in ordinary capitals, for names the index does not list.
fn title_case(s: &str) -> String {
    const SMALL: &[&str] = &[
        "a", "an", "and", "as", "at", "by", "for", "in", "of", "on", "or", "the", "to", "with",
        "toward", "towards",
    ];
    s.split_whitespace()
        .enumerate()
        .map(|(i, w)| {
            let inner = w.trim_matches(|c: char| !c.is_alphanumeric());
            let acronym = w.starts_with('(')
                || (inner.len() <= 4
                    && inner
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                    && inner.chars().any(|c| c.is_ascii_digit()));
            if acronym {
                return w.to_string();
            }
            let lower = w.to_lowercase();
            if i > 0 && SMALL.contains(&lower.as_str()) {
                return lower;
            }
            let mut c = lower.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

pub fn convert(doc: &Document) -> Directory {
    let paras: Vec<(String, bool)> = doc
        .blocks
        .iter()
        .flat_map(|b| match b {
            Block::Para(p) => vec![(p.text.clone(), p.bold == Some(true))],
            Block::Table(_) => vec![],
        })
        .collect();
    // The subject index lists each measure in ordinary capitals: "Adult Communication Behaviors, 1".
    let index_at = paras
        .iter()
        .position(|(t, _)| tidy(t) == "Subject Index")
        .unwrap_or(paras.len());
    let mut proper: HashMap<String, String> = HashMap::new();
    let index_line = Regex::new(r"^(.+?),\s*\d+(?:\s+\w+)?\s*$").unwrap();
    for (t, _) in &paras[index_at..] {
        if let Some(c) = index_line.captures(tidy(t).as_str()) {
            proper.entry(key(&c[1])).or_insert_with(|| tidy(&c[1]));
        }
    }
    let label_re = Regex::new(&format!(r"^\s*({})\s*:\s*(.*)$", LABELS.join("|"))).unwrap();
    let chapter_re = Regex::new(r"^Chapter [A-Z][a-z]+(?:[- ][A-Z][a-z]+)?$").unwrap();
    let doi_re = Regex::new(r"(?i)\b(10\.\d{4,9}/[^\s]+[^\s.,;])").unwrap();

    let mut entries: Vec<Entry> = Vec::new();
    let mut chapter = String::new();
    let mut expect_chapter_title = false;
    let mut current: Option<HashMap<&'static str, Vec<String>>> = None;
    let mut last_label: &'static str = "";
    let finish =
        |cur: HashMap<&'static str, Vec<String>>, chapter: &str, entries: &mut Vec<Entry>| {
            let get = |k: &str| cur.get(k).map(|v| tidy(&v.join(" "))).unwrap_or_default();
            let raw_name = get("Test Name");
            if raw_name.is_empty() {
                return;
            }
            let name = proper
                .get(&key(&raw_name))
                .cloned()
                .unwrap_or_else(|| title_case(&raw_name));
            let source = get("Source");
            let doi = doi_re.captures(&source).map(|c| c[1].to_string());
            let related: Vec<String> = cur
                .get("Related Research")
                .map(|v| {
                    v.iter()
                        .map(|r| tidy(r))
                        .filter(|r| !r.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let id = format!("{}-{}", slug(&name), entries.len() + 1);
            entries.push(Entry {
                id,
                name,
                chapter: chapter.to_string(),
                purpose: get("Purpose"),
                number_of_items: get("Number of items"),
                format: get("Format"),
                original_sample: get("Original Sample"),
                reliability: get("Reliability"),
                validity: get("Validity"),
                critique: get("Critique"),
                source_apa7: apa7(&source, doi.as_deref()),
                source,
                doi,
                related_research: related,
                categories: tidy_categories(&get("Categories")),
                critique_updated: None,
                reference_check: None,
            });
        };
    for (t, bold) in &paras[..index_at] {
        let text = tidy(t);
        if text.is_empty() {
            continue;
        }
        if chapter_re.is_match(&text) {
            if let Some(cur) = current.take() {
                finish(cur, &chapter, &mut entries);
            }
            expect_chapter_title = true;
            continue;
        }
        if expect_chapter_title {
            chapter = text.strip_suffix(" Measures").unwrap_or(&text).to_string();
            expect_chapter_title = false;
            continue;
        }
        // A bold heading in capitals ("RELATED MEASURES") ends the part above it and is not part of any entry.
        if *bold
            && text.chars().any(char::is_alphabetic)
            && !text.chars().any(char::is_lowercase)
            && !label_re.is_match(&text)
        {
            last_label = "";
            continue;
        }
        if let Some(c) = label_re.captures(&text) {
            let label = LABELS
                .iter()
                .find(|l| l.eq_ignore_ascii_case(&c[1]))
                .copied()
                .unwrap_or("");
            if label == "Test Name" {
                if let Some(cur) = current.take() {
                    finish(cur, &chapter, &mut entries);
                }
                current = Some(HashMap::new());
            }
            if let Some(cur) = current.as_mut() {
                cur.entry(label).or_default().push(c[2].to_string());
                last_label = label;
            }
            continue;
        }
        // A paragraph with no label continues the part above it (another related study, a long critique).
        if let Some(cur) = current.as_mut() {
            if !last_label.is_empty() {
                cur.entry(last_label).or_default().push(text);
            }
        }
    }
    if let Some(cur) = current.take() {
        finish(cur, &chapter, &mut entries);
    }
    Directory {
        directory_version: DIRECTORY_VERSION,
        title: "Directory of Communication Related Mental Measures".into(),
        authors: vec!["Jason S. Wrench".into(), "Doreen Jowi".into(), "Alan K. Goodboy".into()],
        rights: "The rights to the text are held by Jason S. Wrench (confirmed 2026-11-17). The directory describes published measures; it does not reproduce their items.".into(),
        introduction: introduction(&paras),
        entries,
    }
}

/// Headings of the book's preface that come straight after a list, which the layout alone cannot tell from the
/// list's last line.
const HEADINGS_AFTER_LISTS: &[&str] = &["Using the Directory"];

/// The preface, from its heading to the dedication, and the preface's reference list, which follows the
/// acknowledgements; all before the first chapter. A short line that a long paragraph follows is a heading; other
/// short lines are list items.
fn introduction(paras: &[(String, bool)]) -> Vec<IntroBlock> {
    let lines: Vec<String> = paras
        .iter()
        .flat_map(|(t, _)| t.split(['\n', '\r', '\u{b}']).map(tidy).collect::<Vec<_>>())
        .collect();
    let first_chapter = lines
        .iter()
        .position(|t| t.starts_with("Chapter ") && t.split_whitespace().count() <= 3)
        .unwrap_or(lines.len());
    let front = &lines[..first_chapter];
    let Some(start) = front.iter().position(|t| t == "Preface") else {
        return Vec::new();
    };
    let end = front[start..]
        .iter()
        .position(|t| t == "Dedication" || t == "Acknowledgements" || t == "References")
        .map(|k| start + k)
        .unwrap_or(front.len());
    let body: Vec<&String> = front[start + 1..end]
        .iter()
        .filter(|t| !t.is_empty())
        .collect();
    let words = |t: &str| t.split_whitespace().count();
    let mut out = Vec::new();
    for (k, t) in body.iter().enumerate() {
        let short = |t: &str| words(t) <= 10 && !t.ends_with(['.', '?', '!']);
        let next_long = body.get(k + 1).is_some_and(|n| words(n) > 20);
        // The last line of a list is followed by a paragraph too; a heading comes after a paragraph, not a list line.
        let after_list = k > 0 && short(body[k - 1]);
        let kind = if !short(t) {
            IntroKind::Paragraph
        } else if next_long && (!after_list || HEADINGS_AFTER_LISTS.contains(&t.as_str())) {
            IntroKind::Heading
        } else {
            IntroKind::Item
        };
        out.push(IntroBlock {
            kind,
            text: t.to_string(),
        });
    }
    if let Some(r) = front[end..].iter().position(|t| t == "References") {
        let refs: Vec<&String> = front[end + r + 1..]
            .iter()
            .filter(|t| !t.is_empty())
            .collect();
        if !refs.is_empty() {
            out.push(IntroBlock {
                kind: IntroKind::Heading,
                text: "References".into(),
            });
            out.extend(refs.into_iter().map(|t| IntroBlock {
                kind: IntroKind::Reference,
                text: t.to_string(),
            }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Block, Document, Paragraph};

    #[test]
    fn the_preface_becomes_the_introduction() {
        let p = |t: &str| (t.to_string(), false);
        let long = "This paragraph is long enough to be read as the body of a section, with more than twenty words in it, so the line above it is a heading.";
        let paras = vec![
            p("Table of Contents"),
            p("Preface"),
            p("A Brief History"),
            p(long),
            p("First Journal"),
            p("Second Journal"),
            p(long),
            p("Dedication"),
            p("For the students."),
            p("References"),
            p("Author, A. (1900). A work. Publisher."),
            p("Chapter One"),
            p("Test Name: Something"),
        ];
        let intro = introduction(&paras);
        let kinds: Vec<IntroKind> = intro.iter().map(|b| b.kind).collect();
        use IntroKind::*;
        assert_eq!(
            kinds,
            vec![Heading, Paragraph, Item, Item, Paragraph, Heading, Reference]
        );
        assert!(
            intro.iter().all(|b| b.text != "For the students."),
            "the dedication is left out"
        );
    }

    fn p(t: &str, bold: bool) -> Block {
        Block::Para(Paragraph {
            text: t.into(),
            bold: Some(bold),
        })
    }

    #[test]
    fn entries_chapters_and_index_names() {
        let doc = Document {
            blocks: vec![
                p("Chapter One", true),
                p("Verbal Communication", true),
                p("Test Name: ADULT COMMUNICATION BEHAVIORS", false),
                p("Purpose: To assess talk.", false),
                p(
                    "Number of items: The measure consists of 20 Likert-type items.",
                    false,
                ),
                p("Critique: Small sample.", false),
                p("A second paragraph of critique.", false),
                p(
                    "Source: Tuttle, G. E. (1984). A model. Communication Education, 33, 272-283.",
                    false,
                ),
                p("Related Research: One, A. (1990). First. J, 1, 1-2.", false),
                p("Two, B. (1991). Second. J, 2, 3-4.", false),
                p("Categories: Verbal Communication, Interaction Trait", false),
                p("Test Name: SPEECH DIALECT ATTITUDE SCALE (SDAS)", false),
                p("Purpose: Attitudes.", false),
                p("Categories: Verbal Communication.", false),
                p("Subject Index", false),
                p("Adult Communication Behaviors, 1 ", false),
            ],
        };
        let d = convert(&doc);
        assert_eq!(d.entries.len(), 2);
        let a = &d.entries[0];
        assert_eq!(a.name, "Adult Communication Behaviors");
        assert_eq!(a.chapter, "Verbal Communication");
        assert_eq!(a.critique, "Small sample. A second paragraph of critique.");
        assert_eq!(a.related_research.len(), 2);
        assert_eq!(
            a.categories,
            vec!["Verbal Communication", "Interaction Traits"]
        );
        assert!(a.source_apa7.contains("272\u{2013}283"));
        assert_eq!(d.entries[1].name, "Speech Dialect Attitude Scale (SDAS)");
    }
}
