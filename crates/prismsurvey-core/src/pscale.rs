//! The `.pscale` file: one plain JSON file per scale, with a schema version in every file, so a scale can be backed up,
//! copied or handed to a colleague as a single file. The format is described in `docs/PSCALE-FORMAT.md`.

use crate::model::{HistoryEntry, Scale, SCHEMA_VERSION};
use crate::scale::computed_range;
use std::path::Path;

pub const EXTENSION: &str = "pscale";

#[derive(Debug, thiserror::Error)]
pub enum PscaleError {
    #[error("could not read or write the scale file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the scale file is not valid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the scale file has no pscaleVersion, so it is not a Prism Survey scale")]
    NoVersion,
    #[error("the scale file was made by a newer Prism Survey (format version {0}); this version reads up to {SCHEMA_VERSION}")]
    TooNew(u32),
}

/// Reads a scale from JSON text, bringing an older file up to date.
pub fn from_json(text: &str) -> Result<Scale, PscaleError> {
    let mut value: serde_json::Value = serde_json::from_str(text)?;
    let version = value
        .get("pscaleVersion")
        .and_then(|v| v.as_u64())
        .ok_or(PscaleError::NoVersion)? as u32;
    if version > SCHEMA_VERSION {
        return Err(PscaleError::TooNew(version));
    }
    migrate(&mut value, version);
    Ok(serde_json::from_value(value)?)
}

/// Steps an older file up to the current schema. Version 1 is the first, so there is nothing to do yet.
fn migrate(value: &mut serde_json::Value, from: u32) {
    let _ = from;
    value["pscaleVersion"] = SCHEMA_VERSION.into();
}

pub fn to_json(scale: &Scale) -> String {
    serde_json::to_string_pretty(scale).expect("a scale always serialises")
}

pub fn load(path: &Path) -> Result<Scale, PscaleError> {
    let text = std::fs::read_to_string(path)?;
    from_json(text.trim_start_matches('\u{feff}'))
}

/// Saves a scale, adding a history entry and refreshing the computed range. The file is written beside its final name
/// and then moved into place, so a failed save never leaves half a file.
pub fn save(scale: &mut Scale, path: &Path, summary: &str) -> Result<(), PscaleError> {
    scale.pscale_version = SCHEMA_VERSION;
    scale.score_range.computed = computed_range(scale);
    let version = scale.history.last().map(|h| h.version + 1).unwrap_or(1);
    scale.history.push(HistoryEntry {
        version,
        saved_at: now_rfc3339(),
        summary: summary.to_string(),
    });
    write_atomic(path, to_json(scale).as_bytes())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PscaleError> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let tmp = path.with_extension("pscale.saving");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// The current UTC time as RFC 3339, without a time library.
pub fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    rfc3339(secs)
}

pub fn rfc3339(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Format;
    use crate::scale::new_blank;

    #[test]
    fn round_trips_through_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blank.pscale");
        let mut s = new_blank(Format::SemanticDifferential);
        s.title = "Test".into();
        save(&mut s, &path, "Created").unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.history.len(), 1);
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("\"pscaleVersion\": 1"));
    }

    #[test]
    fn a_newer_file_is_refused_with_a_reason() {
        let err = from_json(r#"{"pscaleVersion": 99}"#).unwrap_err();
        assert!(matches!(err, PscaleError::TooNew(99)));
    }

    #[test]
    fn dates_are_rfc3339() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_795_392_000), "2026-11-23T00:00:00Z");
    }
}
