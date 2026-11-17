//! One scale as HTML for the clipboard (specification: "copied to the clipboard ready to paste into an open Word
//! document"). Word keeps HTML tables as tables when pasting, so the grids stay grids; the type follows the document
//! pasted into.

use super::Copy;
use prismsurvey_core::instructions::box_grid_key;
use prismsurvey_core::scale::{computed_range, reversed_numbers};
use prismsurvey_core::*;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn scale_html(scale: &Scale, copy: Copy) -> String {
    let researcher = copy == Copy::Researcher;
    let mut h = String::from("<div>");
    if !scale.title.trim().is_empty() {
        h.push_str(&format!(
            "<p style=\"text-align:center\"><b>{}</b></p>",
            esc(&scale.title)
        ));
    }
    if !scale.instructions.text.trim().is_empty() {
        h.push_str(&format!("<p>{}</p>", esc(&scale.instructions.text)));
    }
    let r = &scale.response;
    let label = |i: usize| r.labels.get(i).cloned().flatten().unwrap_or_default();
    let mark = |it: &Item| {
        if researcher && it.reverse.is_reversed() {
            " (R)"
        } else {
            ""
        }
    };
    let td = "style=\"padding:2px 6px\"";
    let box_td = "style=\"border:1px solid black;width:28px;height:18px\"";
    match scale.format {
        Format::LikertBlankLine => {
            h.push_str("<table style=\"margin:auto\"><tr>");
            for i in 0..r.points as usize {
                h.push_str(&format!(
                    "<td {td} align=\"center\">{}</td>",
                    esc(&label(i))
                ));
            }
            h.push_str("</tr><tr>");
            for v in &r.values {
                h.push_str(&format!("<td {td} align=\"center\">{v}</td>"));
            }
            h.push_str("</tr></table>");
            for it in &scale.items {
                h.push_str(&format!(
                    "<p>_____{}. {}{}</p>",
                    it.number,
                    esc(&it.text),
                    mark(it)
                ));
            }
        }
        Format::LikertBoxGrid => {
            let key = box_grid_key(r);
            if !key.is_empty() {
                h.push_str(&format!("<p>{}</p>", esc(&key)));
            }
            h.push_str("<table><tr><td></td><td></td>");
            for (i, a) in r.abbreviations.iter().enumerate() {
                let a = if a.is_empty() {
                    r.values[i].to_string()
                } else {
                    a.clone()
                };
                h.push_str(&format!(
                    "<td {td} align=\"center\"><b>{}</b></td>",
                    esc(&a)
                ));
            }
            h.push_str("</tr>");
            for it in &scale.items {
                h.push_str(&format!(
                    "<tr><td {td}>{}.</td><td {td}>{}{}</td>",
                    it.number,
                    esc(&it.text),
                    mark(it)
                ));
                for _ in 0..r.points {
                    h.push_str(&format!("<td {box_td}></td>"));
                }
                h.push_str("</tr>");
            }
            h.push_str("</table>");
        }
        Format::SemanticDifferential => {
            h.push_str("<table>");
            for it in &scale.items {
                h.push_str(&format!(
                    "<tr><td {td}>{}.{}</td><td {td} align=\"right\">{}</td>",
                    it.number,
                    mark(it),
                    esc(it.left.as_deref().unwrap_or(""))
                ));
                for v in &r.values {
                    h.push_str(&format!("<td {td} align=\"center\">{v}</td>"));
                }
                h.push_str(&format!(
                    "<td {td}>{}</td></tr>",
                    esc(it.right.as_deref().unwrap_or(""))
                ));
            }
            h.push_str("</table>");
        }
    }
    if researcher {
        let rev = reversed_numbers(scale);
        h.push_str("<p><b>Scoring</b></p>");
        if rev.is_empty() {
            h.push_str("<p>No items are reverse scored.</p>");
        } else {
            let pairs: Vec<String> = prismsurvey_core::scale::recode_table(scale)
                .iter()
                .map(|(a, b)| format!("{a} = {b}"))
                .collect();
            h.push_str(&format!(
                "<p>Recode items {}: {}.</p>",
                rev.iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                pairs.join(", ")
            ));
        }
        for s in &scale.subscales {
            let items = s.items.resolve(scale.items.len() as u32);
            h.push_str(&format!(
                "<p>{}: items {}.</p>",
                esc(&s.name),
                items
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some((a, b)) = computed_range(scale) {
            h.push_str(&format!("<p>Scores should range between {a} and {b}.</p>"));
        }
    }
    if scale.source_placement == SourcePlacement::UnderScale {
        if let Some(c) = &scale.source {
            h.push_str(&format!(
                "<p><b>Source:</b></p><p style=\"margin-left:36pt;text-indent:-36pt\">{}</p>",
                esc(&c.apa)
            ));
        }
        if let Some(c) = &scale.adapted_from {
            h.push_str(&format!(
                "<p><b>Adapted from:</b></p><p style=\"margin-left:36pt;text-indent:-36pt\">{}</p>",
                esc(&c.apa)
            ));
        }
    }
    h.push_str("</div>");
    h
}

/// The same scale as plain text, for programs that do not take HTML.
pub fn scale_text(scale: &Scale) -> String {
    let mut t = String::new();
    t.push_str(&scale.title);
    t.push_str("\n\n");
    if !scale.instructions.text.is_empty() {
        t.push_str(&scale.instructions.text);
        t.push_str("\n\n");
    }
    for it in &scale.items {
        match (&it.left, &it.right) {
            (Some(l), Some(r)) if it.text.is_empty() => t.push_str(&format!(
                "{}. {l}  {}  {r}\n",
                it.number,
                scale
                    .response
                    .values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            )),
            _ => t.push_str(&format!("_____{}. {}\n", it.number, it.text)),
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::sample;

    #[test]
    fn html_keeps_grids_as_tables() {
        let h = scale_html(&sample(Format::LikertBoxGrid), Copy::Respondent);
        assert!(h.contains("<table>") && h.contains("border:1px solid black"));
        assert!(h.contains("Statement number 2 &amp; more."));
        assert!(!h.contains("(R)"));
        let r = scale_html(&sample(Format::LikertBlankLine), Copy::Researcher);
        assert!(r.contains("more. (R)") && r.contains("Recode items 2"));
    }
}
