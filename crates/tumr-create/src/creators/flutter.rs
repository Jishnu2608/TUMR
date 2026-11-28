//! Flutter: `flutter create --empty --platforms android`, the official minimal app (one
//! `MaterialApp` with "Hello World!"), without `pub get` (the first `tumr run` gets the packages).
//!
//! Flutter's application id is `<org>.<project_name>`, and a Dart project name is snake case
//! (`hello_app`). When the user's package id differs (`com.example.helloapp`), TUMR sets the
//! namespace and application id to it and moves `MainActivity` to its package, as Android Studio
//! does.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::{android, replace_in, snake, split_package, tool, Staging, Toolset};
use crate::job::Creator;
use crate::request::Validated;
use crate::Framework;

pub struct Flutter {
    pub tools: Toolset,
}

impl Creator for Flutter {
    fn framework(&self) -> Framework {
        Framework::Flutter
    }

    fn create(
        &self,
        v: &Validated,
        target: &Path,
        out: &mut dyn FnMut(&str),
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let flutter = self.tools.flutter()?;
        let staging = Staging::new(&self.tools.staging_root)?;
        let project = staging.dir.join("p");
        let dart_name = snake(&v.name);
        let (org, _) = split_package(&v.package_id);
        let project_arg = project.display().to_string();
        tool::run(
            "Flutter (flutter create)",
            &flutter,
            &[
                "create",
                "--empty",
                "--platforms",
                "android",
                "--org",
                org,
                "--project-name",
                &dart_name,
                "--no-pub",
                &project_arg,
            ],
            &staging.dir,
            &self.tools,
            out,
            cancel,
        )?;
        let flutter_id = format!("{org}.{dart_name}");
        if flutter_id != v.package_id {
            out(&format!(
                "setting the application id to {} (Flutter made {flutter_id})",
                v.package_id
            ));
            let gradle = project.join("android/app/build.gradle.kts");
            replace_in(
                &gradle,
                &format!("namespace = \"{flutter_id}\""),
                &format!("namespace = \"{}\"", v.package_id),
            )?;
            replace_in(
                &gradle,
                &format!("applicationId = \"{flutter_id}\""),
                &format!("applicationId = \"{}\"", v.package_id),
            )?;
            android::relocate_main_package(
                &project.join("android/app/src/main/kotlin"),
                &v.package_id,
            )?;
        }
        super::move_contents(&project, target)
    }

    fn expected_files(&self, v: &Validated) -> Vec<String> {
        let dart_name = snake(&v.name);
        [
            ".gitignore",
            ".metadata",
            ".idea/",
            "README.md",
            "analysis_options.yaml",
            "pubspec.yaml",
            "pubspec.lock",
            "lib/",
            "android/",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .chain([format!("{dart_name}.iml")])
        .collect()
    }
}
