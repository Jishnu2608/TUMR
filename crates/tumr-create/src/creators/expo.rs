//! Expo: `create-expo-app` with the `blank-typescript` template, for Expo Go (no `expo prebuild`,
//! no native folders: owner decision CRT-D4), without installing packages (the first `tumr run`
//! installs them).
//!
//! Owner decision CRT-D2: no `AGENTS.md` or `.claude/settings.json` (`--no-agents-md`) and no
//! `LICENSE`. The Git repository `create-expo-app` starts is removed too: no TUMR creator starts
//! one, so the user or their agent decides. `app.json` gets the display name and the Android
//! package id (Expo Go ignores it; a development build later needs it).

use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::{read, remove, replace_in, slug, tool, versions, Staging, Toolset};
use crate::job::Creator;
use crate::request::Validated;
use crate::Framework;

pub struct Expo {
    pub tools: Toolset,
}

/// Sets the display name and the Android package in `app.json`, then reads it back.
fn adjust_app_json(file: &Path, slug: &str, v: &Validated) -> Result<(), String> {
    let name = serde_json::to_string(&v.name).map_err(|e| e.to_string())?;
    replace_in(
        file,
        &format!("\"name\": \"{slug}\""),
        &format!("\"name\": {name}"),
    )?;
    let package = serde_json::to_string(&v.package_id).map_err(|e| e.to_string())?;
    replace_in(
        file,
        "\"android\": {",
        &format!("\"android\": {{\n      \"package\": {package},"),
    )?;
    let json: serde_json::Value = serde_json::from_str(&read(file)?)
        .map_err(|e| format!("{} is no longer valid JSON: {e}", file.display()))?;
    let ok = json["expo"]["name"] == v.name.as_str()
        && json["expo"]["android"]["package"] == v.package_id.as_str();
    if ok {
        Ok(())
    } else {
        Err(format!(
            "{} did not take the name and package",
            file.display()
        ))
    }
}

impl Creator for Expo {
    fn framework(&self) -> Framework {
        Framework::Expo
    }

    fn create(
        &self,
        v: &Validated,
        target: &Path,
        out: &mut dyn FnMut(&str),
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let npx = self.tools.npx()?;
        let staging = Staging::new(&self.tools.staging_root)?;
        let slug = slug(&v.name);
        let package = format!("create-expo-app@{}", versions::CREATE_EXPO_APP);
        tool::run(
            "create-expo-app",
            &npx,
            &[
                "--yes",
                &package,
                &slug,
                "--template",
                "blank-typescript",
                "--no-install",
                "--no-agents-md",
            ],
            &staging.dir,
            &self.tools,
            out,
            cancel,
        )?;
        let project = staging.dir.join(&slug);
        for extra in ["LICENSE", ".git", "AGENTS.md", ".claude"] {
            remove(&project.join(extra))?;
        }
        out("set up for Expo Go: no native folders; packages are installed on the first run");
        adjust_app_json(&project.join("app.json"), &slug, v)?;
        super::move_contents(&project, target)
    }

    fn expected_files(&self, _v: &Validated) -> Vec<String> {
        [
            ".gitignore",
            "App.tsx",
            "app.json",
            "assets/",
            "index.ts",
            "package.json",
            "tsconfig.json",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_json_gets_name_and_package() {
        let d = std::env::temp_dir().join(format!("tumr-create-expo-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("app.json");
        std::fs::write(
            &f,
            "{\n  \"expo\": {\n    \"name\": \"hello-app\",\n    \"slug\": \"hello-app\",\n    \"android\": {\n      \"adaptiveIcon\": {}\n    }\n  }\n}\n",
        )
        .unwrap();
        let v = Validated {
            framework: Framework::Expo,
            name: "Hello App".into(),
            package_id: "com.example.helloapp".into(),
            target: d.clone(),
            creates_folder: true,
            agent: None,
            trust: false,
            warnings: Vec::new(),
        };
        adjust_app_json(&f, "hello-app", &v).unwrap();
        let j: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(j["expo"]["slug"], "hello-app");
        assert_eq!(j["expo"]["android"]["adaptiveIcon"], serde_json::json!({}));
        let _ = std::fs::remove_dir_all(&d);
    }
}
