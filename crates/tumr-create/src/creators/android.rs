//! Android project adjustments shared by the creators.

use std::path::{Path, PathBuf};

use super::{package_path, read, write};

/// Puts the folder holding `MainActivity` (under `src_root`, a `java` or `kotlin` source folder)
/// at the package id's path and sets the `package` line of its source files. Flutter names the
/// package `<org>.<project_name>`; the React Native CLI 21 writes `com/<package id>` as one
/// folder. Both are moved to `com/example/app`.
pub fn relocate_main_package(src_root: &Path, package_id: &str) -> Result<(), String> {
    let found = find_main_activity(src_root)
        .ok_or_else(|| format!("no MainActivity under {}", src_root.display()))?;
    let from = found
        .parent()
        .map(Path::to_path_buf)
        .ok_or("MainActivity has no folder")?;
    let to = src_root.join(package_path(package_id));
    if from != to {
        std::fs::create_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()))?;
        for entry in std::fs::read_dir(&from).map_err(|e| format!("{}: {e}", from.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            std::fs::rename(entry.path(), to.join(entry.file_name()))
                .map_err(|e| format!("cannot move {}: {e}", entry.path().display()))?;
        }
        // remove the old folder and the parents it leaves empty
        let mut dir = from;
        while dir != src_root && dir.starts_with(src_root) {
            if std::fs::remove_dir(&dir).is_err() {
                break;
            }
            match dir.parent() {
                Some(p) => dir = p.to_path_buf(),
                None => break,
            }
        }
    }
    for entry in std::fs::read_dir(&to).map_err(|e| format!("{}: {e}", to.display()))? {
        let p = entry.map_err(|e| e.to_string())?.path();
        let is_source = p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("kt") || x.eq_ignore_ascii_case("java"));
        if is_source {
            set_package_line(&p, package_id)?;
        }
    }
    Ok(())
}

fn find_main_activity(dir: &Path) -> Option<PathBuf> {
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            dirs.push(p);
        } else if name == "MainActivity.kt" || name == "MainActivity.java" {
            return Some(p);
        }
    }
    dirs.iter().find_map(|d| find_main_activity(d))
}

/// Sets the `package` line (Kotlin without `;`, Java with it).
fn set_package_line(file: &Path, package_id: &str) -> Result<(), String> {
    let text = read(file)?;
    let java = file.extension().is_some_and(|x| x == "java");
    let mut changed = false;
    let lines: Vec<String> = text
        .lines()
        .map(|l| {
            if !changed && l.trim_start().starts_with("package ") {
                changed = true;
                if java {
                    format!("package {package_id};")
                } else {
                    format!("package {package_id}")
                }
            } else {
                l.to_string()
            }
        })
        .collect();
    if !changed {
        return Err(format!("{} has no package line", file.display()));
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    write(file, &out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tumr-create-pkg-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn flutter_style_package_is_moved_and_renamed() {
        let src = dir("flutter");
        let old = src.join("com/example/hello_app");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(
            old.join("MainActivity.kt"),
            "package com.example.hello_app\n\nclass MainActivity\n",
        )
        .unwrap();
        relocate_main_package(&src, "com.example.helloapp").unwrap();
        let moved = src.join("com/example/helloapp/MainActivity.kt");
        assert_eq!(
            std::fs::read_to_string(moved).unwrap(),
            "package com.example.helloapp\n\nclass MainActivity\n"
        );
        assert!(!src.join("com/example/hello_app").exists());
        assert!(
            src.join("com/example").is_dir(),
            "a parent still in use stays"
        );
        let _ = std::fs::remove_dir_all(&src);
    }

    #[test]
    fn react_native_dotted_folder_is_fixed() {
        let src = dir("rn");
        let old = src.join("com").join("com.example.probern");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("MainActivity.kt"), "package com.example.probern\n").unwrap();
        std::fs::write(
            old.join("MainApplication.kt"),
            "package com.example.probern\n",
        )
        .unwrap();
        relocate_main_package(&src, "com.example.probern").unwrap();
        assert!(src.join("com/example/probern/MainApplication.kt").is_file());
        assert!(!src.join("com").join("com.example.probern").exists());
        let _ = std::fs::remove_dir_all(&src);
    }

    #[test]
    fn java_package_line_keeps_its_semicolon() {
        let src = dir("java");
        let p = src.join("com/x/y");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("MainActivity.java"), "package com.x.y;\n").unwrap();
        relocate_main_package(&src, "com.x.z").unwrap();
        assert_eq!(
            std::fs::read_to_string(src.join("com/x/z/MainActivity.java")).unwrap(),
            "package com.x.z;\n"
        );
        let _ = std::fs::remove_dir_all(&src);
    }
}
