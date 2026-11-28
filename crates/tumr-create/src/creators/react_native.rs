//! React Native (Community CLI, TypeScript, with the native Android project): the official
//! `@react-native-community/cli init`, without installing packages or starting Git (the first
//! `tumr run` installs the packages). Never Expo.
//!
//! Owner decision CRT-D3: the template's welcome screen (`NewAppScreen` from
//! `@react-native/new-app-screen`) is replaced by a minimal `App.tsx` that shows the app name,
//! and that package is dropped; the official structure and configuration stay. The CLI writes
//! the Android sources to `com/<package id>` as one folder; TUMR moves them to the package path.
//! The project name for the CLI is the name in letters and digits (`HelloApp`); the display name
//! is the user's.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::{android, identifier, read, replace_in, tool, versions, write, Staging, Toolset};
use crate::job::Creator;
use crate::request::Validated;
use crate::Framework;

pub struct ReactNative {
    pub tools: Toolset,
}

const DEMO_PACKAGE: &str = "@react-native/new-app-screen";

fn app_tsx(display_name: &str) -> String {
    format!(
        r"/**
 * @format
 */

import {{ StatusBar, StyleSheet, Text, useColorScheme, View }} from 'react-native';
import {{ SafeAreaProvider }} from 'react-native-safe-area-context';

function App() {{
  const isDarkMode = useColorScheme() === 'dark';

  return (
    <SafeAreaProvider>
      <StatusBar barStyle={{isDarkMode ? 'light-content' : 'dark-content'}} />
      <View style={{styles.container}}>
        <Text style={{isDarkMode ? styles.dark : styles.light}}>{display_name}</Text>
      </View>
    </SafeAreaProvider>
  );
}}

const styles = StyleSheet.create({{
  container: {{
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  }},
  light: {{ color: '#000' }},
  dark: {{ color: '#fff' }},
}});

export default App;
"
    )
}

/// Drops the welcome screen's package from `package.json` (a line of its own in the template).
fn drop_demo_package(file: &Path) -> Result<(), String> {
    let text = read(file)?;
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| !l.contains(&format!("\"{DEMO_PACKAGE}\"")))
        .collect();
    let mut out = kept.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    let json: serde_json::Value = serde_json::from_str(&out).map_err(|e| {
        format!(
            "{} changed shape ({e}); TUMR's adjustment needs an update",
            file.display()
        )
    })?;
    if json["dependencies"].get(DEMO_PACKAGE).is_some() {
        return Err(format!("{DEMO_PACKAGE} is still in {}", file.display()));
    }
    write(file, &out)
}

impl Creator for ReactNative {
    fn framework(&self) -> Framework {
        Framework::ReactNative
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
        let ident = identifier(&v.name);
        let cli = format!("@react-native-community/cli@{}", versions::REACT_NATIVE_CLI);
        tool::run(
            "the React Native CLI (init)",
            &npx,
            &[
                "--yes",
                &cli,
                "init",
                &ident,
                "--version",
                versions::REACT_NATIVE,
                "--skip-install",
                "--skip-git-init",
                "--package-name",
                &v.package_id,
            ],
            &staging.dir,
            &self.tools,
            out,
            cancel,
        )?;
        let project = staging.dir.join(&ident);
        out("replacing the template's welcome screen with a minimal App.tsx");
        write(&project.join("App.tsx"), &app_tsx(&v.name))?;
        drop_demo_package(&project.join("package.json"))?;
        android::relocate_main_package(&project.join("android/app/src/main/java"), &v.package_id)?;
        if v.name != ident {
            replace_in(
                &project.join("app.json"),
                &format!("\"displayName\": \"{ident}\""),
                &format!("\"displayName\": \"{}\"", v.name),
            )?;
            replace_in(
                &project.join("android/app/src/main/res/values/strings.xml"),
                &format!(">{ident}<"),
                &format!(">{}<", v.name),
            )?;
        }
        super::move_contents(&project, target)
    }

    fn expected_files(&self, _v: &Validated) -> Vec<String> {
        [
            ".bundle/",
            ".eslintrc.js",
            ".gitignore",
            ".prettierrc.js",
            ".watchmanconfig",
            "App.tsx",
            "Gemfile",
            "README.md",
            "__tests__/",
            "android/",
            "ios/",
            "app.json",
            "babel.config.js",
            "index.js",
            "jest.config.js",
            "metro.config.js",
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
    fn the_welcome_package_is_dropped() {
        let d = std::env::temp_dir().join(format!("tumr-create-rn-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("package.json");
        std::fs::write(
            &f,
            "{\n  \"dependencies\": {\n    \"react\": \"19.3.0\",\n    \"@react-native/new-app-screen\": \"0.88.1\",\n    \"react-native-safe-area-context\": \"^5.5.2\"\n  }\n}\n",
        )
        .unwrap();
        drop_demo_package(&f).unwrap();
        let j: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert!(j["dependencies"].get(DEMO_PACKAGE).is_none());
        assert_eq!(j["dependencies"]["react"], "19.3.0");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn app_tsx_is_minimal() {
        let a = app_tsx("Hello App");
        assert!(a.contains(">Hello App</Text>") && !a.contains("NewAppScreen"));
    }
}
