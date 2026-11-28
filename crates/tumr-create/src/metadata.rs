//! `.tumr/project.json`: how TUMR recognises a project it created (CRT-A). Plain facts only,
//! never secrets, tokens or personal data; safe to commit with the project.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::request::Validated;

/// Folder and file inside the project.
pub const DIR: &str = ".tumr";
pub const FILE: &str = "project.json";
/// Version of the format below.
pub const SCHEMA: u32 = 1;

/// The metadata file's content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectMetadata {
    /// Always `"TUMR"`.
    pub managed_by: String,
    pub schema_version: u32,
    pub project_name: String,
    /// Framework id (`kotlin`, `react-native`, …).
    pub framework: String,
    pub language: String,
    pub package_id: String,
    /// The agent client id chosen at creation, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// UTC, RFC 3339 (`2026-11-28T09:30:00Z`).
    pub created_at: String,
    /// The TUMR version that created it.
    pub created_with: String,
}

impl ProjectMetadata {
    #[must_use]
    pub fn for_project(v: &Validated, tumr_version: &str) -> Self {
        Self {
            managed_by: "TUMR".into(),
            schema_version: SCHEMA,
            project_name: v.name.clone(),
            framework: v.framework.id().into(),
            language: v.framework.language().into(),
            package_id: v.package_id.clone(),
            agent: v.agent.clone(),
            created_at: rfc3339_utc(SystemTime::now()),
            created_with: tumr_version.into(),
        }
    }
}

#[must_use]
pub fn path(project: &Path) -> PathBuf {
    project.join(DIR).join(FILE)
}

/// Writes the file (creating `.tumr`).
pub fn write(project: &Path, m: &ProjectMetadata) -> Result<(), String> {
    let p = path(project);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
    std::fs::write(&p, text + "\n").map_err(|e| format!("cannot write {}: {e}", p.display()))
}

/// Reads the file; `Ok(None)` when the project has none (not created by TUMR).
pub fn read(project: &Path) -> Result<Option<ProjectMetadata>, String> {
    let p = path(project);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read {}: {e}", p.display())),
    };
    let m: ProjectMetadata =
        serde_json::from_str(&text).map_err(|e| format!("{} is not valid: {e}", p.display()))?;
    if m.managed_by != "TUMR" {
        return Err(format!("{} is not TUMR project metadata", p.display()));
    }
    if m.schema_version > SCHEMA {
        return Err(format!(
            "{} was written by a newer TUMR (schema {}); update TUMR",
            p.display(),
            m.schema_version
        ));
    }
    Ok(Some(m))
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a time (no time-zone database needed: always UTC).
#[must_use]
pub fn rfc3339_utc(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    // days since 1970-01-01 to a civil date (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn times_are_rfc3339_utc() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        // 2026-11-28 09:30:15 UTC
        let t = UNIX_EPOCH + Duration::from_secs(1_795_858_215);
        assert_eq!(rfc3339_utc(t), "2026-11-28T09:30:15Z");
        // a leap day
        let t = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(rfc3339_utc(t), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn write_and_read_back() {
        let d = std::env::temp_dir().join(format!("tumr-create-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(read(&d).unwrap(), None, "no metadata yet");
        let m = ProjectMetadata {
            managed_by: "TUMR".into(),
            schema_version: SCHEMA,
            project_name: "My App".into(),
            framework: "kotlin".into(),
            language: "kotlin".into(),
            package_id: "com.example.myapp".into(),
            agent: Some("claude-code".into()),
            created_at: "2026-11-28T09:30:15Z".into(),
            created_with: "0.1.35".into(),
        };
        write(&d, &m).unwrap();
        assert_eq!(read(&d).unwrap(), Some(m.clone()));
        let text = std::fs::read_to_string(path(&d)).unwrap();
        assert!(text.contains("\"managedBy\": \"TUMR\"") && text.contains("\"schemaVersion\": 1"));
        // a newer schema is refused, not misread
        let newer = text.replace("\"schemaVersion\": 1", "\"schemaVersion\": 9");
        std::fs::write(path(&d), newer).unwrap();
        assert!(read(&d).unwrap_err().contains("newer TUMR"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
