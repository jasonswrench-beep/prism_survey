//! Cover letter templates for a questionnaire (decision 78): a short one for a brief, minimal-risk survey and a long one
//! that sets out every element of informed consent in 45 CFR 46.116, read against the eCFR text on 2026-11-17.
//!
//! Each template opens with the key information 46.116(a)(5)(i) asks for, then covers every basic element of 46.116(b)
//! that can apply to survey research: (b)(6) applies only to research involving more than minimal risk, so the long
//! letter carries it as a bracketed paragraph to keep or delete, and the short one leaves it out. (b)(9) asks for one of
//! two statements about future use, so both letters offer the two as a bracketed choice. The long letter adds the
//! additional elements of 46.116(c) that fit a survey ((c)(2), (c)(3), (c)(4), (c)(6), (c)(8)); the others concern
//! treatments, biospecimens and genome sequencing.
//!
//! A survey that collects no signature relies on the IRB waiving the signed form (46.117(c)); the letters end with the
//! consent-by-continuing sentence such a waiver allows, and the long one offers signature lines in brackets for an IRB
//! that wants them. Square brackets mark what the researcher fills in or chooses; the window lists any left. A line
//! starting with "## " prints as a heading. These are starting points: the researcher's own IRB has the last word.

use serde::Serialize;

/// The elements of 45 CFR 46.116 a paragraph covers, by their paragraph numbers in the regulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Element {
    /// 46.116(a)(5)(i): key information first.
    KeyInformation,
    /// 46.116(b)(1) to (b)(9).
    Basic(u8),
    /// 46.116(c)(1) to (c)(9).
    Additional(u8),
}

pub struct Paragraph {
    pub text: &'static str,
    pub covers: &'static [Element],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Length {
    Short,
    Long,
}

use Element::{Additional as C, Basic as B, KeyInformation as KEY};

const FUTURE_USE: &str = "[CHOOSE ONE AND DELETE THE OTHER: Your answers, with anything that could identify you removed, may be used for future research or shared with other researchers for future research without asking you again. / Your answers will not be used or shared for future research studies, even with anything that could identify you removed.]";

pub const SHORT: &[Paragraph] = &[
    Paragraph {
        text: "You are invited to take part in a research study, [STUDY TITLE], conducted by [RESEARCHER NAME] of [DEPARTMENT], [INSTITUTION]. The study asks about [PURPOSE]. The survey takes about [MINUTES] minutes, and you must be [AGE] or older to take part. Taking part is your choice.",
        covers: &[KEY, B(1)],
    },
    Paragraph {
        text: "You will answer the questions that follow. None of the procedures is experimental.",
        covers: &[B(1)],
    },
    Paragraph {
        text: "We do not expect any risks beyond those of everyday life, although some questions may feel personal. You may skip any question you prefer not to answer.",
        covers: &[B(2)],
    },
    Paragraph {
        text: "There is no direct benefit to you. Your answers may help researchers better understand [WHAT THE STUDY MAY HELP OTHERS UNDERSTAND].",
        covers: &[B(3)],
    },
    Paragraph {
        text: "The survey does not ask for your name. Your answers will be kept confidential to the extent the law allows and reported only as group results. [HOW THE ANSWERS ARE STORED AND WHO CAN SEE THEM.]",
        covers: &[B(5)],
    },
    Paragraph { text: FUTURE_USE, covers: &[B(9)] },
    Paragraph {
        text: "Taking part is voluntary. If you choose not to take part, simply do not complete the survey; there is no penalty and no loss of any benefit to which you are otherwise entitled. You may stop at any time in the same way.",
        covers: &[B(4), B(8)],
    },
    Paragraph {
        text: "If you have questions about the study, contact [RESEARCHER NAME] at [RESEARCHER EMAIL]. If you have questions about your rights as a research participant, or wish to report a research-related problem or injury, contact [IRB NAME] at [IRB EMAIL] or [IRB PHONE]. This study was reviewed by [IRB NAME] (protocol [PROTOCOL NUMBER]).",
        covers: &[B(7)],
    },
    Paragraph {
        text: "By continuing to the survey, you confirm that you are [AGE] or older and that you agree to take part. You may keep a copy of this page.",
        covers: &[],
    },
];

pub const LONG: &[Paragraph] = &[
    Paragraph { text: "## Key information", covers: &[] },
    Paragraph {
        text: "You are invited to take part in a research study. Its purpose is to [PURPOSE]. You will answer a survey that takes about [MINUTES] minutes. Taking part is voluntary, and you may stop at any time. We do not expect any risks beyond those of everyday life. There is no direct benefit to you, but your answers may help others. Please read the rest of this letter before deciding.",
        covers: &[KEY],
    },
    Paragraph { text: "## Who is doing the study", covers: &[] },
    Paragraph {
        text: "[STUDY TITLE] is conducted by [RESEARCHER NAME], [RESEARCHER ROLE], [DEPARTMENT], [INSTITUTION]. [IF A STUDENT STUDY: The faculty supervisor is [SUPERVISOR NAME].]",
        covers: &[],
    },
    Paragraph { text: "## Why we are doing this study", covers: &[] },
    Paragraph {
        text: "This is a research study. We want to learn [PURPOSE, IN A SENTENCE OR TWO].",
        covers: &[B(1)],
    },
    Paragraph { text: "## What you will be asked to do", covers: &[] },
    Paragraph {
        text: "You will answer questions about [TOPICS]. The survey takes about [MINUTES] minutes. About [NUMBER] people will take part. None of the procedures is experimental. You must be [AGE] or older to take part.",
        covers: &[B(1), C(6)],
    },
    Paragraph { text: "## Risks and discomforts", covers: &[] },
    Paragraph {
        text: "We do not expect any risks beyond those of everyday life. Some questions ask about [SENSITIVE TOPICS, OR DELETE THIS SENTENCE]; you may skip any question you prefer not to answer.",
        covers: &[B(2)],
    },
    Paragraph { text: "## Benefits", covers: &[] },
    Paragraph {
        text: "There is no direct benefit to you. Your answers may help researchers and others better understand [WHAT THE STUDY MAY HELP OTHERS UNDERSTAND].",
        covers: &[B(3)],
    },
    Paragraph { text: "## Other choices", covers: &[] },
    Paragraph {
        text: "You may choose not to take part. That is the only alternative to taking part.",
        covers: &[B(4)],
    },
    Paragraph { text: "## Payment and costs", covers: &[] },
    Paragraph {
        text: "[YOU WILL NOT BE PAID FOR TAKING PART. / DESCRIBE ANY PAYMENT, COURSE CREDIT OR DRAWING, AND WHAT HAPPENS IF YOU STOP EARLY.] Taking part costs you nothing beyond your time.",
        covers: &[C(3)],
    },
    Paragraph {
        text: "[KEEP ONLY IF THE STUDY INVOLVES MORE THAN MINIMAL RISK: Say whether any compensation and any medical treatment are available if injury occurs, what they are, or where more information may be obtained.]",
        covers: &[B(6)],
    },
    Paragraph { text: "## Confidentiality", covers: &[] },
    Paragraph {
        text: "[The survey does not ask for your name. / DESCRIBE WHAT IDENTIFYING INFORMATION IS COLLECTED.] Your answers will be kept confidential to the extent the law allows. They will be stored [HOW AND WHERE] for [HOW LONG], seen only by [WHO], and reported only as group results.",
        covers: &[B(5)],
    },
    Paragraph { text: "## Future use of your answers", covers: &[] },
    Paragraph { text: FUTURE_USE, covers: &[B(9)] },
    Paragraph { text: "## Taking part is voluntary", covers: &[] },
    Paragraph {
        text: "Taking part is your choice. If you choose not to take part, or stop at any time, there is no penalty and no loss of any benefit to which you are otherwise entitled. If you stop partway, [WHAT HAPPENS TO ANSWERS ALREADY GIVEN]. [IF IT APPLIES: The researcher may end your taking part if [CIRCUMSTANCES].]",
        covers: &[B(8), C(2), C(4)],
    },
    Paragraph {
        text: "[If you would like a summary of the results, contact [RESEARCHER NAME]. / Individual results will not be shared with you.]",
        covers: &[C(8)],
    },
    Paragraph { text: "## Questions", covers: &[] },
    Paragraph {
        text: "If you have questions about the study, contact [RESEARCHER NAME] at [RESEARCHER EMAIL] or [RESEARCHER PHONE]. If you have questions about your rights as a research participant, or wish to report a research-related problem or injury, contact [IRB NAME] at [IRB EMAIL] or [IRB PHONE]. This study was reviewed by [IRB NAME] (protocol [PROTOCOL NUMBER]).",
        covers: &[B(7)],
    },
    Paragraph { text: "## Consent", covers: &[] },
    Paragraph {
        text: "By continuing to the survey, you confirm that you are [AGE] or older, that you have read this letter, and that you agree to take part. You may keep a copy of this letter. [IF YOUR IRB REQUIRES A SIGNATURE, REPLACE THIS PARAGRAPH WITH: Name ______ Signature ______ Date ______]",
        covers: &[],
    },
];

/// The title the cover letter part starts with.
pub fn default_title(length: Length) -> &'static str {
    match length {
        Length::Short => "About this survey",
        Length::Long => "Information about taking part in a research study",
    }
}

pub fn paragraphs(length: Length) -> &'static [Paragraph] {
    match length {
        Length::Short => SHORT,
        Length::Long => LONG,
    }
}

/// The template as the text a cover letter part holds, one paragraph per line.
pub fn template(length: Length) -> String {
    paragraphs(length)
        .iter()
        .map(|p| p.text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The bracketed parts still to fill in or choose, in order, each once.
pub fn placeholders(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, ch) in text.char_indices() {
        match ch {
            '[' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            ']' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let found = text[start..=i].to_string();
                    if !out.contains(&found) {
                        out.push(found);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// A line of a cover letter that prints as a heading, with its text.
pub fn heading(line: &str) -> Option<&str> {
    line.trim_start().strip_prefix("## ").map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn covered(length: Length) -> HashSet<Element> {
        paragraphs(length)
            .iter()
            .flat_map(|p| p.covers.iter().copied())
            .collect()
    }

    #[test]
    fn both_letters_open_with_key_information() {
        for length in [Length::Short, Length::Long] {
            let first = paragraphs(length)
                .iter()
                .find(|p| heading(p.text).is_none())
                .unwrap();
            assert!(first.covers.contains(&KEY), "{length:?}");
        }
    }

    #[test]
    fn the_long_letter_covers_every_basic_element_and_the_survey_ones_of_the_additional() {
        let c = covered(Length::Long);
        for n in 1..=9 {
            assert!(c.contains(&B(n)), "46.116(b)({n})");
        }
        for n in [2, 3, 4, 6, 8] {
            assert!(c.contains(&C(n)), "46.116(c)({n})");
        }
    }

    #[test]
    fn the_short_letter_covers_every_basic_element_but_the_more_than_minimal_risk_one() {
        let c = covered(Length::Short);
        for n in [1, 2, 3, 4, 5, 7, 8, 9] {
            assert!(c.contains(&B(n)), "46.116(b)({n})");
        }
        assert!(!c.contains(&B(6)));
    }

    #[test]
    fn placeholders_are_listed_once_with_nested_brackets_kept_whole() {
        let p = placeholders("Hi [NAME], [NAME]. [IF A: The supervisor is [SUP].] Done.");
        assert_eq!(p, vec!["[NAME]", "[IF A: The supervisor is [SUP].]"]);
        assert!(placeholders(&template(Length::Short)).contains(&"[STUDY TITLE]".to_string()));
        assert_eq!(heading("## Risks"), Some("Risks"));
        assert_eq!(heading("Risks"), None);
    }
}
