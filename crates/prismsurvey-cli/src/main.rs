//! `prismsurvey`: the command-line side of Prism Survey.
//!
//!   prismsurvey import <file> [--out <folder>]           import a stored scale (.txt, .docx, .doc) to .pscale files
//!   prismsurvey export <file.pscale> [--researcher] [--out <file.docx>]
//!   prismsurvey new <blank-line|box-grid|semantic-differential> [--points N] [--out <file.pscale>]
//!   prismsurvey check-folder <folder> [--out <folder>] [--no-recurse]
//!
//! `check-folder` is what `Prism Survey Import Check.bat` runs: it imports every stored scale in a folder and writes,
//! for each, the .pscale file, a respondent copy and a researcher copy in Word, and one report to read beside the
//! originals.

use prismsurvey_core::pscale;
use prismsurvey_core::scale::{computed_range, new_blank, reversed_numbers, set_points};
use prismsurvey_core::*;
use prismsurvey_docx::{export_scale, Copy, ExportOptions};
use prismsurvey_import::stored::import_path;
use prismsurvey_import::ImportReport;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[cfg(feature = "dev-server")]
mod serve;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("import") => cmd_import(&args[1..]),
        Some("export") => cmd_export(&args[1..]),
        Some("new") => cmd_new(&args[1..]),
        Some("check-folder") => cmd_check_folder(&args[1..]),
        Some("detect") => cmd_detect(&args[1..]),
        Some("benchmark") => cmd_benchmark(&args[1..]),
        Some("study") => cmd_study(&args[1..]),
        Some("addon-build") => cmd_addon_build(&args[1..]),
        #[cfg(feature = "dev-server")]
        Some("serve") => serve::run(
            value(&args[1..], "--port")
                .and_then(|p| p.parse().ok())
                .unwrap_or(4892),
        ),
        Some("--help") | Some("-h") | None => {
            println!("{}", HELP);
            Ok(())
        }
        Some(other) => Err(format!("There is no command \"{other}\".\n\n{HELP}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

const HELP: &str = "Prism Survey

  prismsurvey import <file> [--out <folder>]
  prismsurvey export <file.pscale> [--researcher] [--out <file.docx>]
  prismsurvey new <blank-line|box-grid|semantic-differential> [--points N] [--out <file.pscale>]
  prismsurvey check-folder <folder> [--out <folder>] [--no-recurse]";

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn positional(args: &[String]) -> Option<String> {
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if a == "--out" || a == "--points" {
            skip = true;
            continue;
        }
        if !a.starts_with("--") {
            return Some(a.clone());
        }
    }
    None
}

/// A file name safe on Windows.
fn safe_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if r#"<>:"/\|?*"#.contains(c) || (c as u32) < 32 {
                '_'
            } else {
                c
            }
        })
        .collect();
    let t = cleaned.trim().trim_end_matches('.').to_string();
    if t.is_empty() {
        "Untitled scale".into()
    } else {
        t.chars().take(120).collect()
    }
}

fn scale_file_stem(r: &ImportReport, source: &Path) -> String {
    let base = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if r.part > 1 || r.scale.title.is_empty() {
        safe_name(&format!("{base} (scale {})", r.part))
    } else {
        safe_name(&base)
    }
}

fn cmd_import(args: &[String]) -> Result<(), String> {
    let file = positional(args).ok_or("Name a file to import.")?;
    let out = value(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let path = PathBuf::from(&file);
    let reports = import_path(&path).map_err(|e| format!("{file}: {e}"))?;
    for r in reports {
        let mut scale = r.scale.clone();
        let target = out.join(format!(
            "{}.{}",
            scale_file_stem(&r, &path),
            pscale::EXTENSION
        ));
        pscale::save(&mut scale, &target, &format!("Imported from {}", r.file))
            .map_err(|e| e.to_string())?;
        println!("{} -> {}", r.file, target.display());
        for n in &r.notes {
            println!("  note: {n}");
        }
    }
    Ok(())
}

fn cmd_export(args: &[String]) -> Result<(), String> {
    let file = positional(args).ok_or("Name a .pscale file to export.")?;
    let scale = pscale::load(Path::new(&file)).map_err(|e| format!("{file}: {e}"))?;
    let copy = if flag(args, "--researcher") {
        Copy::Researcher
    } else {
        Copy::Respondent
    };
    let out = value(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(&file).with_extension("docx"));
    let done = export_scale(
        &scale,
        &ExportOptions {
            copy,
            ..Default::default()
        },
    );
    std::fs::write(&out, &done.bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    for n in done.notes {
        println!("  note: {n}");
    }
    Ok(())
}

fn cmd_new(args: &[String]) -> Result<(), String> {
    let kind =
        positional(args).ok_or("Name a format: blank-line, box-grid or semantic-differential.")?;
    let format = match kind.as_str() {
        "blank-line" => Format::LikertBlankLine,
        "box-grid" => Format::LikertBoxGrid,
        "semantic-differential" => Format::SemanticDifferential,
        other => {
            return Err(format!(
                "\"{other}\" is not a format: use blank-line, box-grid or semantic-differential."
            ))
        }
    };
    let mut scale = new_blank(format);
    if let Some(p) = value(args, "--points") {
        let p: u8 = p
            .parse()
            .map_err(|_| "--points takes a number from 2 to 11.")?;
        set_points(&mut scale, p);
    }
    let out = value(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("New {} scale.pscale", format.label())));
    pscale::save(&mut scale, &out, "Created from a blank template").map_err(|e| e.to_string())?;
    println!("{}", out.display());
    Ok(())
}

fn is_stored_scale(p: &Path) -> bool {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.starts_with("~$") {
        // Word's lock file for a document that is open.
        return false;
    }
    matches!(
        p.extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .as_deref(),
        Some("doc" | "docx" | "txt")
    )
}

fn collect(dir: &Path, recurse: bool, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if recurse {
                collect(&p, recurse, out);
            }
        } else if is_stored_scale(&p) {
            out.push(p);
        }
    }
}

fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{}", x as i64)
    } else {
        format!("{x:.2}")
    }
}

fn list(n: &[u32]) -> String {
    if n.is_empty() {
        "none".into()
    } else {
        n.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The report section for one scale.
fn describe(r: &ImportReport, out: &mut String) {
    let s = &r.scale;
    let _ = writeln!(
        out,
        "  Title: {}",
        if s.title.is_empty() {
            "(none)"
        } else {
            &s.title
        }
    );
    let _ = writeln!(
        out,
        "  Format: {}, {} points",
        s.format.label(),
        s.response.points
    );
    let labels: Vec<String> = s
        .response
        .labels
        .iter()
        .zip(&s.response.values)
        .map(|(l, v)| format!("{v} = {}", l.clone().unwrap_or_else(|| "(no label)".into())))
        .collect();
    let _ = writeln!(out, "  Anchors: {}", labels.join("; "));
    let _ = writeln!(
        out,
        "  Instructions ({:?}): {}",
        s.instructions.origin,
        if s.instructions.text.is_empty() {
            "(none)"
        } else {
            &s.instructions.text
        }
    );
    let _ = writeln!(out, "  Items: {}", s.items.len());
    let _ = writeln!(out, "  Reverse scored: {}", list(&reversed_numbers(s)));
    if let Some(st) = &s.recode.stated {
        let _ = writeln!(out, "    as the file states it: \"{}\"", st.text);
    }
    if let RecodeKind::Table { pairs } = &s.recode.rule {
        let _ = writeln!(out, "    recode table kept as written: {pairs:?}");
    }
    for sub in &s.subscales {
        let others = if sub.other_names.is_empty() {
            String::new()
        } else {
            format!(" (also called {})", sub.other_names.join(", "))
        };
        let _ = writeln!(
            out,
            "  Subscale {}{}: {}",
            sub.name,
            others,
            list(&sub.items.resolve(s.items.len() as u32))
        );
    }
    if s.scoring.has_total {
        let from = match s.scoring.total_from {
            TotalFrom::Items => "items",
            TotalFrom::Subscales => "subscales",
        };
        let _ = writeln!(
            out,
            "  Total: {} ({:?} of {from})",
            s.scoring
                .total_name
                .clone()
                .unwrap_or_else(|| "(unnamed)".into()),
            s.scoring.total_method
        );
        if !s.scoring.other_total_names.is_empty() {
            let _ = writeln!(
                out,
                "    also called: {}",
                s.scoring.other_total_names.join("; ")
            );
        }
    } else {
        let _ = writeln!(out, "  Total: none");
    }
    let stated = s
        .score_range
        .stated
        .map(|(a, b)| format!("{} to {}", fmt_num(a), fmt_num(b)))
        .unwrap_or_else(|| "not stated".into());
    let computed = computed_range(s)
        .map(|(a, b)| format!("{} to {}", fmt_num(a), fmt_num(b)))
        .unwrap_or_else(|| "none".into());
    let _ = writeln!(out, "  Score range: stated {stated}; computed {computed}");
    if let Some(c) = &s.source {
        let _ = writeln!(out, "  Source: {}", c.apa);
    }
    if let Some(c) = &s.adapted_from {
        let _ = writeln!(out, "  Adapted from: {}", c.apa);
    }
    if let Some(d) = &s.development {
        let kept = d
            .items
            .iter()
            .filter(|i| i.marked_kept == Some(true))
            .count();
        let mapped = d.items.iter().filter(|i| i.final_number.is_some()).count();
        let _ = writeln!(
            out,
            "  Development table: {} items, factors {}; {} marked kept (bold), {} matched to final items by wording",
            d.items.len(),
            if d.factors.is_empty() { "(unnamed)".into() } else { d.factors.join(", ") },
            kept,
            mapped
        );
        for di in d
            .items
            .iter()
            .filter(|i| i.final_number.is_some_and(|f| f != i.number))
        {
            let _ = writeln!(
                out,
                "    development item {} is final item {}",
                di.number,
                di.final_number.unwrap()
            );
        }
    }
    for n in &r.notes {
        let _ = writeln!(out, "  NOTE: {n}");
    }
    for u in &s.unparsed {
        let _ = writeln!(out, "  NOT UNDERSTOOD (paragraph {}): {}", u.line, u.text);
    }
    for f in prismsurvey_core::selfcheck::self_check(s) {
        let level = match f.severity {
            prismsurvey_core::selfcheck::Severity::Warning => "CHECK",
            prismsurvey_core::selfcheck::Severity::Note => "check note",
        };
        let _ = writeln!(out, "  {level} [{}]: {}", f.check_label, f.message);
    }
}

fn cmd_check_folder(args: &[String]) -> Result<(), String> {
    let dir = PathBuf::from(positional(args).ok_or("Name the folder to check.")?);
    if !dir.is_dir() {
        return Err(format!("{} is not a folder.", dir.display()));
    }
    let stamp = pscale::now_rfc3339()
        .replace(':', "-")
        .replace('T', " ")
        .trim_end_matches('Z')
        .to_string();
    let out = value(args, "--out").map(PathBuf::from).unwrap_or_else(|| {
        let docs = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Documents");
        docs.join("Prism Survey Import Check").join(&stamp)
    });
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut files = Vec::new();
    collect(&dir, !flag(args, "--no-recurse"), &mut files);

    let mut report = String::new();
    let _ = writeln!(report, "Prism Survey import check");
    let _ = writeln!(report, "Folder: {}", dir.display());
    let _ = writeln!(report, "Run: {stamp} UTC");
    let _ = writeln!(report, "Files: {}\n", files.len());
    let _ = writeln!(
        report,
        "For each file there is a .pscale file, a respondent copy and a researcher copy in Word, in the folder named after it.\nThe researcher copy is the one to set beside the original: it carries the scoring lines and sources the stored files have.\n"
    );
    let (mut ok, mut failed, mut scales) = (0, 0, 0);
    for f in &files {
        let rel = f.strip_prefix(&dir).unwrap_or(f).display().to_string();
        let _ = writeln!(report, "==== {rel}");
        match import_path(f) {
            Ok(reports) => {
                ok += 1;
                let sub = out.join(safe_name(&rel.replace(['\\', '/'], " - ")));
                for r in reports {
                    scales += 1;
                    if r.part > 1 || reports_len_hint(&r) {
                        let _ = writeln!(report, "  -- scale {}", r.part);
                    }
                    describe(&r, &mut report);
                    let stem = scale_file_stem(&r, f);
                    let mut scale = r.scale.clone();
                    let target = sub.join(format!("{stem}.{}", pscale::EXTENSION));
                    if let Err(e) =
                        pscale::save(&mut scale, &target, &format!("Imported from {}", r.file))
                    {
                        let _ = writeln!(report, "  COULD NOT SAVE: {e}");
                        continue;
                    }
                    for (copy, label) in [
                        (Copy::Respondent, "respondent"),
                        (Copy::Researcher, "researcher"),
                    ] {
                        let done = export_scale(
                            &scale,
                            &ExportOptions {
                                copy,
                                ..Default::default()
                            },
                        );
                        let path = sub.join(format!("{stem} - {label} copy.docx"));
                        if let Err(e) = std::fs::write(&path, &done.bytes) {
                            let _ = writeln!(report, "  COULD NOT WRITE {}: {e}", path.display());
                        }
                    }
                }
            }
            Err(e) => {
                failed += 1;
                let _ = writeln!(report, "  COULD NOT READ: {e}");
            }
        }
        let _ = writeln!(report);
    }
    let _ = writeln!(
        report,
        "Read {ok} of {} files ({failed} could not be read); {scales} scales found.",
        files.len()
    );
    let report_path = out.join("Import check report.txt");
    // A byte order mark so Notepad and Word open it as UTF-8.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(report.replace('\n', "\r\n").into_bytes());
    std::fs::write(&report_path, bytes).map_err(|e| format!("{}: {e}", report_path.display()))?;
    println!("{}", out.display());
    println!(
        "Read {ok} of {} files; {scales} scales found. The report is \"Import check report.txt\".",
        files.len()
    );
    Ok(())
}

/// Finds measures in journal articles and prints what was found: `prismsurvey detect <file or folder>...`
fn cmd_detect(args: &[String]) -> Result<(), String> {
    let mut files = Vec::new();
    for a in args.iter().filter(|a| !a.starts_with("--")) {
        let p = PathBuf::from(a);
        if p.is_dir() {
            let mut v: Vec<PathBuf> = std::fs::read_dir(&p)
                .map_err(|e| e.to_string())?
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().is_some_and(|e| {
                        matches!(
                            e.to_string_lossy().to_ascii_lowercase().as_str(),
                            "pdf" | "png" | "jpg" | "jpeg"
                        )
                    })
                })
                .collect();
            v.sort();
            files.extend(v);
        } else {
            files.push(p);
        }
    }
    let verbose = flag(args, "--items");
    for f in files {
        match prismsurvey_detect::measure::detect(&f) {
            Ok(d) => {
                println!("==== {} ({} candidates)", d.file, d.candidates.len());
                for n in &d.notes {
                    println!("  note: {n}");
                }
                for (c, s) in d.candidates.iter().zip(&d.scales).take(3) {
                    println!(
                        "  [{:.1}] p{} {} | {} items, {} pts, reversed {:?}, subscales {}{}",
                        c.score,
                        c.page,
                        s.title,
                        s.items.len(),
                        s.response.points,
                        prismsurvey_core::scale::reversed_numbers(s)
                            .into_iter()
                            .chain(
                                s.items
                                    .iter()
                                    .filter(|i| i.reverse.state == ReverseState::Suggested)
                                    .map(|i| i.number)
                            )
                            .collect::<Vec<_>>(),
                        s.subscales.len(),
                        if c.sample_only {
                            " (sample items only)"
                        } else {
                            ""
                        }
                    );
                    if verbose {
                        for it in s.items.iter().take(40) {
                            let t = if it.text.is_empty() {
                                format!(
                                    "{} / {}",
                                    it.left.clone().unwrap_or_default(),
                                    it.right.clone().unwrap_or_default()
                                )
                            } else {
                                it.text.clone()
                            };
                            println!(
                                "      {}. {}",
                                it.number,
                                t.chars().take(110).collect::<String>()
                            );
                        }
                    }
                }
            }
            Err(e) => println!("==== {}: {e}", f.display()),
        }
    }
    Ok(())
}

/// Runs detection over folders, compares with the answer keys and writes the report:
/// `prismsurvey benchmark <folder>... [--keys <folder>] [--out <file>]`.
fn cmd_benchmark(args: &[String]) -> Result<(), String> {
    let keys = value(args, "--keys")
        .map(PathBuf::from)
        .unwrap_or_else(prismsurvey_detect::benchmark::default_key_folder);
    let mut folders = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if a == "--keys" || a == "--out" {
            skip = true;
            continue;
        }
        folders.push(PathBuf::from(a));
    }
    let mut files = Vec::new();
    for f in &folders {
        let mut v: Vec<PathBuf> = std::fs::read_dir(f)
            .map_err(|e| format!("{}: {e}", f.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                !name.starts_with("~$")
                    && p.extension().is_some_and(|e| {
                        matches!(
                            e.to_string_lossy().to_ascii_lowercase().as_str(),
                            "pdf" | "png" | "jpg" | "jpeg" | "doc" | "docx" | "txt"
                        )
                    })
            })
            .collect();
        v.sort();
        files.extend(v);
    }
    let stamp = pscale::now_rfc3339()
        .replace(':', "-")
        .replace('T', " ")
        .trim_end_matches('Z')
        .to_string();
    let out = value(args, "--out").map(PathBuf::from).unwrap_or_else(|| {
        let docs = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Documents");
        docs.join("Prism Survey Benchmark")
            .join(format!("Benchmark {stamp}.txt"))
    });
    println!(
        "Checking {} files against the answer keys in {}...",
        files.len(),
        keys.display()
    );
    let results = prismsurvey_detect::benchmark::run(&files, &keys);
    let report = prismsurvey_detect::benchmark::report(&results, &format!("{stamp} UTC"));
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(report.as_bytes());
    std::fs::write(&out, bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    Ok(())
}

/// Builds a study from scale files and writes both copies:
/// `prismsurvey study <out folder> <scale file>... [--run-on] [--neutral] [--references] [--breaks]`.
/// Each scale file can be a .pscale or a stored scale (.doc, .docx, .txt), which is imported first.
fn cmd_study(args: &[String]) -> Result<(), String> {
    use prismsurvey_core::study::{Numbering, Part, Study, TitleMode};
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let (out, inputs) = files
        .split_first()
        .ok_or("Name an output folder and the scale files.")?;
    let out = PathBuf::from(out);
    let mut st = Study::new("Study");
    for f in inputs {
        let p = PathBuf::from(f);
        if p.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(pscale::EXTENSION))
        {
            let s = pscale::load(&p).map_err(|e| format!("{f}: {e}"))?;
            st.parts.push(Part::Scale {
                scale: s,
                library_path: Some(p.display().to_string()),
            });
        } else {
            for r in import_path(&p).map_err(|e| format!("{f}: {e}"))? {
                st.parts.push(Part::Scale {
                    scale: r.scale,
                    library_path: None,
                });
            }
        }
    }
    if flag(args, "--run-on") {
        st.options.numbering = Numbering::RunOn;
    }
    if flag(args, "--neutral") {
        st.options.titles = TitleMode::Neutral;
    }
    if flag(args, "--references") {
        st.options.sources = SourcePlacement::References;
    }
    st.options.break_between_scales = flag(args, "--breaks");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    for (copy, name) in [
        (Copy::Respondent, "Study - respondent copy.docx"),
        (Copy::Researcher, "Study - researcher copy.docx"),
    ] {
        let done = prismsurvey_docx::study::export_study(&st, copy);
        let path = out.join(name);
        std::fs::write(&path, &done.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
        for n in done.notes {
            println!("  note: {n}");
        }
    }
    let mut st2 = st.clone();
    prismsurvey_core::study::save_study(
        &mut st2,
        &out.join("Study.pstudy"),
        "Built from the command line",
    )
    .map_err(|e| e.to_string())?;
    let (items, minutes) = st.length();
    println!(
        "{} scales, {items} items, about {minutes} minutes.",
        st.scales().count()
    );
    Ok(())
}

/// Converts the Directory of Communication Related Mental Measures into the Comm Add On's data:
/// `prismsurvey addon-build <book.doc> <directory.json>`.
fn cmd_addon_build(args: &[String]) -> Result<(), String> {
    let book = args
        .first()
        .ok_or("Name the book file and the output file.")?;
    let out = args.get(1).ok_or("Name the output file.")?;
    let doc = prismsurvey_import::document::read_path(Path::new(book))
        .map_err(|e| format!("{book}: {e}"))?;
    let mut d = prismsurvey_import::directory::convert(&doc);
    // The one-time Crossref lookup (tools/crossref_lookup.py), kept beside the data so a rebuild keeps it.
    let lookup_path = Path::new(out).with_file_name("doi-lookup.json");
    let mut applied = (0, 0, 0);
    if let Ok(text) = std::fs::read_to_string(&lookup_path) {
        let lookups: std::collections::HashMap<String, prismsurvey_core::directory::Lookup> =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", lookup_path.display()))?;
        for e in &mut d.entries {
            if let Some(l) = lookups.get(&e.id) {
                e.apply_lookup(l);
                let accepted = e
                    .reference_check
                    .as_deref()
                    .is_some_and(|c| c.contains("first page"));
                match if accepted {
                    "matched"
                } else {
                    l.status.as_str()
                } {
                    "matched" => applied.0 += 1,
                    "review" => applied.1 += 1,
                    _ => applied.2 += 1,
                }
            }
        }
    }
    std::fs::write(out, serde_json::to_string_pretty(&d).unwrap_or_default())
        .map_err(|e| format!("{out}: {e}"))?;
    if applied != (0, 0, 0) {
        println!(
            "Crossref: {} matched (DOI and issue applied), {} to check, {} with no match.",
            applied.0, applied.1, applied.2
        );
    }
    let mut cats: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for e in &d.entries {
        for c in &e.categories {
            *cats.entry(c.clone()).or_default() += 1;
        }
    }
    println!("{} entries written to {out}", d.entries.len());
    for (field, missing) in [
        (
            "purpose",
            d.entries.iter().filter(|e| e.purpose.is_empty()).count(),
        ),
        (
            "number of items",
            d.entries
                .iter()
                .filter(|e| e.number_of_items.is_empty())
                .count(),
        ),
        (
            "source",
            d.entries.iter().filter(|e| e.source.is_empty()).count(),
        ),
        (
            "categories",
            d.entries.iter().filter(|e| e.categories.is_empty()).count(),
        ),
    ] {
        println!("  without {field}: {missing}");
    }
    println!(
        "  with a DOI: {}",
        d.entries.iter().filter(|e| e.doi.is_some()).count()
    );
    println!("Categories:");
    for (c, n) in cats {
        println!("  {n:4} {c}");
    }
    Ok(())
}

fn reports_len_hint(r: &ImportReport) -> bool {
    r.notes
        .first()
        .is_some_and(|n| n.starts_with("This file holds"))
}
