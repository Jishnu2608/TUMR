//! The framework creators (CRT-B): one per framework, each running the framework's own official
//! tool (Java: TUMR's template) with TUMR's isolation environment, then trimming what the tool
//! makes to a minimal scaffold (owner decisions CRT-D2, D3, D4, D6).
//!
//! Every tool writes into a fresh staging folder under the managed `tmp` (tools refuse or behave
//! differently with an existing folder, and a failed tool then never touches the project folder).
//! The result is moved into the project folder, which the engine created empty.
//!
//! The Runtime Core builds the [`Toolset`] from the plan for the framework: the tool locations it
//! would reuse and the child environment (`tumr_env::recipe::child_env`).

mod android;
mod expo;
mod flutter;
mod java;
mod kotlin;
mod names;
mod react_native;
mod tool;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::job::Creator;
use crate::Framework;

pub use names::{identifier, slug, snake};

/// Versions of the creation tools this TUMR version uses (checked on this PC 2026-11-28).
pub mod versions {
    /// `create-expo-app` (Expo's official project creator).
    pub const CREATE_EXPO_APP: &str = "5.0.0";
    /// `@react-native-community/cli` (React Native's official `init`).
    pub const REACT_NATIVE_CLI: &str = "21.0.0";
    /// React Native release the CLI creates.
    pub const REACT_NATIVE: &str = "0.88.1";
    /// Android Gradle plugin of the Java template (the Android CLI's own template uses it).
    pub const JAVA_AGP: &str = "9.0.1";
    /// `compileSdk` / `targetSdk` of the Java template (as the Android CLI's template).
    pub const JAVA_COMPILE_SDK: u32 = 37;
}

/// Where the creation tools are and how to run them.
#[derive(Debug, Clone, Default)]
pub struct Toolset {
    /// Variables for every tool (TUMR's child environment: `PATH`, `JAVA_HOME`, isolation folders).
    pub env: HashMap<String, String>,
    /// Folder holding `node.exe` and `npx.cmd`.
    pub node_dir: Option<PathBuf>,
    /// The Flutter SDK (holds `bin\flutter.bat`).
    pub flutter_root: Option<PathBuf>,
    /// The Android SDK (the Android CLI is `cmdline-tools\latest\bin\android.exe`).
    pub android_sdk: Option<PathBuf>,
    /// Parent of the staging folders (the managed `tmp`).
    pub staging_root: PathBuf,
}

impl Toolset {
    fn npx(&self) -> Result<PathBuf, String> {
        let dir = self
            .node_dir
            .as_ref()
            .ok_or("Node.js is not installed for TUMR")?;
        existing(dir.join("npx.cmd"), "npx")
    }

    fn flutter(&self) -> Result<PathBuf, String> {
        let root = self
            .flutter_root
            .as_ref()
            .ok_or("the Flutter SDK is not installed for TUMR")?;
        existing(root.join("bin").join("flutter.bat"), "flutter")
    }

    fn android_cli(&self) -> Result<PathBuf, String> {
        let sdk = self
            .android_sdk
            .as_ref()
            .ok_or("the Android SDK is not installed for TUMR")?;
        existing(
            sdk.join("cmdline-tools")
                .join("latest")
                .join("bin")
                .join("android.exe"),
            "the Android CLI",
        )
    }
}

fn existing(p: PathBuf, what: &str) -> Result<PathBuf, String> {
    if p.is_file() {
        Ok(p)
    } else {
        Err(format!("{what} was not found at {}", p.display()))
    }
}

/// The creator for `framework`.
#[must_use]
pub fn creator_for(framework: Framework, tools: Toolset) -> Box<dyn Creator> {
    match framework {
        Framework::Expo => Box::new(expo::Expo { tools }),
        Framework::ReactNative => Box::new(react_native::ReactNative { tools }),
        Framework::Flutter => Box::new(flutter::Flutter { tools }),
        Framework::Kotlin => Box::new(kotlin::Kotlin { tools }),
        Framework::Java => Box::new(java::Java),
    }
}

/// A fresh folder for one tool run, removed when dropped.
struct Staging {
    dir: PathBuf,
}

impl Staging {
    fn new(root: &Path) -> Result<Self, String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = root.join(format!("create-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        Ok(Self { dir })
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        // a tool that was just ended may still hold a file or its working folder for a moment
        for _ in 0..20 {
            if std::fs::remove_dir_all(&self.dir).is_ok() || !self.dir.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
}

/// Moves everything inside `from` into `to` (rename; copy and remove across volumes).
fn move_contents(from: &Path, to: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let dest = to.join(entry.file_name());
        if std::fs::rename(entry.path(), &dest).is_err() {
            copy_tree(&entry.path(), &dest)?;
        }
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let err =
        |e: std::io::Error| format!("cannot copy {} to {}: {e}", from.display(), to.display());
    if from.is_dir() {
        std::fs::create_dir_all(to).map_err(err)?;
        for entry in std::fs::read_dir(from).map_err(err)? {
            let entry = entry.map_err(err)?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(from, to).map_err(err)?;
    }
    Ok(())
}

/// Replaces `from` with `to` in a text file; the text must be there (else the template changed).
fn replace_in(file: &Path, from: &str, to: &str) -> Result<(), String> {
    let text = read(file)?;
    if !text.contains(from) {
        return Err(format!(
            "{} has changed: `{from}` is not in it (TUMR's adjustments need an update)",
            file.display()
        ));
    }
    write(file, &text.replacen(from, to, 1))
}

fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))
}

fn write(file: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(file, text).map_err(|e| format!("cannot write {}: {e}", file.display()))
}

/// Removes a file or folder the tool made but the minimal scaffold does not keep.
fn remove(path: &Path) -> Result<(), String> {
    let r = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else if path.exists() {
        std::fs::remove_file(path)
    } else {
        return Ok(());
    };
    r.map_err(|e| format!("cannot remove {}: {e}", path.display()))
}

/// `com.example.app` as a relative folder (`com/example/app`).
fn package_path(package_id: &str) -> String {
    package_id.replace('.', "/")
}

/// The package id without its last segment (`com.example`), and that segment.
fn split_package(package_id: &str) -> (&str, &str) {
    package_id
        .rsplit_once('.')
        .unwrap_or(("com.example", package_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_helpers() {
        assert_eq!(package_path("com.example.app"), "com/example/app");
        assert_eq!(
            split_package("com.example.my_app"),
            ("com.example", "my_app")
        );
    }

    #[test]
    fn replace_in_refuses_a_changed_template() {
        let d = std::env::temp_dir().join(format!("tumr-create-replace-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("a.txt");
        std::fs::write(&f, "namespace = \"x\"").unwrap();
        replace_in(&f, "\"x\"", "\"y\"").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "namespace = \"y\"");
        assert!(replace_in(&f, "\"x\"", "\"z\"")
            .unwrap_err()
            .contains("has changed"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn move_contents_moves_files_and_folders() {
        let d = std::env::temp_dir().join(format!("tumr-create-move-{}", std::process::id()));
        let (a, b) = (d.join("a"), d.join("b"));
        std::fs::create_dir_all(a.join("x/y")).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("x/y/z.txt"), "1").unwrap();
        std::fs::write(a.join("top.txt"), "2").unwrap();
        move_contents(&a, &b).unwrap();
        assert!(b.join("x/y/z.txt").is_file() && b.join("top.txt").is_file());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_tools_are_named() {
        let t = Toolset::default();
        assert!(t.npx().unwrap_err().contains("Node.js"));
        assert!(t.flutter().unwrap_err().contains("Flutter"));
        assert!(t.android_cli().unwrap_err().contains("Android SDK"));
    }
}
