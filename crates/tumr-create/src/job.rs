//! The staged creation run (CRT-A): validate again → prepare the folder → scaffold (the
//! framework's creator) → metadata → verify (TUMR's own detection). The Runtime Core adds the
//! stages around it (tool check before, registration after) and reports all of them.
//!
//! Failure or cancel rolls back only what this run made: the folder it created, or (for an
//! existing empty folder) the entries inside it, never the folder itself and never anything that
//! was there before. A created project is never reported until it passed verification.
//!
//! Verify also lists files the creator did not say it makes ([`Creator::expected_files`]). They do
//! not fail the creation, but the Core does not trust such a project on its own: the user confirms
//! first (owner decision CRT-D1).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::metadata::{self, ProjectMetadata};
use crate::request::{self, CreateRequest, Validated};
use crate::Framework;

/// The stages a creation goes through, in order (the Core reports `CheckTools` and `Register`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Validate,
    CheckTools,
    Prepare,
    Scaffold,
    Metadata,
    Verify,
    Register,
}

/// One progress line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub stage: Stage,
    pub message: String,
}

/// A framework's creator: makes the scaffold in `target`, which exists and is empty.
pub trait Creator: Send + Sync {
    fn framework(&self) -> Framework;
    /// Creates the project files. `out` receives the tool's progress lines; `cancel` is checked
    /// between steps (a creator running a tool stops it when set).
    fn create(
        &self,
        v: &Validated,
        target: &Path,
        out: &mut dyn FnMut(&str),
        cancel: &AtomicBool,
    ) -> Result<(), String>;
    /// What the creator makes, relative to the project folder, with `/` separators: a file path
    /// (`app/build.gradle`), or a folder ending in `/` that covers everything inside it
    /// (`node_modules/`). TUMR's own `.tumr/` is always expected. Anything else found after the
    /// creation is reported as unexpected.
    fn expected_files(&self, v: &Validated) -> Vec<String>;
}

/// How many unexpected paths a result lists (the count is always complete).
pub const UNEXPECTED_LISTED: usize = 50;

/// A finished, verified project.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Created {
    pub path: PathBuf,
    pub framework: Framework,
    pub package_id: String,
    pub metadata: PathBuf,
    /// TUMR's detection confidence for the new project (0–100).
    pub confidence: u8,
    pub warnings: Vec<String>,
    /// Paths the creator did not say it makes (relative, `/` separators, a folder ends in `/`),
    /// at most [`UNEXPECTED_LISTED`]. Not empty: the project is not trusted without the user's
    /// confirmation.
    pub unexpected: Vec<String>,
    /// How many unexpected paths there are in all.
    pub unexpected_count: usize,
}

/// Why a creation stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateError {
    pub stage: Stage,
    pub message: String,
    pub cancelled: bool,
    /// What the rollback did (or why it left something in place).
    pub cleanup: String,
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {} ({})", self.stage, self.message, self.cleanup)
    }
}

/// Undoes what this run made, if it does not finish.
struct Rollback {
    target: PathBuf,
    created_folder: bool,
    armed: bool,
}

impl Rollback {
    fn undo(&mut self) -> String {
        self.armed = false;
        if self.created_folder {
            return match std::fs::remove_dir_all(&self.target) {
                Ok(()) => format!("removed the folder it created ({})", self.target.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => "nothing to remove".into(),
                Err(e) => format!(
                    "could not remove {} ({e}); delete it by hand",
                    self.target.display()
                ),
            };
        }
        // an existing folder that was empty: remove what is inside now, keep the folder
        let mut left = Vec::new();
        for entry in std::fs::read_dir(&self.target)
            .into_iter()
            .flatten()
            .flatten()
        {
            let p = entry.path();
            let r = if entry.file_type().is_ok_and(|t| t.is_dir()) {
                std::fs::remove_dir_all(&p)
            } else {
                std::fs::remove_file(&p)
            };
            if r.is_err() {
                left.push(p.display().to_string());
            }
        }
        if left.is_empty() {
            format!(
                "emptied {} again (the folder was empty before)",
                self.target.display()
            )
        } else {
            format!("could not remove {}; delete them by hand", left.join(", "))
        }
    }
}

impl Drop for Rollback {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.undo();
        }
    }
}

/// Runs a creation with `creator` (it must be the request's framework).
pub fn run(
    req: &CreateRequest,
    creator: &dyn Creator,
    tumr_version: &str,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<Created, CreateError> {
    let fail = |stage, message: String| CreateError {
        stage,
        message,
        cancelled: false,
        cleanup: "nothing was created".into(),
    };
    let mut say = |stage, message: String| progress(Progress { stage, message });

    // 1. validate again, right before anything is written
    say(
        Stage::Validate,
        "checking the project name and folder".into(),
    );
    let v = request::validate(req).map_err(|e| fail(Stage::Validate, e))?;
    if creator.framework() != v.framework {
        return Err(fail(
            Stage::Validate,
            format!(
                "internal error: the {} creator was given a {} project",
                creator.framework().name(),
                v.framework.name()
            ),
        ));
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(CreateError {
            cancelled: true,
            ..fail(Stage::Validate, "cancelled".into())
        });
    }

    // 2. prepare the folder
    say(Stage::Prepare, format!("preparing {}", v.target.display()));
    if v.creates_folder {
        // create_dir, not create_dir_all: fails if something appeared there in the meantime
        std::fs::create_dir(&v.target).map_err(|e| {
            fail(
                Stage::Prepare,
                format!("cannot create {}: {e}", v.target.display()),
            )
        })?;
    } else if std::fs::read_dir(&v.target).map_or(true, |mut d| d.next().is_some()) {
        return Err(fail(
            Stage::Prepare,
            format!("{} is no longer empty", v.target.display()),
        ));
    }
    let mut rollback = Rollback {
        target: v.target.clone(),
        created_folder: v.creates_folder,
        armed: true,
    };
    let stop = |rollback: &mut Rollback, stage, message: String, cancelled| CreateError {
        stage,
        message,
        cancelled,
        cleanup: rollback.undo(),
    };

    // 3. scaffold with the framework's own tooling
    say(
        Stage::Scaffold,
        format!("creating the {} project", v.framework.name()),
    );
    let mut out = |line: &str| {
        progress(Progress {
            stage: Stage::Scaffold,
            message: line.to_string(),
        });
    };
    if let Err(e) = creator.create(&v, &v.target, &mut out, cancel) {
        let cancelled = cancel.load(Ordering::SeqCst);
        return Err(stop(&mut rollback, Stage::Scaffold, e, cancelled));
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(stop(
            &mut rollback,
            Stage::Scaffold,
            "cancelled".into(),
            true,
        ));
    }

    // 4. TUMR metadata
    progress(Progress {
        stage: Stage::Metadata,
        message: format!("writing {}\\{}", metadata::DIR, metadata::FILE),
    });
    let meta = ProjectMetadata::for_project(&v, tumr_version);
    if let Err(e) = metadata::write(&v.target, &meta) {
        return Err(stop(&mut rollback, Stage::Metadata, e, false));
    }

    // 5. verify: TUMR's own detection must recognise it, and the metadata must read back
    let confidence = match verify(&v, &meta, progress) {
        Ok(c) => c,
        Err(msg) => return Err(stop(&mut rollback, Stage::Verify, msg, false)),
    };
    let (unexpected, unexpected_count) = unexpected(&v, creator, progress);

    rollback.armed = false;
    Ok(Created {
        path: v.target.clone(),
        framework: v.framework,
        package_id: v.package_id.clone(),
        metadata: metadata::path(&v.target),
        confidence,
        warnings: v.warnings,
        unexpected,
        unexpected_count,
    })
}

/// Lists what the creator made beyond its [`Creator::expected_files`] (and says so in the progress).
fn unexpected(
    v: &Validated,
    creator: &dyn Creator,
    progress: &mut dyn FnMut(Progress),
) -> (Vec<String>, usize) {
    let mut expected = creator.expected_files(v);
    expected.push(".tumr/".into());
    let expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    let mut unexpected = Vec::new();
    unexpected_files(&v.target, "", &expected, &mut unexpected);
    unexpected.sort();
    let unexpected_count = unexpected.len();
    if unexpected_count > 0 {
        let first: Vec<&str> = unexpected.iter().take(5).map(String::as_str).collect();
        progress(Progress {
            stage: Stage::Verify,
            message: format!(
                "{unexpected_count} file(s) the {} creator does not normally make: {}",
                v.framework.name(),
                first.join(", ")
            ),
        });
    }
    unexpected.truncate(UNEXPECTED_LISTED);
    (unexpected, unexpected_count)
}

/// Collects the paths under `dir` (whose path relative to the project is `rel`) that `expected`
/// does not cover. A folder nothing expected lies in is reported once, as `name/`. Links are
/// never followed.
fn unexpected_files(dir: &Path, rel: &str, expected: &[&str], out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        out.push(format!("{rel} (cannot be read)"));
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{rel}{name}");
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_dir {
            let folder = format!("{path}/");
            if expected.contains(&folder.as_str()) {
                continue;
            }
            if expected.iter().any(|e| e.starts_with(&folder)) {
                unexpected_files(&entry.path(), &folder, expected, out);
            } else {
                out.push(folder);
            }
        } else if !expected.contains(&path.as_str()) {
            out.push(path);
        }
    }
}

/// TUMR's own detection must recognise the project, and the metadata must read back unchanged.
fn verify(
    v: &Validated,
    meta: &ProjectMetadata,
    progress: &mut dyn FnMut(Progress),
) -> Result<u8, String> {
    progress(Progress {
        stage: Stage::Verify,
        message: "checking that TUMR recognises the project".into(),
    });
    let d = tumr_resolver::detect::detect(&v.target);
    if d.framework != v.framework.detected_as() || d.confidence < 50 {
        return Err(format!(
            "the new folder does not look like a {} project to TUMR (detected {:?}, confidence {})",
            v.framework.name(),
            d.framework,
            d.confidence
        ));
    }
    match metadata::read(&v.target)? {
        Some(m) if &m == meta => Ok(d.confidence),
        _ => Err("the project metadata did not read back".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Destination;

    /// Writes a minimal native Android layout (what detection looks for), or fails on request.
    struct Stub {
        framework: Framework,
        fail: bool,
        cancel_during: bool,
    }

    impl Creator for Stub {
        fn framework(&self) -> Framework {
            self.framework
        }
        fn create(
            &self,
            v: &Validated,
            target: &Path,
            out: &mut dyn FnMut(&str),
            cancel: &AtomicBool,
        ) -> Result<(), String> {
            out("stub: writing files");
            std::fs::write(target.join("settings.gradle"), "include ':app'\n").unwrap();
            std::fs::create_dir_all(target.join("app/src/main")).unwrap();
            std::fs::write(
                target.join("app/build.gradle"),
                format!(
                    "plugins {{ id 'com.android.application' }}\nandroid {{ namespace '{}' }}\n",
                    v.package_id
                ),
            )
            .unwrap();
            std::fs::write(
                target.join("app/src/main/AndroidManifest.xml"),
                "<manifest/>",
            )
            .unwrap();
            std::fs::write(target.join("gradlew.bat"), "@echo off\n").unwrap();
            if self.cancel_during {
                cancel.store(true, Ordering::SeqCst);
            }
            if self.fail {
                return Err("the framework tool failed (stub)".into());
            }
            Ok(())
        }
        fn expected_files(&self, _: &Validated) -> Vec<String> {
            STUB_FILES.iter().map(|s| (*s).to_string()).collect()
        }
    }

    const STUB_FILES: &[&str] = &[
        "settings.gradle",
        "gradlew.bat",
        "app/build.gradle",
        "app/src/main/AndroidManifest.xml",
    ];

    fn parent(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tumr-create-job-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // the engine records real (long) paths; TEMP may hold an 8.3 short name
        crate::request::real_path(&d)
    }

    fn req(framework: Framework, dest: Destination) -> CreateRequest {
        CreateRequest {
            framework,
            name: "Hello App".into(),
            package_id: None,
            destination: dest,
            agent: None,
            trust: false,
        }
    }

    fn stub(framework: Framework) -> Stub {
        Stub {
            framework,
            fail: false,
            cancel_during: false,
        }
    }

    #[test]
    fn creates_verifies_and_reports_every_stage() {
        let p = parent("ok");
        let mut stages = Vec::new();
        let created = run(
            &req(
                Framework::Kotlin,
                Destination::NewIn {
                    parent: p.display().to_string(),
                },
            ),
            &stub(Framework::Kotlin),
            "0.1.35",
            &mut |pr| stages.push(pr.stage),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(created.path, p.join("Hello App"));
        assert_eq!(created.package_id, "com.example.helloapp");
        assert!(created.confidence >= 50);
        assert_eq!(
            created.unexpected,
            Vec::<String>::new(),
            "only expected files"
        );
        assert_eq!(created.unexpected_count, 0);
        let m = metadata::read(&created.path).unwrap().unwrap();
        assert_eq!(
            (m.framework.as_str(), m.language.as_str()),
            ("kotlin", "kotlin")
        );
        for s in [
            Stage::Validate,
            Stage::Prepare,
            Stage::Scaffold,
            Stage::Metadata,
            Stage::Verify,
        ] {
            assert!(stages.contains(&s), "{s:?} not reported");
        }
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn a_failed_tool_removes_the_folder_it_created() {
        let p = parent("fail");
        let e = run(
            &req(
                Framework::Kotlin,
                Destination::NewIn {
                    parent: p.display().to_string(),
                },
            ),
            &Stub {
                framework: Framework::Kotlin,
                fail: true,
                cancel_during: false,
            },
            "0.1.35",
            &mut |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(e.stage, Stage::Scaffold);
        assert!(e.cleanup.contains("removed the folder"), "{}", e.cleanup);
        assert!(!p.join("Hello App").exists());
        assert!(p.exists(), "the parent is never touched");
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn an_existing_folder_is_emptied_again_but_kept() {
        let p = parent("existing");
        let e = run(
            &req(
                Framework::Kotlin,
                Destination::ExistingEmpty {
                    path: p.display().to_string(),
                },
            ),
            &Stub {
                framework: Framework::Kotlin,
                fail: true,
                cancel_during: false,
            },
            "0.1.35",
            &mut |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(e.cleanup.contains("emptied"), "{}", e.cleanup);
        assert!(p.is_dir() && std::fs::read_dir(&p).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn cancel_during_scaffold_rolls_back() {
        let p = parent("cancel");
        let e = run(
            &req(
                Framework::Kotlin,
                Destination::NewIn {
                    parent: p.display().to_string(),
                },
            ),
            &Stub {
                framework: Framework::Kotlin,
                fail: false,
                cancel_during: true,
            },
            "0.1.35",
            &mut |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(e.cancelled);
        assert!(!p.join("Hello App").exists());
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn a_project_detection_does_not_recognise_is_not_reported_as_created() {
        // the creator writes no project at all, but the request says Flutter
        struct Mislabelled;
        impl Creator for Mislabelled {
            fn framework(&self) -> Framework {
                Framework::Flutter
            }
            fn create(
                &self,
                _: &Validated,
                t: &Path,
                _: &mut dyn FnMut(&str),
                _: &AtomicBool,
            ) -> Result<(), String> {
                std::fs::write(t.join("README.md"), "not a project").map_err(|e| e.to_string())
            }
            fn expected_files(&self, _: &Validated) -> Vec<String> {
                vec!["README.md".into()]
            }
        }
        let p = parent("verify");
        let e = run(
            &req(
                Framework::Flutter,
                Destination::NewIn {
                    parent: p.display().to_string(),
                },
            ),
            &Mislabelled,
            "0.1.35",
            &mut |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(e.stage, Stage::Verify);
        assert!(!p.join("Hello App").exists());
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn a_creator_for_another_framework_is_refused() {
        let p = parent("wrong");
        let e = run(
            &req(
                Framework::Java,
                Destination::NewIn {
                    parent: p.display().to_string(),
                },
            ),
            &stub(Framework::Kotlin),
            "0.1.35",
            &mut |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(e.stage, Stage::Validate);
        assert!(!p.join("Hello App").exists(), "nothing was created");
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn files_the_creator_did_not_list_are_reported_but_do_not_fail() {
        struct Extra;
        impl Creator for Extra {
            fn framework(&self) -> Framework {
                Framework::Java
            }
            fn create(
                &self,
                v: &Validated,
                t: &Path,
                out: &mut dyn FnMut(&str),
                c: &AtomicBool,
            ) -> Result<(), String> {
                stub(Framework::Java).create(v, t, out, c)?;
                std::fs::write(t.join("app/src/main/stray.txt"), "?").unwrap();
                std::fs::create_dir_all(t.join(".claude/x")).unwrap();
                std::fs::write(t.join(".claude/x/settings.json"), "{}").unwrap();
                std::fs::write(t.join("LICENSE"), "?").unwrap();
                Ok(())
            }
            fn expected_files(&self, _: &Validated) -> Vec<String> {
                STUB_FILES.iter().map(|s| (*s).to_string()).collect()
            }
        }
        let p = parent("unexpected");
        let mut lines = Vec::new();
        let created = run(
            &req(
                Framework::Java,
                Destination::NewIn {
                    parent: p.display().to_string(),
                },
            ),
            &Extra,
            "0.1.35",
            &mut |pr| lines.push(pr.message),
            &AtomicBool::new(false),
        )
        .unwrap();
        // a folder nothing expected lies in is named once; .tumr/ is always expected
        assert_eq!(
            created.unexpected,
            [".claude/", "LICENSE", "app/src/main/stray.txt"]
        );
        assert_eq!(created.unexpected_count, 3);
        assert!(lines.iter().any(|l| l.contains("3 file(s)")), "{lines:?}");
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn a_listed_folder_covers_everything_inside_it() {
        let p = parent("walk");
        std::fs::create_dir_all(p.join("node_modules/a/b")).unwrap();
        std::fs::write(p.join("node_modules/a/b/c.js"), "").unwrap();
        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(p.join("src/App.tsx"), "").unwrap();
        std::fs::write(p.join("src/extra.ts"), "").unwrap();
        let mut out = Vec::new();
        unexpected_files(&p, "", &["node_modules/", "src/App.tsx"], &mut out);
        assert_eq!(out, ["src/extra.ts"]);
        let _ = std::fs::remove_dir_all(&p);
    }
}
