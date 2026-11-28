//! Kotlin (native Android, Jetpack Compose): Google's Android CLI, `android create
//! empty-activity`, run with TUMR's isolation and `--no-metrics`.
//!
//! The template is more than a blank scaffold: a sample list screen with a view model, a data
//! repository, navigation and tests for them. TUMR keeps the official project (Gradle files,
//! version catalogue, theme, resources, manifest) and replaces the sample with a minimal
//! `MainActivity` that shows the app name (owner constraint: a scaffold, never a sample app),
//! as for the React Native welcome screen (CRT-D3).

use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::{package_path, read, remove, tool, write, Staging, Toolset};
use crate::job::Creator;
use crate::request::Validated;
use crate::Framework;

pub struct Kotlin {
    pub tools: Toolset,
}

/// Folder of the Kotlin sources (`app/src/main/kotlin/com/example/app`).
fn src(package_id: &str) -> String {
    format!("app/src/main/kotlin/{}", package_path(package_id))
}

/// The theme function the template generates (`HelloAppTheme`), read from `ui/theme/Theme.kt`.
fn theme_name(theme_kt: &str) -> Option<String> {
    theme_kt.lines().find_map(|l| {
        let rest = l.trim_start().strip_prefix("fun ")?;
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        (name.ends_with("Theme") && rest[name.len()..].starts_with('(')).then_some(name)
    })
}

fn main_activity(package_id: &str, theme: &str) -> String {
    format!(
        r"package {package_id}

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import {package_id}.ui.theme.{theme}

class MainActivity : ComponentActivity() {{
  override fun onCreate(savedInstanceState: Bundle?) {{
    super.onCreate(savedInstanceState)

    enableEdgeToEdge()
    setContent {{
      {theme} {{
        Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {{
          Box(contentAlignment = Alignment.Center) {{
            Text(stringResource(R.string.app_name))
          }}
        }}
      }}
    }}
  }}
}}
"
    )
}

/// Replaces the sample screen with a minimal one; the rest of the project stays as generated.
fn trim(project: &Path, package_id: &str) -> Result<(), String> {
    let src = project.join(src(package_id));
    for sample in ["Navigation.kt", "NavigationKeys.kt", "data", "ui/main"] {
        remove(&src.join(sample))?;
    }
    // the template's tests exercise the sample screen only
    remove(&project.join("app/src/test"))?;
    remove(&project.join("app/src/androidTest"))?;
    let theme_kt = src.join("ui/theme/Theme.kt");
    let theme = theme_name(&read(&theme_kt)?)
        .ok_or_else(|| format!("{} has changed: no theme function", theme_kt.display()))?;
    write(
        &src.join("MainActivity.kt"),
        &main_activity(package_id, &theme),
    )
}

impl Creator for Kotlin {
    fn framework(&self) -> Framework {
        Framework::Kotlin
    }

    fn create(
        &self,
        v: &Validated,
        target: &Path,
        out: &mut dyn FnMut(&str),
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let android = self.tools.android_cli()?;
        let staging = Staging::new(&self.tools.staging_root)?;
        let project = staging.dir.join("p");
        let name = format!("--name={}", v.name);
        let id = format!("--application-id={}", v.package_id);
        let ns = format!("--namespace={}", v.package_id);
        let output = format!("--output={}", project.display());
        tool::run(
            "the Android CLI (android create)",
            &android,
            &[
                "create",
                "empty-activity",
                &name,
                &id,
                &ns,
                &output,
                "--no-metrics",
            ],
            &staging.dir,
            &self.tools,
            out,
            cancel,
        )?;
        out("replacing the template's sample screen with a minimal one");
        trim(&project, &v.package_id)?;
        super::move_contents(&project, target)
    }

    fn expected_files(&self, v: &Validated) -> Vec<String> {
        let src = src(&v.package_id);
        [
            ".gitignore",
            "build.gradle.kts",
            "settings.gradle.kts",
            "gradle.properties",
            "local.properties",
            "gradlew",
            "gradlew.bat",
            "gradle/",
            "app/.gitignore",
            "app/build.gradle.kts",
            "app/src/main/AndroidManifest.xml",
            "app/src/main/res/",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .chain([format!("{src}/MainActivity.kt"), format!("{src}/ui/theme/")])
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_function_is_found() {
        let t = "package x\n\n@Composable\nfun HelloAppTheme(\n  darkTheme: Boolean,\n) {}\nfun other() {}\n";
        assert_eq!(theme_name(t).as_deref(), Some("HelloAppTheme"));
        assert_eq!(theme_name("fun Theme2() {}"), None);
    }

    #[test]
    fn trim_keeps_the_project_and_drops_the_sample() {
        let d = std::env::temp_dir().join(format!("tumr-create-kotlin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = d.join(src("com.example.app"));
        for f in [
            "Navigation.kt",
            "data/DataRepository.kt",
            "ui/main/MainScreen.kt",
            "MainActivity.kt",
        ] {
            write(&s.join(f), "x").unwrap();
        }
        write(&s.join("ui/theme/Theme.kt"), "fun AppTheme(\n) {}\n").unwrap();
        write(&d.join("app/src/test/kotlin/T.kt"), "x").unwrap();
        trim(&d, "com.example.app").unwrap();
        assert!(!s.join("data").exists() && !s.join("ui/main").exists());
        assert!(!d.join("app/src/test").exists());
        let main = std::fs::read_to_string(s.join("MainActivity.kt")).unwrap();
        assert!(
            main.contains("AppTheme {")
                && main.contains("import com.example.app.ui.theme.AppTheme")
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
