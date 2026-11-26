//! UPD-001: in-app updates. The wizard asks at start whether a newer version is published on the
//! update channel (`[update] manifest_url`: empty = GitHub Releases, DEC-036; `off` = nothing is
//! contacted) and offers
//! **Update now**: `tumr update --yes --relaunch` downloads the installer, checks its size and
//! SHA-256 (and signature when required), closes this app, installs over the existing copy
//! (settings, toolchains and emulators stay) and opens the app again.
//!
//! UPD-004: the offer is a toast (main window and phone window) with three ways to update: Update
//! now (above), from the terminal (the page copies `tumr update`), or the installer from the
//! release page. The installer and the release notes open in the browser through [`open`], which
//! takes the address from the last check's manifest, never from the web view.

use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde_json::Value;

use crate::agents::tumr_exe;

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

/// The last check's result, so [`open`] can find the release's addresses.
static LAST: Mutex<Option<Value>> = Mutex::new(None);

/// `tumr update --check --json`: `{ current, channel, available: Release | null }`, or
/// `{ current, configured: false }` when no channel is set.
pub fn check() -> Result<Value, String> {
    let v = run_check()?;
    if let Ok(mut last) = LAST.lock() {
        *last = Some(v.clone());
    }
    Ok(v)
}

fn run_check() -> Result<Value, String> {
    let mut cmd = Command::new(tumr_exe()?);
    cmd.args(["update", "--check", "--json"]);
    #[cfg(windows)]
    no_window(&mut cmd);
    let out = cmd.output().map_err(|e| format!("cannot run tumr: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(text.trim()).map_err(|_| {
        let err = String::from_utf8_lossy(&out.stderr);
        err.trim().trim_start_matches("tumr: ").to_string()
    })
}

/// Starts `tumr update --yes --relaunch` on its own: it closes this app, so it must not be a child
/// that dies with it.
pub fn apply() -> Result<(), String> {
    let mut cmd = Command::new(tumr_exe()?);
    cmd.args(["update", "--yes", "--relaunch"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot start the update: {e}"))
}

/// The address for `what` (`notes` = the release notes, `installer` = the installer download) in a
/// check result, or why there is none. Only `https://` addresses are opened.
pub fn link(result: &Value, what: &str) -> Result<String, String> {
    let rel = result
        .get("available")
        .filter(|v| !v.is_null())
        .ok_or("no newer version to open")?;
    let url = match what {
        "notes" => rel
            .get("notes_url")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "installer" => rel
            .get("files")
            .and_then(Value::as_array)
            .and_then(|files| {
                files
                    .iter()
                    .find(|f| f.get("kind").and_then(Value::as_str) == Some("installer"))
            })
            .and_then(|f| f.get("url"))
            .and_then(Value::as_str)
            .unwrap_or_default(),
        _ => return Err(format!("unknown link '{what}'")),
    };
    if url.is_empty() {
        return Err(format!("this release has no {what} address"));
    }
    if !url.starts_with("https://") || url.chars().any(|c| c.is_whitespace() || c == '"') {
        return Err(format!("refused to open '{url}': not an https address"));
    }
    Ok(url.to_string())
}

/// UPD-004: the Windows notification text for the last check's newer version, or `None`.
pub fn notice() -> Option<(String, String)> {
    let last = LAST.lock().ok()?.clone()?;
    let rel = last.get("available").filter(|v| !v.is_null())?;
    let version = rel.get("version")?.as_str()?;
    let current = last
        .get("current")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    Some((
        format!("Version {version} is available"),
        format!("You have {current}. Open Toolchain Universal Mobile Runtime to update."),
    ))
}

/// Opens the release notes or the installer download of the last check in the default browser.
pub fn open(what: &str) -> Result<(), String> {
    let last = LAST
        .lock()
        .map_err(|_| "update state unavailable".to_string())?
        .clone()
        .ok_or("check for updates first")?;
    let url = link(&last, what)?;
    // explorer.exe hands an https address to the default browser; no shell is involved
    Command::new("explorer.exe")
        .arg(&url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot open {url}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::link;
    use serde_json::json;

    fn result(notes: &str, installer: &str) -> serde_json::Value {
        json!({ "current": "0.1.35", "available": { "version": "0.1.36", "notes_url": notes,
            "files": [ { "kind": "cli", "url": "https://example.org/cli.zip" },
                       { "kind": "installer", "url": installer } ] } })
    }

    #[test]
    fn picks_the_installer_and_the_notes() {
        let r = result(
            "https://github.com/o/r/releases/tag/v0.1.36",
            "https://github.com/o/r/releases/download/v0.1.36/setup.exe",
        );
        assert_eq!(
            link(&r, "installer").unwrap(),
            "https://github.com/o/r/releases/download/v0.1.36/setup.exe"
        );
        assert_eq!(
            link(&r, "notes").unwrap(),
            "https://github.com/o/r/releases/tag/v0.1.36"
        );
    }

    #[test]
    fn refuses_other_schemes_unknown_links_and_no_update() {
        let r = result("file:///C:/x", "http://example.org/setup.exe");
        assert!(link(&r, "notes").is_err());
        assert!(link(&r, "installer").is_err());
        assert!(link(&r, "anything").is_err());
        assert!(link(&json!({ "current": "0.1.35", "available": null }), "notes").is_err());
    }
}
