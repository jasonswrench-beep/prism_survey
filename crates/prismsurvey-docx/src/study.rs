//! A study as one Word document (specification, "Study builder and Word export"). Each scale keeps its own title,
//! instructions, anchors and numbering; numbering restarts at 1 for each scale unless it runs on; titles print, hide,
//! or give way to neutral labels ("Section 3"); sources print under each scale, together in a References section at
//! the end, or not at all; and a page break, a short transition sentence or a demographic block can sit between scales.
//!
//! Two outputs: the respondent copy (titles or labels, instructions, anchors and items, nothing about scoring) and the
//! researcher copy and scoring key (all of that, plus reverse marks, recode tables, subscales, scoring rules, ranges,
//! sources in APA 7 and any self-check findings still open).

use super::{package, scale_body, Copy, ExportOptions, Exported, P};
use prismsurvey_core::cover_letter::heading;
use prismsurvey_core::demographics::is_choice_line;
use prismsurvey_core::study::{Part, Study, TitleMode};
use prismsurvey_core::{LicenceStatus, SourcePlacement};

const PAGE_BREAK: &str = r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#;

pub fn export_study(study: &Study, copy: Copy) -> Exported {
    let base = ExportOptions {
        copy,
        font: study.options.font.clone(),
        size_pt: study.options.size_pt,
        ..Default::default()
    };
    let offsets = study.number_offsets();
    let mut body = String::new();
    let mut notes = Vec::new();
    let mut references: Vec<String> = Vec::new();
    let mut k = 0usize;
    let researcher = copy == Copy::Researcher;
    // True while nothing has been written on the current page, so a section that starts a new page, a cover letter's
    // own page and "start each scale on a new page" never leave a blank page between them.
    let mut at_page_top = true;
    let page_break = |body: &mut String, at_top: &mut bool| {
        if !*at_top {
            body.push_str(PAGE_BREAK);
            *at_top = true;
        }
    };
    if researcher {
        body.push_str(
            &P::new("Heading1")
                .text(&format!("{}: researcher copy and scoring key", study.title))
                .keep_next()
                .xml(),
        );
    }
    for part in &study.parts {
        match part {
            Part::Scale { scale, .. } => {
                if k > 0 && study.options.break_between_scales {
                    page_break(&mut body, &mut at_page_top);
                }
                at_page_top = false;
                let label = format!("{} {}", study.options.neutral_word.trim(), k + 1);
                let (show_title, neutral) = match (study.options.titles, researcher) {
                    // The researcher copy always names the scale, with its section label beside it.
                    (TitleMode::Neutral, true) => {
                        (false, Some(format!("{label}: {}", scale.title)))
                    }
                    (TitleMode::Hide, true) | (TitleMode::Show, _) => (true, None),
                    (TitleMode::Neutral, false) => (false, Some(label)),
                    (TitleMode::Hide, false) => (false, None),
                };
                let opts = ExportOptions {
                    show_title,
                    neutral_label: neutral,
                    number_offset: offsets[k],
                    sources: Some(study.options.sources),
                    ..base.clone()
                };
                body.push_str(&scale_body(scale, &opts));
                if study.options.sources == SourcePlacement::References {
                    for c in [&scale.source, &scale.adapted_from].into_iter().flatten() {
                        if !references.iter().any(|r| r.eq_ignore_ascii_case(&c.apa)) {
                            references.push(c.apa.clone());
                        }
                    }
                }
                if !researcher
                    && matches!(
                        scale.licence.status,
                        LicenceStatus::Unknown | LicenceStatus::LocalOnly
                    )
                    && scale.source.is_some()
                {
                    let note =
                        "Reproducing a published measure may need the rights holder's permission."
                            .to_string();
                    if !notes.contains(&note) {
                        notes.push(note);
                    }
                }
                k += 1;
            }
            Part::PageBreak => page_break(&mut body, &mut at_page_top),
            Part::Section { title, new_page } => {
                if *new_page {
                    page_break(&mut body, &mut at_page_top);
                }
                if !title.trim().is_empty() && (researcher || study.options.section_headings) {
                    body.push_str(&P::new("Heading1").text(title.trim()).keep_next().xml());
                }
            }
            Part::Transition { text } => {
                if !text.trim().is_empty() {
                    body.push_str(&P::new("PSTransition").text(text.trim()).xml());
                    at_page_top = false;
                }
            }
            Part::CoverLetter { title, text } => {
                if !title.trim().is_empty() {
                    body.push_str(&P::new("Heading1").text(title.trim()).keep_next().xml());
                }
                for line in text.lines().filter(|l| !l.trim().is_empty()) {
                    match heading(line) {
                        Some(h) => {
                            body.push_str(&P::new("PSCoverHeading").text(h).keep_next().xml())
                        }
                        None => body.push_str(&P::new("PSCoverLetter").text(line.trim()).xml()),
                    }
                }
                // The letter has its page to itself: the questions start on the next one.
                at_page_top = false;
                page_break(&mut body, &mut at_page_top);
            }
            Part::Demographics { title, text } => {
                if !title.trim().is_empty() {
                    body.push_str(&P::new("Heading1").text(title.trim()).keep_next().xml());
                }
                // A question keeps its answer choices (lines starting with a tick box) on its page.
                let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
                for (i, line) in lines.iter().enumerate() {
                    let choice = is_choice_line(line);
                    let style = if choice {
                        "PSDemographicChoice"
                    } else {
                        "PSDemographic"
                    };
                    let mut p = P::new(style).text(line.trim());
                    if lines.get(i + 1).is_some_and(|n| is_choice_line(n)) {
                        p = p.keep_next();
                    }
                    body.push_str(&p.xml());
                }
                if !lines.is_empty() {
                    at_page_top = false;
                }
            }
        }
    }
    if !references.is_empty() {
        references.sort_by_key(|r| r.to_lowercase());
        page_break(&mut body, &mut at_page_top);
        body.push_str(&P::new("Heading1").text("References").keep_next().xml());
        for r in references {
            body.push_str(&P::new("PSSource").text(&r).xml());
        }
    }
    // A page break with nothing after it would print an empty last page.
    while body.ends_with(PAGE_BREAK) {
        body.truncate(body.len() - PAGE_BREAK.len());
    }
    if k == 0 {
        notes.push("The study has no scales yet.".into());
    }
    Exported {
        bytes: package(&body, &base),
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::{paragraphs, sample};
    use prismsurvey_core::study::{Numbering, Study};
    use prismsurvey_core::{Citation, Format};

    fn study() -> Study {
        let mut st = Study::new("Spring study");
        let mut a = sample(Format::LikertBlankLine);
        a.title = "Pond Mood Scale".into();
        let mut b = sample(Format::LikertBoxGrid);
        b.title = "Garden Joy Scale".into();
        b.source = Some(Citation {
            apa: "Adams, A. (2019). Gardens. Journal of Soil, 2, 3-4.".into(),
            doi: None,
            reference_manager_id: None,
        });
        st.parts.push(Part::Scale {
            scale: a,
            library_path: None,
        });
        st.parts.push(Part::Transition {
            text: "The next questions ask about gardens.".into(),
        });
        st.parts.push(Part::Scale {
            scale: b,
            library_path: None,
        });
        st.parts.insert(
            0,
            Part::CoverLetter {
                title: "About this survey".into(),
                text: "## Key information\nYou are invited to take part.".into(),
            },
        );
        st.parts.push(Part::Demographics {
            title: "About you".into(),
            text: "What is your age? ______\nWhat is your major? ______\nDo you supervise other employees?\n☐ Yes\n☐ No".into(),
        });
        st
    }

    #[test]
    fn numbering_runs_on_and_titles_become_labels() {
        let mut st = study();
        st.options.numbering = Numbering::RunOn;
        st.options.titles = TitleMode::Neutral;
        let p = paragraphs(&export_study(&st, Copy::Respondent).bytes);
        assert!(
            p.contains(&"Section 1".to_string()) && p.contains(&"Section 2".to_string()),
            "{p:?}"
        );
        assert!(!p.iter().any(|x| x.contains("Pond Mood")));
        assert!(p.iter().any(|x| x.starts_with("_____3.")));
        // The box grid's first row is item 4 of the study.
        assert!(
            p.iter().any(|x| x.starts_with("4.|Statement number 1")),
            "{p:?}"
        );
        assert!(p.contains(&"The next questions ask about gardens.".to_string()));
        assert!(p.contains(&"What is your major? ______".to_string()));
        assert!(p.contains(&"☐ Yes".to_string()), "{p:?}");
        assert!(
            p.contains(&"Key information".to_string()) && !p.iter().any(|x| x.starts_with("## ")),
            "{p:?}"
        );
        assert!(p.contains(&"You are invited to take part.".to_string()));
    }

    #[test]
    fn sections_head_their_parts_and_the_cover_letter_has_its_own_page() {
        let mut st = study();
        st.parts.insert(
            1,
            Part::Section {
                title: "Your garden".into(),
                new_page: true,
            },
        );
        st.options.break_between_scales = true;
        let doc = export_study(&st, Copy::Respondent);
        let p = paragraphs(&doc.bytes);
        assert!(p.contains(&"Your garden".to_string()), "{p:?}");
        let xml = {
            use std::io::Read;
            let mut z = zip::ZipArchive::new(std::io::Cursor::new(&doc.bytes)).unwrap();
            let mut x = String::new();
            z.by_name("word/document.xml")
                .unwrap()
                .read_to_string(&mut x)
                .unwrap();
            x
        };
        // One break after the letter (the section's own new page is the same one), one before the second scale.
        assert_eq!(xml.matches(PAGE_BREAK).count(), 2, "{xml}");
        assert!(!xml.contains(&format!("{PAGE_BREAK}{PAGE_BREAK}")));
        st.options.section_headings = false;
        let p = paragraphs(&export_study(&st, Copy::Respondent).bytes);
        assert!(!p.contains(&"Your garden".to_string()));
        let p = paragraphs(&export_study(&st, Copy::Researcher).bytes);
        assert!(
            p.contains(&"Your garden".to_string()),
            "the researcher copy keeps section headings"
        );
    }

    #[test]
    fn references_are_gathered_at_the_end() {
        let mut st = study();
        st.options.sources = SourcePlacement::References;
        let p = paragraphs(&export_study(&st, Copy::Respondent).bytes);
        let refs = p
            .iter()
            .position(|x| x == "References")
            .expect("a References heading");
        assert!(p[refs + 1..].iter().any(|x| x.starts_with("Adams, A.")));
        assert!(
            p[..refs].iter().all(|x| x != "Source:"),
            "no sources under the scales"
        );
        assert!(p[refs + 1..].iter().any(|x| x.starts_with("Heron, G.")));
    }

    #[test]
    fn the_researcher_copy_names_scales_and_scores_them() {
        let mut st = study();
        st.options.titles = TitleMode::Neutral;
        let p = paragraphs(&export_study(&st, Copy::Researcher).bytes);
        assert!(
            p.contains(&"Section 1: Pond Mood Scale".to_string()),
            "{p:?}"
        );
        assert!(p.iter().any(|x| x.starts_with("Recode items 2")));
    }
}
