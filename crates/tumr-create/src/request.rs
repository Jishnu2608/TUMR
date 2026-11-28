//! What the user chose, and its validation. Everything here is checked again by the engine right
//! before it writes anything (the inputs may come from a front end, flags or an old wizard state).
//!
//! Destinations, two ways (the brief's options A and B):
//! * [`Destination::ExistingEmpty`]: an existing folder that is empty; it becomes the project.
//! * [`Destination::NewIn`]: a parent folder; TUMR creates `<parent>\<project name>`.
//!
//! Rules: absolute paths only, no `..` parts, no Windows reserved names, never an existing
//! project or a non-empty folder.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Framework;

/// Where the project goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum Destination {
    /// An existing, empty folder becomes the project folder.
    ExistingEmpty { path: String },
    /// TUMR creates a folder named after the project inside `parent`.
    NewIn { parent: String },
}

/// A creation request from either front end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRequest {
    pub framework: Framework,
    /// The project's name, as the user typed it (also the folder name for [`Destination::NewIn`]).
    pub name: String,
    /// Android application id; derived from the name when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    pub destination: Destination,
    /// The coding agent chosen for the project (an agent client id); only recorded here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Trust the new folder so its build scripts may run (CRT-D1: the user's choice).
    #[serde(default)]
    pub trust: bool,
}

/// A request that passed validation, with everything derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Validated {
    pub framework: Framework,
    pub name: String,
    pub package_id: String,
    /// The project folder (exists and is empty, or does not exist yet).
    pub target: PathBuf,
    /// TUMR makes the folder (and may remove it again on failure).
    pub creates_folder: bool,
    pub agent: Option<String>,
    pub trust: bool,
    /// Things that do not stop creation but are worth saying (a long path for native builds).
    pub warnings: Vec<String>,
}

const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Java keywords and literals: not allowed as a package segment.
const JAVA_WORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
    "true",
    "false",
    "null",
    "var",
    "record",
    "yield",
];

/// Above this length a project path risks Windows' 260-character limit in native C++ builds
/// (React Native and Expo modules; RN-001).
const LONG_PATH: usize = 60;

/// Checks a project name: 1–64 characters, starts with a letter, then letters, digits, spaces,
/// `-` or `_`; no trailing space or dot; not a Windows reserved name.
pub fn check_name(name: &str) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("the project name is empty".into());
    }
    if n.chars().count() > 64 {
        return Err("the project name is longer than 64 characters".into());
    }
    if !n.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err("the project name must start with a letter (A–Z)".into());
    }
    if let Some(c) = n
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_')))
    {
        return Err(format!(
            "the project name may use letters, digits, spaces, '-' and '_' only (not '{c}')"
        ));
    }
    if n.ends_with('.') || name.ends_with(' ') {
        return Err("the project name must not end with a dot or a space".into());
    }
    if RESERVED.contains(&n.to_ascii_lowercase().as_str()) {
        return Err(format!("'{n}' is a reserved name on Windows"));
    }
    Ok(())
}

/// Checks an Android application id: at least two dot-separated segments, each starting with a
/// letter and made of letters, digits and `_`, none a Java keyword.
pub fn check_package_id(id: &str) -> Result<(), String> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return Err(format!(
            "'{id}' needs at least two parts separated by dots, for example com.example.app"
        ));
    }
    if id.len() > 150 {
        return Err("the package id is longer than 150 characters".into());
    }
    for p in parts {
        if !p.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return Err(format!(
                "each part of the package id must start with a letter ('{p}' in '{id}')"
            ));
        }
        if !p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!(
                "the package id may use letters, digits and '_' in each part ('{p}' in '{id}')"
            ));
        }
        if JAVA_WORDS.contains(&p) {
            return Err(format!(
                "'{p}' is a Java keyword and cannot be part of the package id"
            ));
        }
    }
    Ok(())
}

/// The package id TUMR suggests for a name: `com.example.<letters and digits, lower case>`.
#[must_use]
pub fn default_package_id(name: &str) -> String {
    let mut last: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    if !last.starts_with(|c: char| c.is_ascii_alphabetic()) {
        last.insert(0, 'a');
    }
    if JAVA_WORDS.contains(&last.as_str()) {
        last.push_str("app");
    }
    format!("com.example.{last}")
}

/// An absolute path without `..` or other relative parts.
fn plain_absolute(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path.trim().trim_matches('"'));
    if p.as_os_str().is_empty() {
        return Err("no folder given".into());
    }
    if !p.is_absolute() {
        return Err(format!(
            "'{}' is not a full path (for example C:\\Projects)",
            p.display()
        ));
    }
    if p.components()
        .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(format!("'{}' contains '.' or '..' parts", p.display()));
    }
    for c in p.components() {
        if let Component::Normal(s) = c {
            let seg = s.to_string_lossy().to_ascii_lowercase();
            let stem = seg.split('.').next().unwrap_or("");
            if RESERVED.contains(&stem) {
                return Err(format!("'{seg}' is a reserved name on Windows"));
            }
        }
    }
    Ok(p)
}

fn is_empty_dir(p: &Path) -> Result<bool, String> {
    let mut rd = std::fs::read_dir(p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
    Ok(rd.next().is_none())
}

/// An existing folder's real path: 8.3 short names (`C:\Users\JOHNSM~1`) become the long ones,
/// so trust, recent projects and metadata record the path TUMR finds later, whichever front end
/// sent the request (CRT-F found the CLI and the desktop disagreeing). No `\\?\` prefix.
pub(crate) fn real_path(p: &Path) -> PathBuf {
    match std::fs::canonicalize(p) {
        Ok(r) => {
            let s = r.display().to_string();
            PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s))
        }
        Err(_) => p.to_path_buf(),
    }
}

/// Validates a request against the file system as it is now.
pub fn validate(r: &CreateRequest) -> Result<Validated, String> {
    check_name(&r.name)?;
    let name = r.name.trim().to_string();
    let package_id = match r.package_id.as_deref().map(str::trim) {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => default_package_id(&name),
    };
    check_package_id(&package_id)?;
    let (target, creates_folder) = match &r.destination {
        Destination::ExistingEmpty { path } => {
            let p = plain_absolute(path)?;
            if !p.is_dir() {
                return Err(format!("{} is not an existing folder", p.display()));
            }
            if !is_empty_dir(&p)? {
                return Err(format!(
                    "{} is not empty; choose an empty folder or a parent folder for a new one",
                    p.display()
                ));
            }
            (real_path(&p), false)
        }
        Destination::NewIn { parent } => {
            let p = plain_absolute(parent)?;
            if !p.is_dir() {
                return Err(format!("{} is not an existing folder", p.display()));
            }
            let t = real_path(&p).join(&name);
            if t.exists() {
                return Err(format!(
                    "{} already exists; TUMR never overwrites an existing folder",
                    t.display()
                ));
            }
            (t, true)
        }
    };
    let mut warnings = Vec::new();
    let len = target.display().to_string().chars().count();
    if len > LONG_PATH && matches!(r.framework, Framework::ReactNative | Framework::Expo) {
        warnings.push(format!(
            "the project path is {len} characters long; native C++ builds of React Native modules can hit Windows' 260-character limit (a short folder such as C:\\dev is safer)"
        ));
    }
    if let Some(a) = &r.agent {
        if a.is_empty()
            || !a
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!("'{a}' is not an agent id"));
        }
    }
    Ok(Validated {
        framework: r.framework,
        name,
        package_id,
        target,
        creates_folder,
        agent: r.agent.clone(),
        trust: r.trust,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_become_the_real_path() {
        // TEMP is often an 8.3 path (C:\Users\JOHNSM~1\…); the target is always the long one
        let raw =
            std::env::temp_dir().join(format!("tumr-create-req-short-{}", std::process::id()));
        std::fs::create_dir_all(&raw).unwrap();
        let v = validate(&req(Destination::NewIn {
            parent: raw.display().to_string(),
        }))
        .unwrap();
        let real = std::fs::canonicalize(&raw).unwrap().display().to_string();
        let real = real.trim_start_matches(r"\\?\").to_string();
        assert_eq!(v.target, PathBuf::from(real).join("My App"));
        assert!(
            !v.target.display().to_string().contains('~'),
            "{}",
            v.target.display()
        );
        let _ = std::fs::remove_dir_all(&raw);
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tumr-create-req-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // the engine records real (long) paths; TEMP may hold an 8.3 short name
        crate::request::real_path(&d)
    }

    fn req(dest: Destination) -> CreateRequest {
        CreateRequest {
            framework: Framework::Kotlin,
            name: "My App".into(),
            package_id: None,
            destination: dest,
            agent: Some("claude-code".into()),
            trust: true,
        }
    }

    #[test]
    fn names() {
        for ok in ["MyCalculator", "my app", "Expense-Tracker_2"] {
            assert!(check_name(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "2fast",
            "a/b",
            "my.app",
            "con",
            "naïve",
            "x ",
            &"a".repeat(65),
        ] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn package_ids() {
        assert!(check_package_id("com.example.app").is_ok());
        assert!(check_package_id("dev.tumr.my_app2").is_ok());
        for bad in [
            "app",
            "com..app",
            "com.1app",
            "com.example.class",
            "com.ex-ample.app",
        ] {
            assert!(check_package_id(bad).is_err(), "{bad}");
        }
        assert_eq!(
            default_package_id("My Calculator 2"),
            "com.example.mycalculator2"
        );
        assert_eq!(default_package_id("Class"), "com.example.classapp");
        assert!(check_package_id(&default_package_id("New")).is_ok());
    }

    #[test]
    fn new_folder_in_a_parent_with_spaces() {
        let parent = tmp("parent with spaces");
        let v = validate(&req(Destination::NewIn {
            parent: parent.display().to_string(),
        }))
        .unwrap();
        assert_eq!(v.target, parent.join("My App"));
        assert!(v.creates_folder);
        assert_eq!(v.package_id, "com.example.myapp");
        // an existing folder of that name is never reused
        std::fs::create_dir_all(parent.join("My App")).unwrap();
        let e = validate(&req(Destination::NewIn {
            parent: parent.display().to_string(),
        }))
        .unwrap_err();
        assert!(e.contains("already exists"), "{e}");
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn existing_folder_must_be_empty() {
        let d = tmp("existing");
        let v = validate(&req(Destination::ExistingEmpty {
            path: d.display().to_string(),
        }))
        .unwrap();
        assert_eq!(v.target, d);
        assert!(!v.creates_folder);
        std::fs::write(d.join("notes.txt"), b"x").unwrap();
        assert!(validate(&req(Destination::ExistingEmpty {
            path: d.display().to_string()
        }))
        .unwrap_err()
        .contains("not empty"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unsafe_paths_are_refused() {
        for bad in [
            r"relative\folder",
            r"C:\Projects\..\Windows",
            r"C:\Projects\NUL",
            "",
        ] {
            assert!(
                validate(&req(Destination::NewIn { parent: bad.into() })).is_err(),
                "{bad}"
            );
        }
        let mut r = req(Destination::NewIn {
            parent: std::env::temp_dir().display().to_string(),
        });
        r.agent = Some("claude code; rm".into());
        assert!(validate(&r).is_err());
    }

    #[test]
    fn long_paths_warn_for_react_native() {
        let parent = tmp("a-rather-long-parent-folder-name-for-native-builds");
        let mut r = req(Destination::NewIn {
            parent: parent.display().to_string(),
        });
        r.framework = Framework::ReactNative;
        let v = validate(&r).unwrap();
        assert_eq!(v.warnings.len(), 1, "{:?}", v.warnings);
        r.framework = Framework::Kotlin;
        assert_eq!(validate(&r).unwrap().warnings, Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&parent);
    }
}
