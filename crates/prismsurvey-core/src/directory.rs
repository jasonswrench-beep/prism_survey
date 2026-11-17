//! The Comm Add On (specification addendum, section 4): the Directory of Communication Related Mental Measures as data.
//! Each entry describes one published measure (it does not hold the measure's items); the app searches the entries and
//! imports the measure itself from its journal article or a stored file, keeping the scale linked to its entry.

use crate::search::{Doc, Field};
use serde::{Deserialize, Serialize};

pub const DIRECTORY_VERSION: u32 = 1;
/// The collection name search results carry.
pub const COLLECTION: &str = "Comm Directory";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Stable: a short form of the name and its place in the book ("adult-communication-behaviors-1").
    pub id: String,
    pub name: String,
    /// The chapter the book lists it under.
    pub chapter: String,
    pub purpose: String,
    pub number_of_items: String,
    pub format: String,
    pub original_sample: String,
    pub reliability: String,
    pub validity: String,
    pub critique: String,
    /// The reference as the book gives it.
    pub source: String,
    /// The reference brought up to APA 7 (spacing, page ranges, DOI as a link when known).
    pub source_apa7: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_research: Vec<String>,
    /// Tidied categories, also the entry's starting tags.
    pub categories: Vec<String>,
    /// A refreshed critique, once written; the book's critique stays beside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub critique_updated: Option<String>,
    /// How the APA 7 reference was checked ("DOI and issue from Crossref"), or what needs a person's eye.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_check: Option<String>,
}

/// One reference's result from the one-time Crossref lookup (tools/crossref_lookup.py).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Lookup {
    /// "matched", "review" or "no match".
    pub status: String,
    pub doi: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub crossref_title: Option<String>,
    pub crossref_year: Option<i32>,
    pub journal: Option<String>,
    pub title_similarity: f64,
    /// The full author list, fetched for references the book shortens with "et al." (tools/crossref_authors.py).
    pub authors: Vec<Author>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Author {
    pub family: String,
    pub given: String,
    pub suffix: Option<String>,
}

impl Author {
    /// "Holmes-Rovner, M." or "Davidson, W. S., II" in APA 7 form.
    pub fn apa(&self) -> String {
        let mut family = self.family.trim().to_string();
        let mut suffix = self.suffix.clone().filter(|x| !x.trim().is_empty());
        for sfx in [" Jr.", " Jr", " Sr.", " III", " II", " IV"] {
            if suffix.is_none() && family.ends_with(sfx) {
                family.truncate(family.len() - sfx.len());
                suffix = Some(sfx.trim().to_string());
            }
        }
        let initials: Vec<String> = self
            .given
            .split_whitespace()
            .map(|w| {
                w.split('-')
                    .filter_map(|part| part.chars().next())
                    .map(|c| format!("{}.", c.to_uppercase()))
                    .collect::<Vec<_>>()
                    .join("-")
            })
            .filter(|w| !w.is_empty())
            .collect();
        let mut out = family;
        if !initials.is_empty() {
            out = format!("{out}, {}", initials.join(" "));
        }
        if let Some(sfx) = suffix {
            out = format!("{out}, {sfx}");
        }
        out
    }
}

/// Replaces a book's "et al." with the rest of the authors, keeping the book's own form for the authors it names.
/// Returns None when the names do not line up with Crossref's list or there are more than 20 authors.
pub fn complete_authors(reference: &str, authors: &[Author]) -> Option<String> {
    let year = regex::Regex::new(r" \(\d{4}[a-z]?\)")
        .unwrap()
        .find(reference)?;
    let (part, rest) = reference.split_at(year.start());
    let etal = regex::Regex::new(r",?\s*et al\.?\s*$").unwrap();
    let named = etal.find(part)?;
    let kept = part[..named.start()].trim_end_matches(',').trim_end();
    let count = regex::Regex::new(r", (?:[A-Z][a-z]?\.[ -]?)+")
        .unwrap()
        .find_iter(kept)
        .count();
    if count == 0 || count >= authors.len() || authors.len() > 20 {
        return None;
    }
    let lower = kept.to_lowercase();
    let family_of = |a: &Author| a.apa().split(',').next().unwrap_or("").to_lowercase();
    if !authors[..count]
        .iter()
        .all(|a| lower.contains(&family_of(a)))
    {
        return None;
    }
    let more: Vec<String> = authors[count..].iter().map(Author::apa).collect();
    let (last, middle) = more.split_last()?;
    let mut out = kept.to_string();
    for m in middle {
        out = format!("{out}, {m}");
    }
    Some(format!("{out}, & {last}{rest}"))
}

/// Puts the issue after the volume, as APA 7 asks: "Communication Education, 33, 272" becomes "..., 33(3), 272".
/// Returns None when the volume is not where it is expected or an issue is already given.
pub fn insert_issue(reference: &str, volume: &str, issue: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r",\s*{}\s*(,|\.)", regex::escape(volume.trim()))).ok()?;
    let m = re.find(reference)?;
    let sep = &reference[m.end() - 1..m.end()];
    Some(format!(
        "{}, {}({}){}{}",
        &reference[..m.start()],
        volume.trim(),
        issue.trim(),
        sep,
        &reference[m.end()..]
    ))
}

/// The book's volume and first page agree with Crossref's.
pub fn volume_and_page_agree(reference: &str, l: &Lookup) -> bool {
    let (Some(v), Some(pages)) = (&l.volume, &l.pages) else {
        return false;
    };
    let first = pages.split(['-', '\u{2013}']).next().unwrap_or("").trim();
    let re =
        regex::Regex::new(r"\(\d{4}[a-z]?\).*?,\s*(\d+)\s*(?:\([^)]*\))?\s*,\s*(\d+)").unwrap();
    re.captures(reference)
        .is_some_and(|c| &c[1] == v.trim() && &c[2] == first && !first.is_empty())
}

impl Entry {
    /// Applies a lookup result: an accepted match brings the DOI and the issue number; a close call is noted for review.
    pub fn apply_lookup(&mut self, l: &Lookup) {
        // A close call whose volume and first page agree with the book is the same article with its title worded a
        // little differently ("communication" for "communicative"): accepted.
        let accepted = l.status == "review" && volume_and_page_agree(&self.source, l);
        let status = if accepted {
            "matched"
        } else {
            l.status.as_str()
        };
        match status {
            "matched" => {
                let mut source = self.source.clone();
                let mut authors_done = false;
                if source.contains("et al.") {
                    if let Some(full) = complete_authors(&source, &l.authors) {
                        source = full;
                        authors_done = true;
                    }
                }
                let mut check = None;
                let mut note = "DOI from Crossref; title and year agree.".to_string();
                if let (Some(v), Some(i)) = (&l.volume, &l.issue) {
                    let given = format!("{}({})", v.trim(), i.trim());
                    if source.contains(&given) {
                        note = "DOI from Crossref; title and year agree; the book already gives the issue number."
                            .into();
                    } else {
                        match insert_issue(&source, v, i) {
                            Some(with_issue) => {
                                source = with_issue;
                                note = "DOI and issue number from Crossref; title and year agree."
                                    .into();
                            }
                            None => {
                                check = Some(format!(
                                    "Check: DOI from Crossref (title and year agree), but Crossref gives volume {v}, issue {i}, which the book's volume does not match; the issue was not added."
                                ))
                            }
                        }
                    }
                }
                self.doi = l.doi.clone();
                self.source_apa7 = apa7(&source, self.doi.as_deref());
                if authors_done {
                    note = format!("{note} The full author list is from Crossref.");
                }
                self.reference_check = Some(check.unwrap_or(note));
            }
            "review" => {
                self.reference_check = Some(format!(
                    "Check: Crossref's closest match is \"{}\" ({}), DOI {}. Not applied.",
                    l.crossref_title.clone().unwrap_or_default(),
                    l.crossref_year.map(|y| y.to_string()).unwrap_or_default(),
                    l.doi.clone().unwrap_or_default()
                ));
            }
            _ => {
                self.reference_check = Some("No match in Crossref; the reference stays as the book gives it (it may be a book, a chapter or a journal without DOIs).".into());
            }
        }
        if accepted {
            if let Some(c) = &self.reference_check {
                self.reference_check = Some(c.replace(
                    "title and year agree",
                    "volume, first page and year agree (the title is worded a little differently)",
                ));
            }
        }
        if self.source_apa7.contains("et al.") {
            let c = self.reference_check.take().unwrap_or_default();
            self.reference_check = Some(format!(
                "Check: the book shortens the author list with \"et al.\"; APA 7 lists every author (up to 20). {c}"
            ).trim().to_string());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub directory_version: u32,
    pub title: String,
    pub authors: Vec<String>,
    /// Who holds the rights, and the permission note shipped with the add-on.
    pub rights: String,
    /// The book's preface (a brief history of mental measures and how to use the directory), with its references,
    /// shown as the add-on's introduction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub introduction: Vec<IntroBlock>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntroKind {
    Heading,
    Paragraph,
    /// One line of a list (the journals searched, the categories).
    Item,
    /// One work in the preface's reference list.
    Reference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntroBlock {
    pub kind: IntroKind,
    pub text: String,
}

impl Entry {
    pub fn to_doc(&self) -> Doc {
        let mut fields = vec![(Field::Title, self.name.clone())];
        fields.extend(self.categories.iter().map(|c| (Field::Tags, c.clone())));
        for d in [
            &self.purpose,
            &self.format,
            &self.number_of_items,
            &self.original_sample,
            &self.reliability,
            &self.validity,
            &self.critique,
        ] {
            fields.push((Field::Description, d.clone()));
        }
        fields.push((Field::Source, self.source_apa7.clone()));
        fields.extend(
            self.related_research
                .iter()
                .map(|r| (Field::Source, r.clone())),
        );
        Doc {
            key: self.id.clone(),
            title: self.name.clone(),
            format: None,
            points: None,
            items: leading_number(&self.number_of_items).unwrap_or(0),
            tags: self.categories.clone(),
            licence: None,
            collection: COLLECTION.into(),
            fields,
        }
    }
}

/// The first number in "The scale consists of 20 Likert-type items" (written out numbers too).
pub fn leading_number(s: &str) -> Option<usize> {
    const WORDS: &[(&str, usize)] = &[
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
        ("fifteen", 15),
        ("twenty", 20),
        ("thirty", 30),
    ];
    for w in s.split(|c: char| !c.is_alphanumeric()) {
        if let Ok(n) = w.parse::<usize>() {
            return Some(n);
        }
        if let Some((_, n)) = WORDS.iter().find(|(x, _)| x.eq_ignore_ascii_case(w)) {
            return Some(*n);
        }
    }
    None
}

/// Merges the book's category variants ("Interaction Trait" and "Interaction Traits", a name split at its commas)
/// into one spelling each.
pub fn tidy_categories(raw: &str) -> Vec<String> {
    // The book's own multi-part category names, which its commas would otherwise split.
    let raw = raw
        .replace(
            "Public Communication, Forensics, & Theater",
            "Public Communication; Forensics & Theater",
        )
        .replace(
            "Public Communication, Forensics & Theater",
            "Public Communication; Forensics & Theater",
        );
    let mut out: Vec<String> = Vec::new();
    for part in raw.split([',', ';']) {
        let p = part.trim().trim_end_matches('.').trim();
        let p = p.strip_prefix("and ").unwrap_or(p).trim();
        if p.is_empty() || p == "&" {
            continue;
        }
        let tidy = match p.to_lowercase().as_str() {
            "interaction trait" => "Interaction Traits",
            "adaptation trait" => "Adaptation Traits",
            "aggression trait" => "Aggression Traits",
            "personality trait" => "Personality Traits",
            "receiver perception" => "Receiver Perceptions",
            "computer-mediated" | "computer mediated communication" => {
                "Computer-Mediated Communication"
            }
            "organizational" => "Organizational Communication",
            "gender and communication" => "Gender & Communication",
            "sexuality and communication" => "Sexuality & Communication",
            "international & intercultural communication"
            | "international and intercultural communication" => "Intercultural Communication",
            "communication apprehension & avoidance" | "apprehension & avoidance traits" => {
                "Communication Apprehension & Avoidance"
            }
            "& theater" | "forensics" | "forensics & theater" => "Forensics & Theater",
            "persuasion and compliance gaining" => "Persuasion & Compliance Gaining",
            "group and team communication" => "Group & Team Communication",
            "conflict & communication" | "conflict and communication" => "Conflict Communication",
            _ => p,
        }
        .to_string();
        if !out.iter().any(|x| x.eq_ignore_ascii_case(&tidy)) {
            out.push(tidy);
        }
    }
    out
}

/// Brings a reference up to APA 7 as far as can be done from the text alone: spacing, an en dash in page ranges, and
/// the DOI written as a link. Issue numbers and DOIs that the book does not give are found when the add-on is prepared.
pub fn apa7(reference: &str, doi: Option<&str>) -> String {
    let mut t = reference.split_whitespace().collect::<Vec<_>>().join(" ");
    // Authors: "&" before the last of two or more, never "and".
    if let Some(year) = regex::Regex::new(r" \((\d{4}[a-z]?|n\.d\.|in press)")
        .unwrap()
        .find(&t)
    {
        let (authors, rest) = t.split_at(year.start());
        let mut a = authors.replace(", and ", ", & ");
        if !a.contains('&') && !a.contains("et al") {
            let last = regex::Regex::new(
                r"^(.+\.,) ([^,&]+, (?:[A-Z][a-z]?\.[ -]?)+(?:,? (?:Jr\.|Sr\.|I{2,3}\.?|IV\.?))?)$",
            )
            .unwrap();
            if let Some(c) = last.captures(a.trim_end()) {
                a = format!("{}, & {}", c[1].trim_end_matches(','), &c[2]);
            }
        }
        t = format!("{a}{rest}");
    }
    // A page range written with a space for the dash ("284 294.").
    t = regex::Regex::new(r"(\d+(?:\(\d+\))?, \d+) (\d+)\.")
        .unwrap()
        .replace_all(&t, "$1-$2.")
        .to_string();
    // An article number: "Article 11".
    t = regex::Regex::new(r"(\d\)?, )article (\d+)")
        .unwrap()
        .replace_all(&t, "${1}Article $2")
        .to_string();
    // With a DOI, the DOI replaces a web address.
    if doi.is_some() {
        t = regex::Regex::new(r"\s*https?://\S+")
            .unwrap()
            .replace_all(&t, |c: &regex::Captures| {
                if c[0].contains("doi.org") {
                    c[0].to_string()
                } else {
                    String::new()
                }
            })
            .to_string();
    }
    t = regex::Regex::new(r"(\d)\s*-\s*(\d)")
        .unwrap()
        .replace_all(&t, "$1\u{2013}$2")
        .to_string();
    t = regex::Regex::new(r"\s+([.,;:])")
        .unwrap()
        .replace_all(&t, "$1")
        .to_string();
    if let Some(d) = doi {
        if !t.contains(d) {
            t = format!("{} https://doi.org/{d}", t.trim_end());
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_merge_their_variants() {
        assert_eq!(
            tidy_categories("Interaction Trait, Receiver Perception"),
            vec!["Interaction Traits", "Receiver Perceptions"]
        );
        assert_eq!(
            tidy_categories("Public Communication, Forensics, & Theater"),
            vec!["Public Communication", "Forensics & Theater"]
        );
        assert_eq!(
            tidy_categories("Interpersonal Communication. , Computer-Mediated"),
            vec![
                "Interpersonal Communication",
                "Computer-Mediated Communication"
            ]
        );
        assert_eq!(
            tidy_categories("Verbal Communication, verbal communication"),
            vec!["Verbal Communication"]
        );
    }

    #[test]
    fn references_are_tidied() {
        let t = apa7(
            "Tuttle, G. E.  (1984).  A field based model . Communication Education, 33, 272 - 283.",
            Some("10.1080/03634528409384751"),
        );
        assert_eq!(t, "Tuttle, G. E. (1984). A field based model. Communication Education, 33, 272\u{2013}283. https://doi.org/10.1080/03634528409384751");
    }

    #[test]
    fn issues_go_after_the_volume() {
        assert_eq!(
            insert_issue(
                "Tuttle, G. E. (1984). A model. Communication Education, 33, 272-283.",
                "33",
                "3"
            )
            .unwrap(),
            "Tuttle, G. E. (1984). A model. Communication Education, 33(3), 272-283."
        );
        assert!(insert_issue("Book. New York: Press.", "33", "3").is_none());
        let mut e = Entry {
            id: "x".into(),
            name: "X".into(),
            chapter: String::new(),
            purpose: String::new(),
            number_of_items: String::new(),
            format: String::new(),
            original_sample: String::new(),
            reliability: String::new(),
            validity: String::new(),
            critique: String::new(),
            source: "Tuttle, G. E. (1984). A model. Communication Education, 33, 272-283.".into(),
            source_apa7: String::new(),
            doi: None,
            related_research: vec![],
            categories: vec![],
            critique_updated: None,
            reference_check: None,
        };
        e.apply_lookup(&Lookup {
            status: "matched".into(),
            doi: Some("10.1080/0363".into()),
            volume: Some("33".into()),
            issue: Some("3".into()),
            ..Default::default()
        });
        assert_eq!(e.source_apa7, "Tuttle, G. E. (1984). A model. Communication Education, 33(3), 272\u{2013}283. https://doi.org/10.1080/0363");
        assert!(e.reference_check.unwrap().contains("issue number"));
    }

    #[test]
    fn close_calls_with_the_same_volume_and_page_are_accepted() {
        let l = Lookup {
            status: "review".into(),
            doi: Some("10.1111/j.1468-2958.1977.tb00518.x".into()),
            volume: Some("3".into()),
            issue: Some("3".into()),
            pages: Some("195-213".into()),
            ..Default::default()
        };
        let src = "Wiemann, J. M. (1977). Explication and test of a model of communication competence. Human Communication Research, 3, 195-213.";
        assert!(volume_and_page_agree(src, &l));
        assert!(!volume_and_page_agree(
            &src.replace("195-213", "200-213"),
            &l
        ));
        let mut e: Entry = serde_json::from_value(serde_json::json!({
            "id": "x", "name": "X", "chapter": "", "purpose": "", "numberOfItems": "", "format": "",
            "originalSample": "", "reliability": "", "validity": "", "critique": "", "source": src,
            "sourceApa7": "", "categories": []
        }))
        .unwrap();
        e.apply_lookup(&l);
        assert!(e.source_apa7.contains("3(3), 195"));
        assert!(e
            .source_apa7
            .ends_with("https://doi.org/10.1111/j.1468-2958.1977.tb00518.x"));
        assert!(e.reference_check.unwrap().contains("first page"));
    }

    #[test]
    fn et_al_is_replaced_by_the_rest_of_the_authors() {
        let a = |f: &str, g: &str| Author {
            family: f.into(),
            given: g.into(),
            suffix: None,
        };
        let authors = vec![
            a("Kroll", "Jill"),
            a("Rothert", "Marilyn"),
            a("Davidson II", "William S."),
            a("Schmitt", "Neal"),
            a("Holmes-Rovner", "Margaret"),
            a("Padonu", "Georgia"),
            a("Reischl", "Thomas M."),
        ];
        assert_eq!(authors[2].apa(), "Davidson, W. S., II");
        assert_eq!(a("Dwyer", "Karen Kangas").apa(), "Dwyer, K. K.");
        assert_eq!(a("Martin", "Jean-Paul").apa(), "Martin, J.-P.");
        assert_eq!(
            complete_authors("Kroll, J., Rothert, M., Davidson, W. S., II., Schmitt, N., Holmes-Rovner, M., et al. (2000). Predictors. Health Communication, 12, 339-360.", &authors).unwrap(),
            "Kroll, J., Rothert, M., Davidson, W. S., II., Schmitt, N., Holmes-Rovner, M., Padonu, G., & Reischl, T. M. (2000). Predictors. Health Communication, 12, 339-360."
        );
        assert!(complete_authors(
            "Smith, J., Jones, K., et al. (2000). X. Y, 1, 1-2.",
            &authors
        )
        .is_none());
    }

    #[test]
    fn references_are_brought_to_apa_7() {
        assert_eq!(
            apa7("Kelleher, T., and Miller, B. M. (2006). Blogs. Journal of Computer-Mediated Communication, 11(2), article 1. http://jcmc.indiana.edu/vol11/issue2/kelleher.html", Some("10.1111/x")),
            "Kelleher, T., & Miller, B. M. (2006). Blogs. Journal of Computer-Mediated Communication, 11(2), Article 1. https://doi.org/10.1111/x"
        );
        assert_eq!(
            apa7("Sparks, G. G., Pellechia, M., Irvine, C. (1998). UFOs. Communication Quarterly, 46(3), 284 294.", None),
            "Sparks, G. G., Pellechia, M., & Irvine, C. (1998). UFOs. Communication Quarterly, 46(3), 284\u{2013}294."
        );
        assert_eq!(
            apa7("Infante, D. A., Wigley, C. J., III. (1986). Verbal aggressiveness. Communication Monographs, 53, 61-69.", None),
            "Infante, D. A., & Wigley, C. J., III. (1986). Verbal aggressiveness. Communication Monographs, 53, 61\u{2013}69."
        );
        assert_eq!(
            apa7("Watson, K. W., Barker, L. L., Weaver, J. B. III. (1995). Listening. International Journal of Listening, 9, 1-13.", None),
            "Watson, K. W., Barker, L. L., & Weaver, J. B. III. (1995). Listening. International Journal of Listening, 9, 1\u{2013}13."
        );
        assert_eq!(
            apa7(
                "McCroskey, J. C. (1970). Measures. Speech Monographs, 37, 269-277.",
                None
            ),
            "McCroskey, J. C. (1970). Measures. Speech Monographs, 37, 269\u{2013}277."
        );
    }

    #[test]
    fn item_counts_are_read() {
        assert_eq!(
            leading_number("The measure consists of 20 Likert-type items."),
            Some(20)
        );
        assert_eq!(
            leading_number("Fifteen semantic differential items"),
            Some(15)
        );
        assert_eq!(leading_number("Not reported"), None);
    }
}
