//! The command layer. The window calls `Api::call(cmd, args)` with JSON and gets JSON back; the tests call the same
//! function, so what ships is what was tested. The window holds the scale being edited and sends it with each command;
//! nothing is written to disk except by `scale.save`, `scale.export` and `settings.set`.
//!
//! Commands (see docs/API.md):
//!   scale.new { format }                         -> Scale
//!   scale.import { path }                        -> [ImportReport]
//!   scale.open { path }                          -> Scale
//!   scale.save { path, scale, summary }          -> Scale (with its new history entry)
//!   scale.setFormat { scale, format }            -> { scale, outcome }
//!   scale.setPoints { scale, points }            -> { scale, outcome }
//!   scale.setAnchors { scale, anchorSet, labelMode } -> { scale, outcome }
//!   scale.editInstructions { scale, text }       -> Scale
//!   scale.useHouseInstructions { scale, stem? }  -> Scale
//!   scale.parseSelector { text }                 -> ItemSelector
//!   scale.check { scale }                        -> Check
//!   scale.export { scale, path, copy }           -> { notes }
//!   library.list { folder? }                     -> [LibraryEntry]
//!   settings.get                                 -> Settings
//!   settings.set { settings }                    -> Settings

pub mod refman;

use prismsurvey_core::formats::response_scale;
use prismsurvey_core::instructions::{fit_findings, is_forced_choice, FitFinding};
use prismsurvey_core::recommend::Measure;
use prismsurvey_core::scale::{
    self, computed_range, filled_items, sync_title_count, title_count, ChangeOutcome,
};
use prismsurvey_core::study::{load_study, refresh_from_library, save_study, Part, Study};
use prismsurvey_core::{
    pscale, selfcheck, tags, AnchorSet, DecisionStatus, Format, ItemSelector, LabelMode, Scale,
};
use prismsurvey_docx::{export_scale, Copy, ExportOptions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        ApiError {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// The folder of .pscale files (the personal library).
    pub library_folder: String,
}

impl Settings {
    fn default_for(home: &Path) -> Self {
        Settings {
            library_folder: home
                .join("Documents")
                .join("Prism Survey Library")
                .display()
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub path: String,
    pub title: String,
    pub format: Format,
    pub items: usize,
    pub tags: Vec<String>,
    pub updated: Option<String>,
    pub error: Option<String>,
    /// For a variant: the scales it comes from, nearest first ("Communication Apprehension with Physicians Scale").
    /// A parent not in the library is named from the variant's own record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variant_chain: Vec<String>,
}

/// The parent of a variant, found in the library by its id.
fn find_parent(folder: &str, parent_id: &str) -> Option<(PathBuf, Scale)> {
    library_files(folder).into_iter().find_map(|p| {
        let s = pscale::load(&p).ok()?;
        (s.id == parent_id).then_some((p, s))
    })
}

/// The `.pscale` files in a library folder, sorted.
fn library_files(folder: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case(pscale::EXTENSION))
                })
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    paths
}

/// What the confirm form shows live while a scale is edited (phase 2 adds the full self-check).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub computed_range: Option<(f64, f64)>,
    pub instruction_findings: Vec<FitFinding>,
    pub forced_choice: bool,
    /// The count in the title, when there is one and it differs from the item count.
    pub title_count_mismatch: Option<(u32, usize)>,
    pub filled_items: usize,
    /// The scoring self-check, with the scale's decisions applied.
    pub findings: Vec<selfcheck::Finding>,
    /// The scale's source notes, each with the line that prints for it.
    pub source_notes: Vec<NoteLine>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteLine {
    pub key: String,
    pub subject: String,
    pub items: Vec<u32>,
    pub handling: prismsurvey_core::model::SourceNoteHandling,
    pub text: String,
}

pub struct Api {
    settings_path: PathBuf,
    home: PathBuf,
    /// The article open on the confirm screen, read once.
    article: Option<(String, prismsurvey_detect::measure::Prepared)>,
    /// Library files as last read, kept until the file changes, so searching does not read every file each time.
    search_cache:
        std::collections::HashMap<PathBuf, (std::time::SystemTime, prismsurvey_core::search::Doc)>,
    /// The Comm Add On, read once when first asked for.
    directory: Option<prismsurvey_core::directory::Directory>,
}

/// Where add-ons are looked for, in order: the person's add-ons folder, beside the program (the installer puts them
/// there), and the repository's own folder when running from source.
fn addon_folders(config: &Path) -> Vec<PathBuf> {
    let mut v = vec![config.join("Add-ons")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join("addons"));
            v.push(dir.join("resources").join("addons"));
        }
    }
    v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../addons"));
    v
}

/// Page pictures go to the window as data URLs.
/// A scale as the recommender sees it: known by its library file, and by its Comm Directory entry when it has one.
fn scale_measure(s: &Scale, library_path: Option<&str>) -> Measure {
    let key = library_path
        .map(str::to_string)
        .unwrap_or_else(|| format!("scale:{}", s.id));
    let mut aliases = vec![format!("scale:{}", s.id)];
    if let Some(d) = &s.directory_entry {
        aliases.push(format!("dir:{d}"));
    }
    aliases.retain(|a| *a != key);
    Measure {
        key,
        title: s.title.clone(),
        collection: "library".into(),
        tags: s.tags.clone(),
        items: s.items.len(),
        format: Some(s.format),
        points: Some(s.response.points),
        item_text: s.items.iter().map(|i| i.text.clone()).collect(),
        aliases,
    }
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// A file name safe on Windows.
fn safe_file_name(s: &str) -> String {
    s.chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect()
}

fn arg<T: for<'de> Deserialize<'de>>(args: &Value, name: &str) -> Result<T, ApiError> {
    let v = args.get(name).cloned().unwrap_or(Value::Null);
    serde_json::from_value(v).map_err(|e| ApiError::new("bad_argument", format!("{name}: {e}")))
}

fn opt_arg<T: for<'de> Deserialize<'de>>(args: &Value, name: &str) -> Result<Option<T>, ApiError> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|e| ApiError::new("bad_argument", format!("{name}: {e}"))),
    }
}

fn to_value<T: Serialize>(v: T) -> Result<Value, ApiError> {
    serde_json::to_value(v).map_err(|e| ApiError::new("internal", e.to_string()))
}

fn with_outcome(scale: Scale, outcome: ChangeOutcome) -> Result<Value, ApiError> {
    Ok(json!({ "scale": to_value(scale)?, "outcome": to_value(outcome)? }))
}

/// Reads "1, 3, 5", "1-5", "odd" or "even" as an item selector.
pub fn parse_selector(text: &str) -> Result<ItemSelector, ApiError> {
    let t = text.trim().to_ascii_lowercase();
    if t == "odd" || t == "odd items" {
        return Ok(ItemSelector::Odd);
    }
    if t == "even" || t == "even items" {
        return Ok(ItemSelector::Even);
    }
    let parts: Vec<&str> = t
        .split([',', '&', ';'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() == 1 {
        if let Some((a, b)) = parts[0].split_once('-') {
            let (a, b) = (a.trim().parse::<u32>(), b.trim().parse::<u32>());
            if let (Ok(a), Ok(b)) = (a, b) {
                if a <= b {
                    return Ok(ItemSelector::Block { first: a, last: b });
                }
            }
        }
    }
    let mut numbers = Vec::new();
    for p in parts {
        let p = p.trim_start_matches("and ").trim();
        if let Some((a, b)) = p.split_once('-') {
            let (a, b) = (a.trim().parse::<u32>(), b.trim().parse::<u32>());
            match (a, b) {
                (Ok(a), Ok(b)) if a <= b => numbers.extend(a..=b),
                _ => {
                    return Err(ApiError::new(
                        "bad_selector",
                        format!("\"{p}\" is not a range of item numbers"),
                    ))
                }
            }
        } else {
            numbers.push(p.parse().map_err(|_| {
                ApiError::new("bad_selector", format!("\"{p}\" is not an item number"))
            })?);
        }
    }
    Ok(ItemSelector::List { numbers })
}

impl Api {
    /// The Comm Add On, from the first add-on folder that has it.
    fn directory(&mut self) -> Result<&prismsurvey_core::directory::Directory, ApiError> {
        if self.directory.is_none() {
            let config = self
                .settings_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default();
            let found = addon_folders(&config)
                .into_iter()
                .map(|d| d.join("comm-directory").join("directory.json"))
                .find(|p| p.is_file());
            let Some(path) = found else {
                return Err(ApiError::new(
                    "no_addon",
                    "The Comm Add On is not installed.",
                ));
            };
            let text = std::fs::read_to_string(&path)
                .map_err(|e| ApiError::new("addon_failed", e.to_string()))?;
            self.directory = Some(
                serde_json::from_str(text.trim_start_matches('\u{feff}'))
                    .map_err(|e| ApiError::new("addon_failed", e.to_string()))?,
            );
        }
        Ok(self.directory.as_ref().expect("just read"))
    }

    fn config_dir(&self) -> PathBuf {
        self.settings_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }

    /// The measures of the researcher's saved studies, by study id, for the recommender's "your past studies" basis.
    fn study_history(&self) -> Vec<(String, Vec<String>)> {
        std::fs::read_to_string(self.config_dir().join("study-history.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn record_study(&self, st: &Study) {
        let keys: Vec<String> = st
            .parts
            .iter()
            .filter_map(|p| match p {
                Part::Scale {
                    scale,
                    library_path,
                } => {
                    let m = scale_measure(scale, library_path.as_deref());
                    Some(std::iter::once(m.key).chain(m.aliases).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect();
        let mut h = self.study_history();
        h.retain(|(id, _)| *id != st.id);
        if !keys.is_empty() {
            h.push((st.id.clone(), keys));
        }
        let dir = self.config_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(
            dir.join("study-history.json"),
            serde_json::to_string_pretty(&h).unwrap_or_default(),
        );
    }

    fn refman(&self) -> refman::RefMan {
        let config = self
            .settings_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        refman::RefMan::new(&config, refman::DEFAULT_PORT)
    }

    /// The settings file lives in `config_dir`; `home` is the person's home folder (for the default library).
    pub fn new(config_dir: &Path, home: &Path) -> Self {
        Api {
            settings_path: config_dir.join("settings.json"),
            home: home.to_path_buf(),
            article: None,
            search_cache: std::collections::HashMap::new(),
            directory: None,
        }
    }

    /// From the environment: settings under %APPDATA%\Prism Survey, the library under Documents.
    pub fn from_env() -> Self {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let config = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("Prism Survey");
        Api::new(&config, &home)
    }

    fn settings(&self) -> Settings {
        std::fs::read_to_string(&self.settings_path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| Settings::default_for(&self.home))
    }

    pub fn call(&mut self, cmd: &str, args: Value) -> Result<Value, ApiError> {
        match cmd {
            "scale.new" => {
                let format: Format = arg(&args, "format")?;
                to_value(scale::new_blank(format))
            }
            "scale.import" => {
                let path: String = arg(&args, "path")?;
                let reports = prismsurvey_import::import_path(Path::new(&path))
                    .map_err(|e| ApiError::new("import_failed", e.to_string()))?;
                to_value(reports)
            }
            "scale.open" => {
                let path: String = arg(&args, "path")?;
                to_value(
                    pscale::load(Path::new(&path))
                        .map_err(|e| ApiError::new("open_failed", e.to_string()))?,
                )
            }
            "scale.save" => {
                let path: String = arg(&args, "path")?;
                let mut s: Scale = arg(&args, "scale")?;
                let summary: String = opt_arg(&args, "summary")?.unwrap_or_else(|| "Saved".into());
                sync_title_count(&mut s);
                s.tags = tags::normalise(&s.tags);
                pscale::save(&mut s, Path::new(&path), &summary)
                    .map_err(|e| ApiError::new("save_failed", e.to_string()))?;
                to_value(s)
            }
            "scale.setFormat" => {
                let mut s: Scale = arg(&args, "scale")?;
                let format: Format = arg(&args, "format")?;
                let out = scale::set_format(&mut s, format);
                with_outcome(s, out)
            }
            "scale.setPoints" => {
                let mut s: Scale = arg(&args, "scale")?;
                let points: u8 = arg(&args, "points")?;
                let out = scale::set_points(&mut s, points);
                with_outcome(s, out)
            }
            "scale.setAnchors" => {
                let mut s: Scale = arg(&args, "scale")?;
                let set: AnchorSet = arg(&args, "anchorSet")?;
                let mode: LabelMode = arg(&args, "labelMode")?;
                let r = response_scale(s.response.points, set, mode);
                let out = scale::set_response(&mut s, r);
                with_outcome(s, out)
            }
            "scale.editInstructions" => {
                let mut s: Scale = arg(&args, "scale")?;
                let text: String = arg(&args, "text")?;
                scale::edit_instructions(&mut s, text);
                to_value(s)
            }
            "scale.useHouseInstructions" => {
                let mut s: Scale = arg(&args, "scale")?;
                let stem: Option<String> = opt_arg(&args, "stem")?;
                scale::use_house_instructions(&mut s, stem);
                to_value(s)
            }
            "scale.parseSelector" => {
                let text: String = arg(&args, "text")?;
                to_value(parse_selector(&text)?)
            }
            "scale.check" => {
                let s: Scale = arg(&args, "scale")?;
                let count = s.items.len();
                let check = Check {
                    computed_range: computed_range(&s),
                    instruction_findings: fit_findings(&s.instructions.text, s.format, &s.response),
                    forced_choice: is_forced_choice(&s.response),
                    title_count_mismatch: title_count(&s.title)
                        .filter(|c| *c as usize != count)
                        .map(|c| (c, count)),
                    filled_items: filled_items(&s),
                    findings: selfcheck::self_check(&s),
                    source_notes: s
                        .source_notes
                        .iter()
                        .map(|n| NoteLine {
                            key: n.key.clone(),
                            subject: n.subject.clone(),
                            items: n.items.clone(),
                            handling: n.handling,
                            text: selfcheck::note_text(n),
                        })
                        .collect(),
                };
                to_value(check)
            }
            "scale.decideFinding" => {
                // status: "accepted", "dismissed", or null to reopen.
                let mut s: Scale = arg(&args, "scale")?;
                let key: String = arg(&args, "key")?;
                let status: Option<DecisionStatus> = opt_arg(&args, "status")?;
                let reason: String = opt_arg(&args, "reason")?.unwrap_or_default();
                selfcheck::decide(&mut s, &key, status, &reason, &pscale::now_rfc3339())
                    .map_err(|m| ApiError::new("reason_needed", m))?;
                to_value(s)
            }
            "scale.keepAsPublished" => {
                let mut s: Scale = arg(&args, "scale")?;
                let key: String = arg(&args, "key")?;
                let original: String = arg(&args, "original")?;
                let page: Option<String> = opt_arg(&args, "page")?;
                selfcheck::keep_as_published(
                    &mut s,
                    &key,
                    &original,
                    page.as_deref(),
                    &pscale::now_rfc3339(),
                )
                .map_err(|m| ApiError::new("note_needed", m))?;
                to_value(s)
            }
            "scale.correctFromSource" => {
                let mut s: Scale = arg(&args, "scale")?;
                let key: String = arg(&args, "key")?;
                let fix: selfcheck::Fix = arg(&args, "fix")?;
                let original: String = arg(&args, "original")?;
                let page: Option<String> = opt_arg(&args, "page")?;
                selfcheck::correct_from_source(
                    &mut s,
                    &key,
                    &fix,
                    &original,
                    page.as_deref(),
                    &pscale::now_rfc3339(),
                )
                .map_err(|m| ApiError::new("note_needed", m))?;
                to_value(s)
            }
            "scale.removeSourceNote" => {
                let mut s: Scale = arg(&args, "scale")?;
                let key: String = arg(&args, "key")?;
                selfcheck::remove_source_note(&mut s, &key);
                to_value(s)
            }
            "variant.draft" => {
                // A variant of a saved scale, drafted by replacing the target words (no model needed).
                let path: String = arg(&args, "path")?;
                let req: prismsurvey_core::variant::Request = arg(&args, "request")?;
                let parent = pscale::load(Path::new(&path))
                    .map_err(|e| ApiError::new("open_failed", e.to_string()))?;
                to_value(
                    prismsurvey_core::variant::draft(&parent, &req)
                        .map_err(|m| ApiError::new("bad_request", m))?,
                )
            }
            "variant.finish" => {
                let s: Scale = arg(&args, "scale")?;
                let rows: Vec<prismsurvey_core::variant::RowDecision> = arg(&args, "rows")?;
                to_value(
                    prismsurvey_core::variant::finish(s, &rows)
                        .map_err(|m| ApiError::new("bad_request", m))?,
                )
            }
            "variant.parentChanges" => {
                // The parent's changes since the variant was made, offered one by one; nothing is applied here.
                let s: Scale = arg(&args, "scale")?;
                let folder: String =
                    opt_arg(&args, "folder")?.unwrap_or_else(|| self.settings().library_folder);
                let Some(vo) = &s.variant_of else {
                    return to_value(json!({ "found": false, "changes": [] }));
                };
                match find_parent(&folder, &vo.parent_id) {
                    Some((p, parent)) => to_value(json!({
                        "found": true,
                        "parentPath": p.display().to_string(),
                        "parentTitle": parent.title,
                        "parentVersion": parent.history.last().map(|h| h.version).unwrap_or(0),
                        "changes": prismsurvey_core::variant::parent_changes(&s, &parent),
                    })),
                    None => to_value(json!({ "found": false, "changes": [] })),
                }
            }
            "variant.carry" => {
                let mut s: Scale = arg(&args, "scale")?;
                let parent_path: String = arg(&args, "parentPath")?;
                let change: prismsurvey_core::variant::ParentChange = arg(&args, "change")?;
                let parent = pscale::load(Path::new(&parent_path))
                    .map_err(|e| ApiError::new("open_failed", e.to_string()))?;
                prismsurvey_core::variant::carry(&mut s, &parent, &change)
                    .map_err(|m| ApiError::new("cannot_carry", m))?;
                to_value(s)
            }
            "variant.decline" => {
                let mut s: Scale = arg(&args, "scale")?;
                let key: String = arg(&args, "key")?;
                prismsurvey_core::variant::decline(&mut s, &key);
                to_value(s)
            }
            "scale.applyFix" => {
                let mut s: Scale = arg(&args, "scale")?;
                let fix: selfcheck::Fix = arg(&args, "fix")?;
                selfcheck::apply_fix(&mut s, &fix);
                to_value(s)
            }
            "scale.export" => {
                let s: Scale = arg(&args, "scale")?;
                let path: String = arg(&args, "path")?;
                let copy: Copy = opt_arg(&args, "copy")?.unwrap_or_default();
                let done = export_scale(
                    &s,
                    &ExportOptions {
                        copy,
                        ..Default::default()
                    },
                );
                std::fs::write(&path, &done.bytes)
                    .map_err(|e| ApiError::new("export_failed", format!("{path}: {e}")))?;
                Ok(json!({ "notes": done.notes }))
            }
            "library.tags" => {
                // Every tag in the library with how many scales carry it, most used first.
                let folder: String =
                    opt_arg(&args, "folder")?.unwrap_or_else(|| self.settings().library_folder);
                let mut counts: Vec<(String, usize)> = Vec::new();
                for p in library_files(&folder) {
                    if let Ok(s) = pscale::load(&p) {
                        for t in tags::normalise(&s.tags) {
                            match counts
                                .iter_mut()
                                .find(|(x, _)| tags::key(x) == tags::key(&t))
                            {
                                Some(c) => c.1 += 1,
                                None => counts.push((t, 1)),
                            }
                        }
                    }
                }
                counts.sort_by(|a, b| {
                    b.1.cmp(&a.1)
                        .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
                });
                Ok(Value::Array(
                    counts
                        .into_iter()
                        .map(|(tag, count)| json!({ "tag": tag, "count": count }))
                        .collect(),
                ))
            }
            "library.renameTag" => {
                // Renames (or merges) a tag on every scale in the library that carries it.
                let folder: String =
                    opt_arg(&args, "folder")?.unwrap_or_else(|| self.settings().library_folder);
                let from: String = arg(&args, "from")?;
                let to: String = arg(&args, "to")?;
                let mut changed = Vec::new();
                for p in library_files(&folder) {
                    if let Ok(mut s) = pscale::load(&p) {
                        if tags::rename(&mut s, &from, &to) {
                            pscale::save(
                                &mut s,
                                &p,
                                &format!("Tag \"{from}\" renamed to \"{}\"", tags::tidy(&to)),
                            )
                            .map_err(|e| ApiError::new("save_failed", e.to_string()))?;
                            changed.push(p.display().to_string());
                        }
                    }
                }
                Ok(json!({ "changed": changed }))
            }
            "library.list" => {
                let folder: String =
                    opt_arg(&args, "folder")?.unwrap_or_else(|| self.settings().library_folder);
                let mut out = Vec::new();
                let mut links: Vec<(String, String, Option<String>)> = Vec::new();
                let mut parents: Vec<Option<(String, String)>> = Vec::new();
                {
                    for p in library_files(&folder) {
                        match pscale::load(&p) {
                            Ok(s) => {
                                let parent = s
                                    .variant_of
                                    .as_ref()
                                    .map(|v| (v.parent_id.clone(), v.parent_title.clone()));
                                links.push((
                                    s.id.clone(),
                                    s.title.clone(),
                                    parent.as_ref().map(|p| p.0.clone()),
                                ));
                                parents.push(parent);
                                out.push(LibraryEntry {
                                    path: p.display().to_string(),
                                    title: s.title.clone(),
                                    format: s.format,
                                    items: s.items.len(),
                                    tags: s.tags.clone(),
                                    updated: s.history.last().map(|h| h.saved_at.clone()),
                                    error: None,
                                    variant_chain: Vec::new(),
                                })
                            }
                            Err(e) => {
                                out.push(LibraryEntry {
                                    path: p.display().to_string(),
                                    title: p
                                        .file_stem()
                                        .map(|s| s.to_string_lossy().into_owned())
                                        .unwrap_or_default(),
                                    format: Format::LikertBlankLine,
                                    items: 0,
                                    tags: Vec::new(),
                                    updated: None,
                                    error: Some(e.to_string()),
                                    variant_chain: Vec::new(),
                                });
                                parents.push(None);
                            }
                        }
                    }
                }
                for (entry, parent) in out.iter_mut().zip(&parents) {
                    if let Some((id, title)) = parent {
                        let mut chain = prismsurvey_core::variant::chain(Some(id), &links);
                        if chain.is_empty() {
                            chain.push(title.clone());
                        }
                        entry.variant_chain = chain;
                    }
                }
                to_value(out)
            }
            "article.detect" => {
                // Reads an article (text layer, then OCR where needed) and offers every candidate measure, best first.
                let path: String = arg(&args, "path")?;
                let prepared = prismsurvey_detect::measure::prepare(Path::new(&path))
                    .map_err(|e| ApiError::new("detect_failed", e))?;
                let d = prepared.detection();
                let pages: Vec<Value> = prepared
                    .article
                    .pages
                    .iter()
                    .map(|p| json!({ "number": p.number, "width": p.width, "height": p.height, "source": p.source }))
                    .collect();
                self.article = Some((path, prepared));
                Ok(json!({ "detection": to_value(d)?, "pages": pages }))
            }
            "article.page" => {
                // A picture of one page of the open article, for the confirm screen.
                let path: String = arg(&args, "path")?;
                let page: u32 = arg(&args, "page")?;
                let width: u32 = opt_arg(&args, "width")?.unwrap_or(900);
                let bytes = std::fs::read(&path)
                    .map_err(|e| ApiError::new("page_failed", e.to_string()))?;
                let lower = path.to_lowercase();
                let png = if lower.ends_with(".pdf") {
                    prismsurvey_detect::render::render_page_png(&bytes, page, width)
                        .map_err(|e| ApiError::new("page_failed", e.to_string()))?
                } else {
                    bytes
                };
                let mime = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                    "image/jpeg"
                } else {
                    "image/png"
                };
                Ok(json!({ "dataUrl": format!("data:{mime};base64,{}", base64(&png)) }))
            }
            "article.parseRegion" => {
                let path: String = arg(&args, "path")?;
                let page: u32 = arg(&args, "page")?;
                let region: (f64, f64, f64, f64) = arg(&args, "region")?;
                let Some((open, prepared)) = &self.article else {
                    return Err(ApiError::new("no_article", "open the article first"));
                };
                if *open != path {
                    return Err(ApiError::new("no_article", "open the article first"));
                }
                match prepared.parse_region(page, region) {
                    Some(s) => to_value(s),
                    None => Err(ApiError::new(
                        "nothing_there",
                        "no text was found inside the outline",
                    )),
                }
            }
            "article.confirm" => {
                // Keeps the correction as a labelled example, and in benchmark mode saves the answer key.
                let path: String = arg(&args, "path")?;
                let first_guess: Option<Scale> = opt_arg(&args, "firstGuess")?;
                let confirmed: Scale = arg(&args, "confirmed")?;
                let benchmark: bool = opt_arg(&args, "benchmark")?.unwrap_or(false);
                let now = pscale::now_rfc3339();
                let config = self
                    .settings_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default();
                let folder = config.join("Corrections");
                std::fs::create_dir_all(&folder)
                    .map_err(|e| ApiError::new("confirm_failed", e.to_string()))?;
                let file = Path::new(&path)
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let example = json!({ "file": file, "savedAt": now, "firstGuess": first_guess, "confirmed": confirmed });
                let name = safe_file_name(&format!("{file} {now}.json"));
                std::fs::write(
                    folder.join(name),
                    serde_json::to_string_pretty(&example).unwrap_or_default(),
                )
                .map_err(|e| ApiError::new("confirm_failed", e.to_string()))?;
                let mut key = None;
                if benchmark {
                    let p = prismsurvey_detect::benchmark::save_key(
                        &prismsurvey_detect::benchmark::default_key_folder(),
                        Path::new(&path),
                        first_guess,
                        confirmed,
                        &now,
                    )
                    .map_err(|e| ApiError::new("confirm_failed", e.to_string()))?;
                    key = Some(p.display().to_string());
                }
                Ok(json!({ "answerKey": key }))
            }
            "study.new" => {
                let title: String = opt_arg(&args, "title")?.unwrap_or_else(|| "New study".into());
                to_value(Study::new(&title))
            }
            "study.open" => {
                let path: String = arg(&args, "path")?;
                to_value(
                    load_study(Path::new(&path))
                        .map_err(|e| ApiError::new("open_failed", e.to_string()))?,
                )
            }
            "study.save" => {
                let path: String = arg(&args, "path")?;
                let mut st: Study = arg(&args, "study")?;
                let summary: String = opt_arg(&args, "summary")?.unwrap_or_else(|| "Saved".into());
                save_study(&mut st, Path::new(&path), &summary)
                    .map_err(|e| ApiError::new("save_failed", e.to_string()))?;
                self.record_study(&st);
                to_value(st)
            }
            "study.addScale" => {
                // Adds a copy of a library scale at the end of the study.
                let mut st: Study = arg(&args, "study")?;
                let path: String = arg(&args, "path")?;
                let scale = pscale::load(Path::new(&path))
                    .map_err(|e| ApiError::new("open_failed", e.to_string()))?;
                st.parts.push(Part::Scale {
                    scale,
                    library_path: Some(path),
                });
                to_value(st)
            }
            "study.refresh" => {
                let mut st: Study = arg(&args, "study")?;
                let changed = refresh_from_library(&mut st);
                Ok(json!({ "study": to_value(st)?, "changed": changed }))
            }
            "study.recommend" => {
                // Measures that would sit well beside the study's scales, each with its reasons
                // (crates/prismsurvey-core/src/recommend.rs). Library scales and Comm Directory entries are candidates.
                let st: Study = arg(&args, "study")?;
                let settings: prismsurvey_core::recommend::Settings =
                    opt_arg(&args, "settings")?.unwrap_or_default();
                let study: Vec<Measure> = st
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        Part::Scale {
                            scale,
                            library_path,
                        } => Some(scale_measure(scale, library_path.as_deref())),
                        _ => None,
                    })
                    .collect();
                let folder = self.settings().library_folder;
                let mut candidates: Vec<Measure> = library_files(&folder)
                    .iter()
                    .filter_map(|p| {
                        pscale::load(p)
                            .ok()
                            .map(|s| scale_measure(&s, Some(&p.display().to_string())))
                    })
                    .collect();
                let linked: std::collections::HashSet<String> = candidates
                    .iter()
                    .chain(study.iter())
                    .flat_map(|m| m.aliases.iter().cloned())
                    .collect();
                if let Ok(d) = self.directory() {
                    for e in &d.entries {
                        let key = format!("dir:{}", e.id);
                        if linked.contains(&key) {
                            continue;
                        }
                        let mut tags = e.categories.clone();
                        if !tags.contains(&e.chapter) {
                            tags.push(e.chapter.clone());
                        }
                        candidates.push(Measure {
                            key,
                            title: e.name.clone(),
                            collection: "Comm Directory".into(),
                            tags,
                            items: prismsurvey_core::directory::leading_number(&e.number_of_items)
                                .unwrap_or(0),
                            format: None,
                            points: None,
                            item_text: vec![],
                            aliases: vec![],
                        });
                    }
                }
                let history: Vec<Vec<String>> = self
                    .study_history()
                    .into_iter()
                    .filter(|(id, _)| *id != st.id)
                    .map(|(_, keys)| keys)
                    .collect();
                let recs = prismsurvey_core::recommend::recommend(
                    &study,
                    &candidates,
                    &history,
                    &settings,
                );
                Ok(json!({ "recommendations": to_value(recs)? }))
            }
            "study.length" => {
                let st: Study = arg(&args, "study")?;
                let (items, minutes) = st.length();
                Ok(json!({ "items": items, "minutes": minutes }))
            }
            "study.export" => {
                let st: Study = arg(&args, "study")?;
                let path: String = arg(&args, "path")?;
                let copy: Copy = opt_arg(&args, "copy")?.unwrap_or_default();
                let done = prismsurvey_docx::study::export_study(&st, copy);
                std::fs::write(&path, &done.bytes)
                    .map_err(|e| ApiError::new("export_failed", format!("{path}: {e}")))?;
                Ok(json!({ "notes": done.notes }))
            }
            "scale.clipboard" => {
                // One scale as HTML and plain text, for pasting into an open Word document.
                let s: Scale = arg(&args, "scale")?;
                let copy: Copy = opt_arg(&args, "copy")?.unwrap_or_default();
                Ok(
                    json!({ "html": prismsurvey_docx::html::scale_html(&s, copy), "text": prismsurvey_docx::html::scale_text(&s) }),
                )
            }
            "refman.status" => Ok(self.refman().status()),
            "refman.pair" => {
                let id = self
                    .refman()
                    .pair()
                    .map_err(|e| ApiError::new("refman", e))?;
                Ok(json!({ "requestId": id }))
            }
            "refman.pairStatus" => {
                let id: String = arg(&args, "id")?;
                let state = self
                    .refman()
                    .pair_status(&id)
                    .map_err(|e| ApiError::new("refman", e))?;
                Ok(json!({ "state": state }))
            }
            "refman.forget" => {
                self.refman().forget();
                Ok(Value::Null)
            }
            "refman.cite" => {
                // Links the scale's source and adapted-from work to Prism Reference Manager and formats them in APA 7.
                let mut s: Scale = arg(&args, "scale")?;
                let rm = self.refman();
                let mut notes = Vec::new();
                if let Some(c) = s.source.as_mut() {
                    notes.push(rm.cite(c, "source"));
                }
                if let Some(c) = s.adapted_from.as_mut() {
                    notes.push(rm.cite(c, "adapted-from work"));
                }
                if notes.is_empty() {
                    notes.push("The scale has no source to link.".into());
                }
                Ok(json!({ "scale": to_value(s)?, "notes": notes }))
            }
            "library.search" => {
                // Ranked search over the library, with filters (see crates/prismsurvey-core/src/search.rs).
                let folder: String =
                    opt_arg(&args, "folder")?.unwrap_or_else(|| self.settings().library_folder);
                let query: String = opt_arg(&args, "query")?.unwrap_or_default();
                let filters: prismsurvey_core::search::Filters =
                    opt_arg(&args, "filters")?.unwrap_or_default();
                let files = library_files(&folder);
                let keep: std::collections::HashSet<&PathBuf> = files.iter().collect();
                self.search_cache.retain(|p, _| keep.contains(p));
                for p in &files {
                    let modified = std::fs::metadata(p).and_then(|m| m.modified()).ok();
                    let fresh = matches!((self.search_cache.get(p), modified), (Some((t, _)), Some(m)) if *t == m);
                    if !fresh {
                        if let (Ok(scale), Some(m)) = (pscale::load(p), modified) {
                            let doc = prismsurvey_core::search::Doc::from_scale(
                                &p.display().to_string(),
                                &scale,
                            );
                            self.search_cache.insert(p.clone(), (m, doc));
                        }
                    }
                }
                let docs: Vec<prismsurvey_core::search::Doc> = files
                    .iter()
                    .filter_map(|p| self.search_cache.get(p).map(|(_, d)| d.clone()))
                    .collect();
                let hits = prismsurvey_core::search::search(&docs, &query, &filters);
                Ok(json!({ "hits": to_value(hits)?, "total": docs.len() }))
            }
            "directory.info" => {
                // Whether the Comm Add On is installed, and its chapters and categories for the filters.
                match self.directory() {
                    Ok(d) => {
                        let mut chapters: Vec<String> = Vec::new();
                        let mut categories: std::collections::BTreeMap<String, usize> =
                            std::collections::BTreeMap::new();
                        for e in &d.entries {
                            if !chapters.contains(&e.chapter) {
                                chapters.push(e.chapter.clone());
                            }
                            for c in &e.categories {
                                *categories.entry(c.clone()).or_default() += 1;
                            }
                        }
                        Ok(
                            json!({ "installed": true, "title": d.title, "authors": d.authors, "rights": d.rights, "entries": d.entries.len(), "chapters": chapters, "categories": categories, "hasIntroduction": !d.introduction.is_empty() }),
                        )
                    }
                    Err(_) => Ok(json!({ "installed": false })),
                }
            }
            "directory.introduction" => {
                // The book's preface, shown as the add-on's introduction ("About this directory").
                let d = self
                    .directory()
                    .map_err(|e| ApiError::new("not_installed", e.to_string()))?;
                to_value(json!({ "title": d.title, "blocks": d.introduction }))
            }
            "directory.search" => {
                let query: String = opt_arg(&args, "query")?.unwrap_or_default();
                let mut filters: prismsurvey_core::search::Filters =
                    opt_arg(&args, "filters")?.unwrap_or_default();
                let chapter: Option<String> = opt_arg(&args, "chapter")?;
                filters.collection = None;
                let d = self.directory()?;
                let docs: Vec<prismsurvey_core::search::Doc> = d
                    .entries
                    .iter()
                    .filter(|e| chapter.as_ref().is_none_or(|c| &e.chapter == c))
                    .map(|e| e.to_doc())
                    .collect();
                let hits = prismsurvey_core::search::search(&docs, &query, &filters);
                Ok(json!({ "hits": to_value(hits)?, "total": docs.len() }))
            }
            "directory.entry" => {
                let id: String = arg(&args, "id")?;
                let d = self.directory()?;
                let e = d
                    .entries
                    .iter()
                    .find(|e| e.id == id)
                    .ok_or_else(|| ApiError::new("not_found", "there is no such entry"))?;
                to_value(e)
            }
            "directory.link" => {
                // Links a scale to the entry it was built from: the entry's reference becomes the source when the scale
                // has none, and the entry's categories become tags.
                let mut s: Scale = arg(&args, "scale")?;
                let id: String = arg(&args, "id")?;
                let d = self.directory()?;
                let e = d
                    .entries
                    .iter()
                    .find(|e| e.id == id)
                    .ok_or_else(|| ApiError::new("not_found", "there is no such entry"))?
                    .clone();
                s.directory_entry = Some(e.id.clone());
                if s.source.is_none() || s.source.as_ref().is_some_and(|c| c.apa.trim().is_empty())
                {
                    s.source = Some(prismsurvey_core::Citation {
                        apa: e.source_apa7.clone(),
                        doi: e.doi.clone(),
                        reference_manager_id: None,
                    });
                }
                for c in &e.categories {
                    tags::add(&mut s, c);
                }
                if s.title.trim().is_empty() || s.title == "Untitled measure" {
                    s.title = e.name.clone();
                }
                to_value(s)
            }
            "settings.get" => to_value(self.settings()),
            "settings.set" => {
                let s: Settings = arg(&args, "settings")?;
                if let Some(dir) = self.settings_path.parent() {
                    std::fs::create_dir_all(dir)
                        .map_err(|e| ApiError::new("settings_failed", e.to_string()))?;
                }
                std::fs::write(
                    &self.settings_path,
                    serde_json::to_string_pretty(&s).unwrap_or_default(),
                )
                .map_err(|e| ApiError::new("settings_failed", e.to_string()))?;
                to_value(s)
            }
            other => Err(ApiError::new(
                "unknown_command",
                format!("there is no command {other}"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api() -> (Api, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (Api::new(&dir.path().join("config"), dir.path()), dir)
    }

    #[test]
    fn a_variant_is_drafted_finished_saved_and_follows_its_parent() {
        let (mut api, dir) = api();
        let lib = dir.path().join("lib");
        let mut s = api
            .call("scale.new", json!({ "format": "likertBlankLine" }))
            .unwrap();
        s["title"] = json!("Apprehension with Physicians Scale");
        s["items"] = json!([
            { "number": 1, "text": "I am calm with my physician.", "reverse": { "state": "notReversed" } },
            { "number": 2, "text": "A physician makes me tense.", "reverse": { "state": "confirmed" } }
        ]);
        let pp = lib.join("Parent.pscale");
        let parent = api
            .call("scale.save", json!({ "path": pp, "scale": s }))
            .unwrap();
        let d = api
            .call(
                "variant.draft",
                json!({ "path": pp, "request": { "oldTarget": "physician", "newTarget": "AI" } }),
            )
            .unwrap();
        assert_eq!(d["rows"][1]["proposed"], "An AI makes me tense.");
        let rows: Vec<Value> = d["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| json!({ "number": r["number"], "text": r["proposed"] }))
            .collect();
        let v = api
            .call(
                "variant.finish",
                json!({ "scale": d["scale"], "rows": rows }),
            )
            .unwrap();
        let c = api.call("scale.check", json!({ "scale": v })).unwrap();
        assert!(
            c["findings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["check"] != "variantStructure"),
            "{c}"
        );
        let vp = lib.join("Variant.pscale");
        let v = api
            .call("scale.save", json!({ "path": vp, "scale": v }))
            .unwrap();
        let list = api.call("library.list", json!({ "folder": lib })).unwrap();
        let entry = list
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["title"] == v["title"])
            .unwrap()
            .clone();
        assert_eq!(
            entry["variantChain"],
            json!(["Apprehension with Physicians Scale"])
        );
        // The parent changes; the variant is offered the change and carries it only when asked.
        let mut p2 = parent.clone();
        p2["items"][0]["text"] = json!("I am relaxed with my physician.");
        api.call("scale.save", json!({ "path": pp, "scale": p2 }))
            .unwrap();
        let ch = api
            .call(
                "variant.parentChanges",
                json!({ "scale": v, "folder": lib }),
            )
            .unwrap();
        assert_eq!(ch["found"], true);
        assert_eq!(ch["changes"].as_array().unwrap().len(), 1, "{ch}");
        let v2 = api
            .call(
                "variant.carry",
                json!({ "scale": v, "parentPath": ch["parentPath"], "change": ch["changes"][0] }),
            )
            .unwrap();
        assert_eq!(v2["items"][0]["text"], "I am relaxed with my AI.");
        let none = api
            .call(
                "variant.parentChanges",
                json!({ "scale": v2, "folder": lib }),
            )
            .unwrap();
        assert!(none["changes"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_finding_can_be_kept_as_published_with_a_note() {
        let (mut api, dir) = api();
        let mut s = api
            .call("scale.new", json!({ "format": "likertBlankLine" }))
            .unwrap();
        s["items"] = json!([
            { "number": 1, "text": "I like maps.", "reverse": { "state": "notReversed" } },
            { "number": 2, "text": "I like maps.", "reverse": { "state": "notReversed" } }
        ]);
        let c = api.call("scale.check", json!({ "scale": s })).unwrap();
        let key = c["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["check"] == "duplicateItem")
            .expect("a duplicate is reported")["key"]
            .clone();
        let e = api
            .call(
                "scale.keepAsPublished",
                json!({ "scale": s, "key": key, "original": "" }),
            )
            .unwrap_err();
        assert_eq!(e.code, "note_needed");
        let s = api
            .call(
                "scale.keepAsPublished",
                json!({ "scale": s, "key": key, "original": "the original prints item 1 twice", "page": "3" }),
            )
            .unwrap();
        let c = api.call("scale.check", json!({ "scale": s })).unwrap();
        let f = c["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["key"] == key)
            .unwrap()
            .clone();
        assert_eq!(f["status"], "published");
        assert_eq!(
            c["sourceNotes"][0]["text"],
            "As published (page 3): the original prints item 1 twice. Kept as printed."
        );
        assert_eq!(c["sourceNotes"][0]["items"], json!([1, 2]));
        // The note is saved in the scale file and read back.
        let p = dir.path().join("Maps.pscale");
        api.call("scale.save", json!({ "path": p, "scale": s }))
            .unwrap();
        let back = api.call("scale.open", json!({ "path": p })).unwrap();
        assert_eq!(back["sourceNotes"][0]["handling"], "kept");
        let s = api
            .call(
                "scale.removeSourceNote",
                json!({ "scale": back, "key": key }),
            )
            .unwrap();
        assert!(s.get("sourceNotes").is_none());
    }

    #[test]
    fn new_switch_save_list_export() {
        let (mut api, dir) = api();
        let s = api
            .call("scale.new", json!({ "format": "likertBlankLine" }))
            .unwrap();
        let r = api
            .call(
                "scale.setFormat",
                json!({ "scale": s, "format": "semanticDifferential" }),
            )
            .unwrap();
        assert_eq!(r["outcome"]["instructionsRewritten"], true);
        let s = r["scale"].clone();
        let lib = dir.path().join("lib");
        let path = lib.join("a.pscale");
        let saved = api
            .call(
                "scale.save",
                json!({ "path": path, "scale": s, "summary": "First" }),
            )
            .unwrap();
        assert_eq!(saved["history"][0]["summary"], "First");
        let list = api.call("library.list", json!({ "folder": lib })).unwrap();
        assert_eq!(list.as_array().unwrap().len(), 1);
        let docx = dir.path().join("a.docx");
        api.call(
            "scale.export",
            json!({ "scale": saved, "path": docx, "copy": "researcher" }),
        )
        .unwrap();
        assert!(std::fs::metadata(&docx).unwrap().len() > 1000);
    }

    #[test]
    fn check_reports_the_title_count_and_range() {
        let (mut api, _dir) = api();
        let mut s: Scale = serde_json::from_value(
            api.call("scale.new", json!({ "format": "likertBlankLine" }))
                .unwrap(),
        )
        .unwrap();
        s.title = "Test-15 Scale".into();
        s.items.truncate(16);
        let c = api.call("scale.check", json!({ "scale": s })).unwrap();
        assert_eq!(c["titleCountMismatch"], json!([15, 16]));
        assert_eq!(c["computedRange"], json!([16.0, 80.0]));
    }

    #[test]
    fn selectors() {
        assert_eq!(parse_selector("odd").unwrap(), ItemSelector::Odd);
        assert_eq!(
            parse_selector("1-5").unwrap(),
            ItemSelector::Block { first: 1, last: 5 }
        );
        assert_eq!(
            parse_selector("1, 3, & 5").unwrap(),
            ItemSelector::List {
                numbers: vec![1, 3, 5]
            }
        );
        assert!(parse_selector("one").is_err());
    }

    #[test]
    fn settings_round_trip() {
        let (mut api, dir) = api();
        let s = api.call("settings.get", Value::Null).unwrap();
        assert!(s["libraryFolder"]
            .as_str()
            .unwrap()
            .ends_with("Prism Survey Library"));
        let new = json!({ "libraryFolder": dir.path().join("x") });
        api.call("settings.set", json!({ "settings": new }))
            .unwrap();
        assert_eq!(api.call("settings.get", Value::Null).unwrap(), new);
    }

    #[test]
    fn tags_are_counted_and_renamed_across_the_library() {
        let (mut api, dir) = api();
        let lib = dir.path().join("lib");
        for (name, t) in [
            ("a", vec!["Health", "Dissertation"]),
            ("b", vec!["health "]),
            ("c", vec![]),
        ] {
            let mut s: Scale = serde_json::from_value(
                api.call("scale.new", json!({ "format": "likertBlankLine" }))
                    .unwrap(),
            )
            .unwrap();
            s.tags = t.into_iter().map(String::from).collect();
            api.call(
                "scale.save",
                json!({ "path": lib.join(format!("{name}.pscale")), "scale": s }),
            )
            .unwrap();
        }
        let counts = api.call("library.tags", json!({ "folder": lib })).unwrap();
        assert_eq!(
            counts,
            json!([{ "tag": "Health", "count": 2 }, { "tag": "Dissertation", "count": 1 }])
        );
        let r = api
            .call(
                "library.renameTag",
                json!({ "folder": lib, "from": "HEALTH", "to": "Health Communication" }),
            )
            .unwrap();
        assert_eq!(r["changed"].as_array().unwrap().len(), 2);
        let list = api.call("library.list", json!({ "folder": lib })).unwrap();
        assert_eq!(list[1]["tags"], json!(["Health Communication"]));
    }

    #[test]
    fn a_study_is_built_saved_and_exported() {
        let (mut api, dir) = api();
        let lib = dir.path().join("lib");
        let mut paths = Vec::new();
        for name in ["a", "b"] {
            let mut s: Scale = serde_json::from_value(
                api.call("scale.new", json!({ "format": "likertBoxGrid" }))
                    .unwrap(),
            )
            .unwrap();
            s.title = format!("Scale {name}");
            s.items.truncate(4);
            for (k, it) in s.items.iter_mut().enumerate() {
                it.text = format!("Statement {k}");
            }
            let p = lib.join(format!("{name}.pscale"));
            api.call("scale.save", json!({ "path": p, "scale": s }))
                .unwrap();
            paths.push(p);
        }
        let mut st = api.call("study.new", json!({ "title": "Spring" })).unwrap();
        for p in &paths {
            st = api
                .call("study.addScale", json!({ "study": st, "path": p }))
                .unwrap();
        }
        assert_eq!(
            api.call("study.length", json!({ "study": st })).unwrap(),
            json!({ "items": 8, "minutes": 1 })
        );
        let sp = dir.path().join("spring.pstudy");
        let saved = api
            .call("study.save", json!({ "path": sp, "study": st }))
            .unwrap();
        assert_eq!(
            api.call("study.open", json!({ "path": sp })).unwrap(),
            saved
        );
        let out = dir.path().join("spring.docx");
        api.call(
            "study.export",
            json!({ "study": saved, "path": out, "copy": "respondent" }),
        )
        .unwrap();
        assert!(std::fs::metadata(&out).unwrap().len() > 1000);
        let clip = api
            .call(
                "scale.clipboard",
                json!({ "scale": saved["parts"][0]["scale"] }),
            )
            .unwrap();
        assert!(clip["html"].as_str().unwrap().contains("Statement 1"));
    }

    #[test]
    fn a_study_gets_recommendations_with_reasons() {
        let (mut api, dir) = api();
        let lib = dir.path().join("lib");
        api.call(
            "settings.set",
            json!({ "settings": { "libraryFolder": lib } }),
        )
        .unwrap();
        let mut paths = Vec::new();
        for (name, title, tag) in [
            ("a", "Classroom Apprehension", "Apprehension"),
            ("b", "Willingness to Speak", "Apprehension"),
            ("c", "Garden Joy", "Leisure"),
            ("d", "Office Mood", "Work"),
        ] {
            let mut s: Scale = serde_json::from_value(
                api.call("scale.new", json!({ "format": "likertBlankLine" }))
                    .unwrap(),
            )
            .unwrap();
            s.title = title.into();
            s.tags = vec![tag.into()];
            for (k, it) in s.items.iter_mut().enumerate() {
                it.text = format!("{name} statement {k}");
            }
            let p = lib.join(format!("{name}.pscale"));
            api.call("scale.save", json!({ "path": p, "scale": s }))
                .unwrap();
            paths.push(p);
        }
        // An earlier study held a and d together.
        let mut old = api.call("study.new", json!({ "title": "Old" })).unwrap();
        for p in [&paths[0], &paths[3]] {
            old = api
                .call("study.addScale", json!({ "study": old, "path": p }))
                .unwrap();
        }
        api.call(
            "study.save",
            json!({ "path": dir.path().join("old.pstudy"), "study": old }),
        )
        .unwrap();
        let mut st = api.call("study.new", json!({ "title": "New" })).unwrap();
        st = api
            .call("study.addScale", json!({ "study": st, "path": paths[0] }))
            .unwrap();
        let r = api.call("study.recommend", json!({ "study": st })).unwrap();
        let recs = r["recommendations"].as_array().unwrap();
        let titles: Vec<&str> = recs.iter().map(|x| x["title"].as_str().unwrap()).collect();
        assert!(titles.contains(&"Willingness to Speak"), "{titles:?}");
        assert!(titles.contains(&"Office Mood"), "{titles:?}");
        assert!(!titles.contains(&"Garden Joy"));
        assert!(!titles.contains(&"Classroom Apprehension"));
        let mood = recs.iter().find(|x| x["title"] == "Office Mood").unwrap();
        assert!(mood["reasons"][0]
            .as_str()
            .unwrap()
            .contains("earlier study"));
    }

    #[test]
    fn the_library_can_be_searched() {
        let (mut api, dir) = api();
        let lib = dir.path().join("lib");
        for (name, title, tag) in [
            ("a", "Communication Anxiety Scale", "Instructional"),
            ("b", "Library Comfort Scale", "Leisure"),
        ] {
            let mut s: Scale = serde_json::from_value(
                api.call("scale.new", json!({ "format": "likertBlankLine" }))
                    .unwrap(),
            )
            .unwrap();
            s.title = title.into();
            s.tags = vec![tag.into()];
            api.call(
                "scale.save",
                json!({ "path": lib.join(format!("{name}.pscale")), "scale": s }),
            )
            .unwrap();
        }
        let r = api
            .call(
                "library.search",
                json!({ "folder": lib, "query": "anxiety" }),
            )
            .unwrap();
        assert_eq!(r["hits"].as_array().unwrap().len(), 1);
        assert_eq!(r["total"], 2);
        let r = api
            .call(
                "library.search",
                json!({ "folder": lib, "query": "", "filters": { "tag": "leisure" } }),
            )
            .unwrap();
        assert_eq!(r["hits"][0]["title"], "Library Comfort Scale");
    }

    #[test]
    fn the_comm_directory_is_searched_and_linked() {
        let (mut api, _dir) = api();
        let info = api.call("directory.info", Value::Null).unwrap();
        assert_eq!(
            info["installed"], true,
            "the repository's add-on folder is found when running from source"
        );
        assert_eq!(info["entries"], 509);
        assert_eq!(info["hasIntroduction"], true);
        let intro = api.call("directory.introduction", Value::Null).unwrap();
        assert!(intro["blocks"].as_array().unwrap().len() > 50);
        let hits = api
            .call("directory.search", json!({ "query": "listening barriers" }))
            .unwrap();
        let first = hits["hits"][0]["key"].as_str().unwrap().to_string();
        let e = api.call("directory.entry", json!({ "id": first })).unwrap();
        assert!(e["name"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("listening"));
        let s = api
            .call("scale.new", json!({ "format": "likertBlankLine" }))
            .unwrap();
        let linked = api
            .call("directory.link", json!({ "scale": s, "id": e["id"] }))
            .unwrap();
        assert_eq!(linked["directoryEntry"], e["id"]);
        assert!(linked["source"]["apa"].as_str().unwrap().len() > 10);
        assert!(linked["tags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t == "Listening"));
    }

    #[test]
    fn unknown_commands_say_so() {
        let (mut api, _dir) = api();
        assert_eq!(
            api.call("nope", Value::Null).unwrap_err().code,
            "unknown_command"
        );
    }
}
