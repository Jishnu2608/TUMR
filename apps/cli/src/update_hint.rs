//! UPD-004: a one-line "update available" hint after `tumr` commands, so people who only use the
//! command line hear about new versions too.
//!
//! * At most one check a day: the time of the last check is kept in `<data>/state/update-hint.json`.
//! * Only for a person at a terminal: both stdout and stderr must be consoles, so scripts, agents
//!   (`tumr mcp`, `tumr rpc`, stop hooks) and `--json` output piped to a program never see it or
//!   wait for it. The hint goes to stderr.
//! * Never for `tumr update` itself, and never when update checks are off (`manifest_url = "off"`).
//! * The check gives up after a few seconds; a failed or slow check stays silent.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;
use tumr_installer::self_update;

use crate::core_process::CoreTarget;
use crate::update_cmd::VERSION;

/// How long between two checks.
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// How long a check may hold the prompt back.
const TIMEOUT: Duration = Duration::from_secs(3);

fn state_file(root: &Path) -> PathBuf {
    root.join("state").join("update-hint.json")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Is a check due, given the time of the last one (seconds since 1970)?
#[must_use]
pub fn due(last: Option<u64>, now: u64) -> bool {
    last.is_none_or(|l| now.saturating_sub(l) >= EVERY.as_secs())
}

fn last_check(file: &Path) -> Option<u64> {
    let text = std::fs::read_to_string(file).ok()?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .get("last_check")?
        .as_u64()
}

fn remember(file: &Path, at: u64) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, json!({ "last_check": at }).to_string());
}

/// The line to print for a newer release.
#[must_use]
pub fn line(current: &str, newer: &str) -> String {
    format!("Update available: {current} → {newer}. Run `tumr update` to install it.")
}

/// Whether this command may print the hint (decided before it runs; see the module notes).
#[must_use]
pub fn allowed(is_update: bool, is_machine: bool) -> bool {
    !is_update && !is_machine && std::io::stdout().is_terminal() && std::io::stderr().is_terminal()
}

/// Checks once a day and prints the hint when a newer version is published.
pub async fn maybe(target: &CoreTarget) {
    let root = target.paths.root().to_path_buf();
    let file = state_file(&root);
    let at = now();
    if !due(last_check(&file), at) {
        return;
    }
    let Ok(cfg) = tumr_storage::config::Config::load_in(&root) else {
        return;
    };
    let url = std::env::var("TUMR_UPDATE_URL")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| cfg.update.manifest());
    let Some(url) = url else {
        return; // update checks are off
    };
    // remembered before the check, so a slow network costs at most one wait a day
    remember(&file, at);
    let channel = cfg.update.channel.clone();
    let task = tokio::task::spawn_blocking(move || self_update::fetch_manifest(&url));
    let Ok(Ok(Ok(manifest))) = tokio::time::timeout(TIMEOUT, task).await else {
        return;
    };
    let build = tumr_platform::os::windows_version().map_or(0, |v| v.build);
    if let Some(rel) = self_update::pick(&manifest, VERSION, &channel, build) {
        eprintln!("\n{}", line(VERSION, &rel.version));
    }
}

#[cfg(test)]
mod tests {
    use super::{due, last_check, line, remember, state_file};

    #[test]
    fn checks_at_most_once_a_day() {
        assert!(due(None, 1_000));
        assert!(!due(Some(1_000), 1_000 + 60));
        assert!(!due(Some(1_000), 1_000 + 86_399));
        assert!(due(Some(1_000), 1_000 + 86_400));
        // a clock set back never blocks the next check for long
        assert!(!due(Some(5_000), 1_000));
    }

    #[test]
    fn remembers_the_last_check() {
        let dir = std::env::temp_dir().join(format!("tumr-hint-{}", std::process::id()));
        let file = state_file(&dir);
        assert_eq!(last_check(&file), None);
        remember(&file, 42);
        assert_eq!(last_check(&file), Some(42));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_hint_names_both_versions_and_the_command() {
        assert_eq!(
            line("0.1.36", "0.1.37"),
            "Update available: 0.1.36 → 0.1.37. Run `tumr update` to install it."
        );
    }
}
