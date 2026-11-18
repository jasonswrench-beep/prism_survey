//! The bank of ready-made demographic questions a study can add to its demographic block (decision 77). Each question
//! goes into the block as plain text the researcher can still edit: the question on one line, then one line per answer
//! choice starting with a tick box (☐), or a blank (______) when the answer is written in. Every closed question offers
//! "Prefer not to say", ranges never overlap, and race and ethnicity follow the single combined question of the 2024
//! US federal standard (OMB SPD 15), marked all that apply.

use serde::Serialize;

/// The mark that starts an answer choice line; the Word export prints such lines as tick-box choices.
pub const CHOICE_MARK: char = '☐';
/// The write-in blank.
pub const BLANK: &str = "______";

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BankQuestion {
    /// A stable short name.
    pub id: &'static str,
    /// The heading it is listed under.
    pub group: &'static str,
    /// The short name shown in the list.
    pub label: &'static str,
    pub question: &'static str,
    /// Empty for a written answer.
    pub choices: &'static [&'static str],
    /// More than one choice may be marked.
    pub multiple: bool,
}

const PNTS: &str = "Prefer not to say";

macro_rules! q {
    ($id:literal, $group:literal, $label:literal, $question:literal) => {
        BankQuestion { id: $id, group: $group, label: $label, question: $question, choices: &[], multiple: false }
    };
    ($id:literal, $group:literal, $label:literal, $question:literal, [$($c:expr),+ $(,)?]) => {
        BankQuestion { id: $id, group: $group, label: $label, question: $question, choices: &[$($c),+], multiple: false }
    };
    ($id:literal, $group:literal, $label:literal, $question:literal, all [$($c:expr),+ $(,)?]) => {
        BankQuestion { id: $id, group: $group, label: $label, question: $question, choices: &[$($c),+], multiple: true }
    };
}

pub const BANK: &[BankQuestion] = &[
    // About you
    q!(
        "age",
        "About you",
        "Age in years",
        "What is your age in years?"
    ),
    q!(
        "age-range",
        "About you",
        "Age range",
        "What is your age?",
        [
            "Under 18",
            "18–24",
            "25–34",
            "35–44",
            "45–54",
            "55–64",
            "65 or older",
            PNTS
        ]
    ),
    q!(
        "gender",
        "About you",
        "Gender",
        "What is your gender?",
        [
            "Woman",
            "Man",
            "Nonbinary",
            "Prefer to self-describe: ______",
            PNTS
        ]
    ),
    q!(
        "sex-at-birth",
        "About you",
        "Sex assigned at birth",
        "What sex were you assigned at birth?",
        ["Female", "Male", "Intersex", PNTS]
    ),
    q!(
        "orientation",
        "About you",
        "Sexual orientation",
        "Which best describes your sexual orientation?",
        [
            "Heterosexual or straight",
            "Gay or lesbian",
            "Bisexual",
            "Prefer to self-describe: ______",
            PNTS
        ]
    ),
    q!("race-ethnicity", "About you", "Race and ethnicity (2024 federal standard)", "What is your race or ethnicity? Mark all that apply.",
    all [
        "American Indian or Alaska Native",
        "Asian",
        "Black or African American",
        "Hispanic or Latino",
        "Middle Eastern or North African",
        "Native Hawaiian or Pacific Islander",
        "White",
        "Some other race or ethnicity: ______",
        PNTS,
    ]),
    q!(
        "disability",
        "About you",
        "Disability",
        "Do you have a disability?",
        ["Yes", "No", "Prefer to self-describe: ______", PNTS]
    ),
    q!(
        "veteran",
        "About you",
        "Military service",
        "Have you served in the armed forces?",
        [
            "Yes, I am serving now",
            "Yes, I served in the past",
            "No",
            PNTS
        ]
    ),
    q!(
        "home-language",
        "About you",
        "Language at home",
        "What language do you speak most at home?",
        [
            "English",
            "Spanish",
            "Chinese",
            "French",
            "Another language: ______",
            PNTS
        ]
    ),
    q!("religion", "About you", "Religion", "What is your religion, if any? Mark all that apply.",
        all ["Christian", "Jewish", "Muslim", "Hindu", "Buddhist", "No religion", "Another religion: ______", PNTS]),
    q!(
        "politics",
        "About you",
        "Political viewpoint",
        "How would you describe your political views?",
        [
            "Very liberal",
            "Liberal",
            "Moderate",
            "Conservative",
            "Very conservative",
            PNTS
        ]
    ),
    q!("news", "About you", "News sources", "Where do you get most of your news? Mark all that apply.",
        all ["Television", "Newspapers or news websites", "Social media", "Radio or podcasts", "Friends and family", "Other: ______", PNTS]),
    // School and work
    q!(
        "education",
        "School and work",
        "Highest education",
        "What is the highest level of education you have completed?",
        [
            "Less than high school",
            "High school or GED",
            "Some college, no degree",
            "Associate degree",
            "Bachelor’s degree",
            "Master’s degree",
            "Professional degree",
            "Doctorate",
            PNTS,
        ]
    ),
    q!(
        "school-year",
        "School and work",
        "Year in school",
        "What is your year in school?",
        [
            "First-year",
            "Second-year",
            "Third-year",
            "Fourth-year or beyond",
            "Graduate student",
            "Not a student",
            PNTS
        ]
    ),
    q!(
        "enrolment",
        "School and work",
        "Full-time or part-time study",
        "Are you enrolled full-time or part-time?",
        ["Full-time", "Part-time", "Not enrolled", PNTS]
    ),
    q!(
        "employment",
        "School and work",
        "Employment status",
        "What is your current employment status?",
        [
            "Employed full-time",
            "Employed part-time",
            "Self-employed",
            "Not employed, looking for work",
            "Not employed, not looking for work",
            "Homemaker or stay-at-home parent",
            "Student",
            "Retired",
            "Unable to work",
            PNTS,
        ]
    ),
    q!(
        "occupation",
        "School and work",
        "Occupation",
        "What is your occupation or job title?"
    ),
    q!(
        "department",
        "School and work",
        "Department",
        "In what department or unit do you work?"
    ),
    q!(
        "tenure",
        "School and work",
        "Years with the organization",
        "How long have you worked for your current organization?",
        [
            "Less than 1 year",
            "1–2 years",
            "3–5 years",
            "6–10 years",
            "More than 10 years",
            PNTS
        ]
    ),
    q!(
        "job-level",
        "School and work",
        "Job level",
        "Which best describes your position?",
        [
            "Entry level",
            "Experienced, not a supervisor",
            "Supervisor or manager",
            "Senior manager or director",
            "Executive",
            PNTS
        ]
    ),
    q!(
        "supervisor",
        "School and work",
        "Supervises others",
        "Do you supervise other employees?",
        ["Yes", "No", PNTS]
    ),
    q!(
        "org-size",
        "School and work",
        "Organization size",
        "About how many people work for your organization?",
        [
            "1–9",
            "10–49",
            "50–249",
            "250–999",
            "1,000 or more",
            "Don’t know",
            PNTS
        ]
    ),
    // Home and household
    q!(
        "relationship",
        "Home and household",
        "Relationship status",
        "What is your relationship status?",
        [
            "Single, never married",
            "Married",
            "Living with a partner",
            "Separated",
            "Divorced",
            "Widowed",
            PNTS
        ]
    ),
    q!(
        "household-size",
        "Home and household",
        "Household size",
        "How many people live in your household, including you?"
    ),
    q!(
        "children",
        "Home and household",
        "Children at home",
        "How many children under 18 live in your household?",
        ["None", "1", "2", "3", "4", "More than 4", PNTS]
    ),
    q!(
        "income",
        "Home and household",
        "Household income",
        "What was your total household income before taxes last year?",
        [
            "Less than $10,000",
            "$10,000–$19,999",
            "$20,000–$29,999",
            "$30,000–$39,999",
            "$40,000–$49,999",
            "$50,000–$59,999",
            "$60,000–$69,999",
            "$70,000–$79,999",
            "$80,000–$89,999",
            "$90,000–$99,999",
            "$100,000 or more",
            PNTS,
        ]
    ),
    q!(
        "housing",
        "Home and household",
        "Own or rent",
        "Do you own or rent your home?",
        ["Own", "Rent", "Other: ______", PNTS]
    ),
    // Where you live
    q!(
        "country",
        "Where you live",
        "Country",
        "In what country do you live?"
    ),
    q!(
        "us-state",
        "Where you live",
        "US state or territory",
        "In what US state or territory do you live?"
    ),
    q!(
        "postal-code",
        "Where you live",
        "ZIP or postal code",
        "What is your ZIP or postal code?"
    ),
    q!(
        "community",
        "Where you live",
        "Urban, suburban or rural",
        "Which best describes where you live?",
        ["Urban", "Suburban", "Rural", PNTS]
    ),
];

impl BankQuestion {
    /// The lines the question adds to a demographic block.
    pub fn text(&self) -> String {
        if self.choices.is_empty() {
            return format!("{} {BLANK}", self.question);
        }
        let mut out = String::from(self.question);
        for c in self.choices {
            out.push('\n');
            out.push(CHOICE_MARK);
            out.push(' ');
            out.push_str(c);
        }
        out
    }
}

/// True for a line of a demographic block that is an answer choice.
pub fn is_choice_line(line: &str) -> bool {
    line.trim_start().starts_with(CHOICE_MARK)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_question_is_named_once_and_every_closed_one_lets_people_opt_out() {
        let ids: HashSet<_> = BANK.iter().map(|q| q.id).collect();
        assert_eq!(ids.len(), BANK.len());
        for q in BANK {
            assert!(
                q.question.ends_with('?') || q.question.ends_with('.'),
                "{}",
                q.id
            );
            if !q.choices.is_empty() {
                assert_eq!(
                    q.choices.last(),
                    Some(&PNTS),
                    "{} needs an opt-out last",
                    q.id
                );
                assert!(q.choices.len() >= 3, "{}", q.id);
            }
            assert_eq!(
                q.multiple,
                q.question.contains("Mark all that apply"),
                "{}",
                q.id
            );
        }
    }

    #[test]
    fn race_and_ethnicity_is_one_combined_question_with_the_seven_federal_categories() {
        let q = BANK.iter().find(|q| q.id == "race-ethnicity").unwrap();
        assert!(q.multiple);
        for c in [
            "American Indian or Alaska Native",
            "Asian",
            "Black or African American",
            "Hispanic or Latino",
            "Middle Eastern or North African",
            "Native Hawaiian or Pacific Islander",
            "White",
        ] {
            assert!(q.choices.contains(&c), "{c}");
        }
    }

    #[test]
    fn a_question_becomes_lines_the_block_can_hold() {
        let age = BANK.iter().find(|q| q.id == "age").unwrap();
        assert_eq!(age.text(), "What is your age in years? ______");
        let sup = BANK.iter().find(|q| q.id == "supervisor").unwrap();
        assert_eq!(
            sup.text(),
            "Do you supervise other employees?\n☐ Yes\n☐ No\n☐ Prefer not to say"
        );
        assert!(is_choice_line("☐ Yes") && !is_choice_line("Do you?"));
    }
}
