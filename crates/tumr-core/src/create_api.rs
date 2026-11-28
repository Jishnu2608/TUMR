//! TUMR Create on the Core (CRT-A): `project.create.frameworks`, `project.create.start`,
//! `project.create.status`, `project.create.cancel`. The CLI wizard (`tumr create`) and the
//! desktop Create Project wizard both call these, so both get the same engine
//! ([`tumr_create::run`]) and the same results.
//!
//! One creation at a time runs on its own thread, like an install job. Around the engine's own
//! stages the Core adds two: `check-tools`, before the thread starts (the framework's creation
//! tool must be installed: the plan for a new project of that framework is built, and anything
//! it would still install is named in the error; nothing is written), and `register` at the end
//! (recent projects; trust only when the request asks and the creator made nothing unexpected:
//! otherwise the front end asks the user and calls `project.trust`, owner decision CRT-D1).
//! Every log line of a creation carries its
//! id (the correlation id the front ends show).
//!
//! The framework creators are in [`tumr_create::creators`] (CRT-B); the Core gives them the tool
//! locations from the plan and TUMR's child environment (isolation, no telemetry).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use serde_json::json;
use tumr_api::methods::{CancelResult, JobParams, ProjectPlanParams};
use tumr_api::rpc::{codes, Id, Response, RpcError};
use tumr_create::creators::Toolset;
use tumr_create::{CreateError, CreateRequest, Created, Creator, Framework, Progress, Stage};

use crate::api_server::{build_plan, ok, ApiState};

/// Progress lines kept per creation (tool output can be long).
const KEEP_PROGRESS: usize = 400;

/// State of a creation, as `project.create.status` returns it.
#[derive(Debug, Clone, Serialize)]
pub struct CreateStatus {
    /// The correlation id (also in every Core log line of this creation).
    pub id: String,
    /// `running`, `done`, `failed` or `cancelled`.
    pub state: String,
    pub framework: Framework,
    pub name: String,
    pub progress: Vec<Progress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Created>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CreateError>,
    /// The project was trusted by this creation (only for a request with `trust`).
    pub trusted: bool,
    /// The request asked for trust, but the project holds files its creator does not normally
    /// make (`result.unexpected`): the front end shows them and asks before calling
    /// `project.trust`.
    pub trust_needs_confirmation: bool,
    pub started_unix: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_unix: Option<u64>,
}

/// A creation in progress or finished.
#[derive(Debug)]
pub struct CreateJob {
    status: Mutex<CreateStatus>,
    cancel: AtomicBool,
}

impl CreateJob {
    #[must_use]
    pub fn status(&self) -> CreateStatus {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        self.status().state == "running"
    }

    fn update(&self, f: impl FnOnce(&mut CreateStatus)) {
        f(&mut self.status.lock().unwrap_or_else(PoisonError::into_inner));
    }

    fn push(&self, p: Progress) {
        self.update(|s| {
            s.progress.push(p);
            let over = s.progress.len().saturating_sub(KEEP_PROGRESS);
            s.progress.drain(..over);
        });
    }
}

fn err(id: Id, code: i64, msg: impl Into<String>) -> Response {
    Response::err(Some(id), RpcError::new(code, msg))
}

/// `project.create.frameworks`: the five choices for the wizards.
#[must_use]
pub fn frameworks(id: Id) -> Response {
    let list: Vec<_> = Framework::ALL
        .iter()
        .map(|f| {
            json!({
                "id": f.id(),
                "name": f.name(),
                "description": f.description(),
                "language": f.language(),
                "creation_tools": f.creation_tools(),
                "available": true,
            })
        })
        .collect();
    ok(id, &json!({ "frameworks": list }))
}

/// `project.create.check`: the request's validation only, nothing written (the desktop wizard
/// checks the name, package id and folder as the user types).
#[must_use]
pub fn check(id: Id, req: &CreateRequest) -> Response {
    match tumr_create::request::validate(req) {
        Ok(v) => ok(
            id,
            &json!({
                "target": v.target.display().to_string(),
                "creates_folder": v.creates_folder,
                "package_id": v.package_id,
                "warnings": v.warnings,
            }),
        ),
        Err(e) => err(id, codes::INVALID_PARAMS, e),
    }
}

/// `project.create.start`: validates, then runs the creation on its own thread with the
/// framework's creator (its official tool; Java: TUMR's template).
pub fn start(state: &ApiState, id: Id, client: Option<&str>, req: &CreateRequest) -> Response {
    let framework = req.framework;
    start_with(state, id, client, req, move |tools| {
        Arc::from(tumr_create::creators::creator_for(framework, tools))
    })
}

/// [`start`] with a given creator, made from the tools the check found (tests use a stub).
pub fn start_with(
    state: &ApiState,
    id: Id,
    client: Option<&str>,
    req: &CreateRequest,
    make_creator: impl FnOnce(Toolset) -> Arc<dyn Creator>,
) -> Response {
    // the inputs are checked before anything starts (and again by the engine)
    if let Err(e) = tumr_create::request::validate(req) {
        return err(id, codes::INVALID_PARAMS, e);
    }
    // check-tools: the framework's creation tool must be there before anything is written
    let tools = match check_tools(state, req) {
        Ok(t) => t,
        Err(message) => {
            tracing::warn!(
                framework = req.framework.id(),
                "project.create refused: {message}"
            );
            return err(id, codes::UNAVAILABLE, message);
        }
    };
    let creator = make_creator(tools);
    let mut slot = state
        .create_job
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if slot.as_ref().is_some_and(|j| j.is_running()) {
        return err(
            id,
            codes::BUSY,
            "another project is being created; see project.create.status",
        );
    }
    let job_id = format!("create-{}", tumr_installer::random_id());
    let job = Arc::new(CreateJob {
        status: Mutex::new(CreateStatus {
            id: job_id.clone(),
            state: "running".into(),
            framework: req.framework,
            name: req.name.trim().to_string(),
            progress: Vec::new(),
            result: None,
            error: None,
            trusted: false,
            trust_needs_confirmation: false,
            started_unix: tumr_installer::now_unix().try_into().unwrap_or(0),
            finished_unix: None,
        }),
        cancel: AtomicBool::new(false),
    });
    *slot = Some(job.clone());
    drop(slot);
    tracing::info!(job = %job_id, framework = req.framework.id(), client = ?client, "project.create started");
    job.push(Progress {
        stage: Stage::CheckTools,
        message: format!("the tools {} needs are installed", req.framework.name()),
    });
    let ctx = JobContext {
        data_root: state.data_root.clone(),
        db: state.db.clone(),
        client: client.map(String::from),
    };
    let r = req.clone();
    let spawned = std::thread::Builder::new()
        .name(job_id.clone())
        .spawn(move || run_job(&ctx, &job, &r, creator.as_ref()));
    match spawned {
        Ok(_) => ok(id, &json!({ "job_id": job_id })),
        Err(e) => err(
            id,
            codes::INTERNAL_ERROR,
            format!("cannot start the creation: {e}"),
        ),
    }
}

/// What the creation thread needs from the Core.
struct JobContext {
    data_root: PathBuf,
    db: Arc<Mutex<tumr_storage::Db>>,
    client: Option<String>,
}

fn run_job(ctx: &JobContext, job: &CreateJob, req: &CreateRequest, creator: &dyn Creator) {
    let client = ctx.client.as_deref();
    let job_id = job.status().id;
    let finish = |state_name: &str, result: Option<Created>, error: Option<CreateError>| {
        job.update(|s| {
            s.state = state_name.into();
            s.result = result;
            s.error = error;
            s.finished_unix = tumr_installer::now_unix().try_into().ok();
        });
    };

    let r = tumr_create::run(
        req,
        creator,
        env!("CARGO_PKG_VERSION"),
        &mut |p| {
            tracing::debug!(job = %job_id, stage = ?p.stage, "{}", p.message);
            job.push(p);
        },
        &job.cancel,
    );
    let created = match r {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(job = %job_id, stage = ?e.stage, cancelled = e.cancelled, "project.create failed: {} ({})", e.message, e.cleanup);
            let name = if e.cancelled { "cancelled" } else { "failed" };
            finish(name, None, Some(e));
            return;
        }
    };

    // register: recent projects, trust when asked
    job.push(Progress {
        stage: Stage::Register,
        message: "adding the project to TUMR".into(),
    });
    let path = created.path.display().to_string();
    crate::home_api::record_run(&ctx.data_root, &path, created.framework.adapter());
    if req.trust && created.unexpected_count > 0 {
        tracing::info!(job = %job_id, unexpected = created.unexpected_count, "project.create: trust waits for the user");
        job.push(Progress {
            stage: Stage::Register,
            message: format!(
                "not trusted yet: the project holds {} file(s) TUMR did not expect; check them, then confirm trust",
                created.unexpected_count
            ),
        });
        job.update(|s| s.trust_needs_confirmation = true);
    } else if req.trust {
        let db = ctx.db.lock().unwrap_or_else(PoisonError::into_inner);
        let now = tumr_installer::now_unix();
        let trusted = db.trust_project(&path, client, now);
        let _ = db.audit(
            now,
            client,
            "project.trust",
            Some(&format!("{path} (created by TUMR Create)")),
        );
        drop(db);
        if trusted.is_ok() {
            job.update(|s| s.trusted = true);
        }
        if let Err(e) = trusted {
            tracing::warn!(job = %job_id, "project created but not trusted: {e}");
            job.push(Progress {
                stage: Stage::Register,
                message: format!("the project was created but could not be trusted ({e}); trust it with `tumr trust`"),
            });
        }
    }
    tracing::info!(job = %job_id, path = %path, framework = created.framework.id(), "project.create done");
    finish("done", Some(created), None);
}

/// The framework's creation tools must be installed (the plan for a new project of that
/// framework would reuse them); names what it would still install. Returns where they are, with
/// TUMR's child environment.
fn check_tools(state: &ApiState, req: &CreateRequest) -> Result<Toolset, String> {
    let needed = req.framework.creation_tools();
    let mut reuse: HashMap<String, PathBuf> = HashMap::new();
    if !needed.is_empty() {
        let target = tumr_create::request::validate(req)?.target;
        let (_, plan) = build_plan(
            state,
            &ProjectPlanParams {
                path: target.display().to_string(),
                rescan: false,
                framework: Some(req.framework.id().into()),
            },
        )
        .map_err(|e| format!("cannot check the tools: {}", e.message))?;
        let missing: Vec<String> = plan
            .items
            .iter()
            .filter(|i| needed.contains(&i.component.as_str()))
            .filter(|i| matches!(i.action, tumr_resolver::plan::Action::Install))
            .map(|i| i.title.clone())
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "{} needs {} first; install it from the plan for a new {} project (nothing was created)",
                req.framework.name(),
                missing.join(", "),
                req.framework.name()
            ));
        }
        for item in plan.items {
            if let tumr_resolver::plan::Action::Reuse { path, .. } = item.action {
                reuse.insert(item.component, PathBuf::from(path));
            }
        }
    }
    toolset(state, &reuse)
}

/// The creators' tools and environment from the plan's reused components.
fn toolset(state: &ApiState, reuse: &HashMap<String, PathBuf>) -> Result<Toolset, String> {
    // a plan path may name the program itself (node.exe, git.exe) or its folder
    let folder = |c: &str| {
        reuse.get(c).map(|p| {
            if p.is_file() {
                p.parent().map_or_else(|| p.clone(), Path::to_path_buf)
            } else {
                p.clone()
            }
        })
    };
    let sdk = state.toolchains.join("android-sdk");
    let sel = tumr_env::recipe::Selection {
        jdk_home: reuse.get("jdk").cloned(),
        android_sdk: Some(sdk.clone()),
        node_dir: folder("node"),
        flutter_root: reuse.get("flutter").cloned(),
        extra_path: folder("git").into_iter().collect(),
        isolate_appdata: true,
        adb_port: Some(state.adb_port),
    };
    let dirs = tumr_env::recipe::IsolationDirs::under(&state.toolchains);
    dirs.create()
        .map_err(|e| format!("cannot prepare TUMR's tool folders: {e}"))?;
    let ctx = tumr_env::ScanContext::from_system(state.toolchains.clone());
    Ok(Toolset {
        env: tumr_env::recipe::child_env(&ctx, &sel),
        node_dir: sel.node_dir,
        flutter_root: sel.flutter_root,
        android_sdk: Some(sdk),
        staging_root: dirs.tmp,
    })
}

fn job_matches(j: &CreateJob, p: &JobParams) -> bool {
    p.job_id.as_ref().is_none_or(|want| *want == j.status().id)
}

/// `project.create.status`.
pub fn status(state: &ApiState, id: Id, p: &JobParams) -> Response {
    let slot = state
        .create_job
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    match slot.as_ref() {
        Some(j) if job_matches(j, p) => ok(id, &j.status()),
        _ => err(id, codes::INVALID_PARAMS, "no such project creation"),
    }
}

/// `project.create.cancel`: the engine stops at its next check and rolls back.
pub fn cancel(state: &ApiState, id: Id, p: &JobParams) -> Response {
    let slot = state
        .create_job
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    match slot.as_ref() {
        Some(j) if job_matches(j, p) => {
            let running = j.is_running();
            j.cancel.store(true, Ordering::SeqCst);
            ok(
                id,
                &CancelResult {
                    cancelling: running,
                },
            )
        }
        _ => err(id, codes::INVALID_PARAMS, "no such project creation"),
    }
}

/// For tests: whether a path holds TUMR's metadata.
#[must_use]
pub fn has_metadata(project: &Path) -> bool {
    tumr_create::metadata::path(project).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStore;
    use crate::test_support::TempRoot;
    use std::time::{Duration, Instant};
    use tumr_create::{Destination, Validated};

    /// Writes the files TUMR's detection looks for in a native Android project.
    struct Stub;

    impl Creator for Stub {
        fn framework(&self) -> Framework {
            Framework::Java
        }
        fn create(
            &self,
            _: &Validated,
            t: &Path,
            out: &mut dyn FnMut(&str),
            _: &AtomicBool,
        ) -> Result<(), String> {
            out("stub creator");
            let w = |rel: &str, text: &str| {
                let p = t.join(rel);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(p, text).unwrap();
            };
            w("settings.gradle", "include ':app'\n");
            w(
                "app/build.gradle",
                "plugins { id 'com.android.application' }\n",
            );
            w("app/src/main/AndroidManifest.xml", "<manifest/>\n");
            w("gradlew.bat", "@echo off\n");
            Ok(())
        }
        fn expected_files(&self, _: &Validated) -> Vec<String> {
            [
                "settings.gradle",
                "app/build.gradle",
                "app/src/main/AndroidManifest.xml",
                "gradlew.bat",
            ]
            .iter()
            .map(|s| (*s).to_string())
            .collect()
        }
    }

    /// Like [`Stub`], plus one file it does not list.
    struct Stray;

    impl Creator for Stray {
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
            Stub.create(v, t, out, c)?;
            std::fs::write(t.join("surprise.cmd"), "@echo off\n").map_err(|e| e.to_string())
        }
        fn expected_files(&self, v: &Validated) -> Vec<String> {
            Stub.expected_files(v)
        }
    }

    fn state(tmp: &TempRoot) -> ApiState {
        let auth = AuthStore::load(&tmp.path().join("clients.json"), "tumr_admin").unwrap();
        let db = tumr_storage::Db::open_in(tmp.path()).unwrap();
        ApiState::new(tmp.path().to_path_buf(), auth, db)
    }

    fn request(parent: &Path, trust: bool) -> CreateRequest {
        CreateRequest {
            framework: Framework::Java,
            name: "Hello Java".into(),
            package_id: Some("com.example.hellojava".into()),
            destination: Destination::NewIn {
                parent: parent.display().to_string(),
            },
            agent: Some("claude-code".into()),
            trust,
        }
    }

    fn wait_done(state: &ApiState) -> CreateStatus {
        let start = Instant::now();
        loop {
            let st = state.create_job.lock().unwrap().as_ref().unwrap().status();
            if st.state != "running" || start.elapsed() > Duration::from_secs(20) {
                return st;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn check_answers_without_writing() {
        let tmp = TempRoot::new("create-check");
        let parent = tmp.path().join("apps");
        std::fs::create_dir_all(&parent).unwrap();
        let mut req = request(&parent, true);
        let r = check(Id::Num(1), &req);
        let v = r.result.expect("valid request");
        assert!(v["target"].as_str().unwrap().ends_with("Hello Java"));
        assert_eq!(v["creates_folder"], true);
        assert!(!parent.join("Hello Java").exists(), "check writes nothing");
        req.name = "1 bad".into();
        assert!(check(Id::Num(2), &req).error.is_some());
    }

    #[test]
    fn the_five_frameworks_are_listed() {
        let r = frameworks(Id::Num(1));
        let list = r.result.unwrap()["frameworks"].as_array().unwrap().clone();
        let ids: Vec<&str> = list.iter().map(|f| f["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["expo", "react-native", "flutter", "kotlin", "java"]);
        assert!(list.iter().all(|f| f["available"] == true));
    }

    #[test]
    fn the_real_java_creator_runs_end_to_end() {
        // Java needs no tool to create (TUMR's template), so this runs everywhere
        let tmp = TempRoot::new("create-java");
        let st = state(&tmp);
        let r = start(&st, Id::Num(1), None, &request(tmp.path(), true));
        assert!(r.error.is_none(), "{:?}", r.error);
        let done = wait_done(&st);
        assert_eq!(done.state, "done", "{:?}", done.error);
        let result = done.result.unwrap();
        assert_eq!(result.unexpected, Vec::<String>::new());
        assert!(done.trusted);
        let main = result
            .path
            .join("app/src/main/java/com/example/hellojava/MainActivity.java");
        assert!(main.is_file());
        assert!(result
            .path
            .join("gradle/wrapper/gradle-wrapper.jar")
            .is_file());
    }

    #[test]
    fn a_missing_creation_tool_is_named_and_nothing_is_created() {
        let tmp = TempRoot::new("create-notools");
        let st = state(&tmp);
        let mut req = request(tmp.path(), false);
        req.framework = Framework::Flutter;
        let r = start(&st, Id::Num(1), None, &req);
        let e = r.error.unwrap();
        assert_eq!(e.code, codes::UNAVAILABLE, "{}", e.message);
        assert!(e.message.contains("nothing was created"), "{}", e.message);
        assert!(!tmp.path().join("Hello Java").exists());
    }

    #[test]
    fn a_creation_runs_registers_and_trusts_when_asked() {
        let tmp = TempRoot::new("create-run");
        let parent = tmp.path().join("projects");
        std::fs::create_dir_all(&parent).unwrap();
        let st = state(&tmp);
        let r = start_with(
            &st,
            Id::Num(1),
            Some("desktop"),
            &request(&parent, true),
            |_| Arc::new(Stub),
        );
        let job_id = r.result.unwrap()["job_id"].as_str().unwrap().to_string();
        assert!(job_id.starts_with("create-"));
        let done = wait_done(&st);
        assert_eq!(done.state, "done", "{:?}", done.error);
        // the engine records the real (long) path; TEMP may be an 8.3 short one
        let real = std::fs::canonicalize(&parent)
            .unwrap()
            .display()
            .to_string();
        let project = PathBuf::from(real.trim_start_matches(r"\\?\")).join("Hello Java");
        let done_trusted = done.trusted && !done.trust_needs_confirmation;
        assert_eq!(done.result.unwrap().path, project);
        assert!(has_metadata(&project));
        let stages: Vec<Stage> = done.progress.iter().map(|p| p.stage).collect();
        for s in [
            Stage::CheckTools,
            Stage::Validate,
            Stage::Scaffold,
            Stage::Verify,
            Stage::Register,
        ] {
            assert!(stages.contains(&s), "{s:?} missing in {stages:?}");
        }
        // registered in recent projects, and trusted because the request asked
        let recent = std::fs::read_to_string(tmp.path().join("recent-projects.json")).unwrap();
        assert!(recent.contains("Hello Java") && recent.contains("gradle-android"));
        let trusted = st
            .db
            .lock()
            .unwrap()
            .is_trusted(&project.display().to_string())
            .unwrap();
        assert!(trusted && done_trusted);
        // status and cancel find the job by its id
        let s = status(
            &st,
            Id::Num(2),
            &JobParams {
                job_id: Some(job_id.clone()),
            },
        );
        assert_eq!(s.result.unwrap()["state"], "done");
        let c = cancel(
            &st,
            Id::Num(3),
            &JobParams {
                job_id: Some(job_id),
            },
        );
        assert_eq!(
            c.result.unwrap()["cancelling"],
            false,
            "a finished job is not cancelled"
        );
    }

    #[test]
    fn untrusted_unless_asked_and_bad_inputs_are_refused_before_any_job() {
        let tmp = TempRoot::new("create-untrusted");
        let st = state(&tmp);
        start_with(&st, Id::Num(1), None, &request(tmp.path(), false), |_| {
            Arc::new(Stub)
        });
        assert_eq!(wait_done(&st).state, "done");
        let project = tmp.path().join("Hello Java").display().to_string();
        assert!(!st.db.lock().unwrap().is_trusted(&project).unwrap());

        let mut bad = request(tmp.path(), false);
        bad.name = "../escape".into();
        let r = start_with(&st, Id::Num(2), None, &bad, |_| Arc::new(Stub));
        assert_eq!(r.error.unwrap().code, codes::INVALID_PARAMS);
        // the same name again: the folder exists now, never overwritten
        let r = start_with(&st, Id::Num(3), None, &request(tmp.path(), false), |_| {
            Arc::new(Stub)
        });
        assert!(r.error.unwrap().message.contains("already exists"));
    }

    #[test]
    fn status_of_an_unknown_job_is_an_error() {
        let tmp = TempRoot::new("create-unknown");
        let st = state(&tmp);
        let r = status(&st, Id::Num(1), &JobParams { job_id: None });
        assert_eq!(r.error.unwrap().code, codes::INVALID_PARAMS);
    }

    #[test]
    fn unexpected_files_hold_trust_for_the_user() {
        let tmp = TempRoot::new("create-stray");
        let st = state(&tmp);
        start_with(&st, Id::Num(1), None, &request(tmp.path(), true), |_| {
            Arc::new(Stray)
        });
        let done = wait_done(&st);
        assert_eq!(done.state, "done", "{:?}", done.error);
        assert!(done.trust_needs_confirmation && !done.trusted);
        let result = done.result.unwrap();
        assert_eq!(result.unexpected, ["surprise.cmd"]);
        let project = result.path.display().to_string();
        assert!(!st.db.lock().unwrap().is_trusted(&project).unwrap());
    }

    /// Live (opt-in): every framework created with the real tools on this PC.
    /// `TUMR_LIVE_TOOLCHAINS` is TUMR's managed root, `TUMR_LIVE_OUT` an empty parent folder for
    /// the projects, `TUMR_LIVE_ONLY` an optional comma list of framework ids. Run with
    /// `--ignored --nocapture`.
    #[test]
    #[ignore = "live: runs the official creation tools (network, minutes)"]
    fn live_create_all_five() {
        let (Ok(toolchains), Ok(out)) = (
            std::env::var("TUMR_LIVE_TOOLCHAINS"),
            std::env::var("TUMR_LIVE_OUT"),
        ) else {
            panic!("set TUMR_LIVE_TOOLCHAINS and TUMR_LIVE_OUT");
        };
        let tmp = TempRoot::new("create-live");
        let auth = AuthStore::load(&tmp.path().join("clients.json"), "tumr_admin").unwrap();
        let db = tumr_storage::Db::open_in(tmp.path()).unwrap();
        let st = ApiState::with_toolchains(tmp.path().to_path_buf(), toolchains.into(), auth, db);
        let mut failed = Vec::new();
        for (framework, name) in [
            (Framework::Java, "Live Java"),
            (Framework::Kotlin, "Live Kotlin"),
            (Framework::Flutter, "Live Flutter"),
            (Framework::Expo, "Live Expo"),
            (Framework::ReactNative, "Live RN"),
        ] {
            let only = std::env::var("TUMR_LIVE_ONLY").unwrap_or_default();
            if !only.is_empty() && !only.split(',').any(|f| f == framework.id()) {
                continue;
            }
            let req = CreateRequest {
                framework,
                name: name.into(),
                package_id: None,
                destination: Destination::NewIn {
                    parent: out.clone(),
                },
                agent: None,
                trust: false,
            };
            let started = Instant::now();
            let r = start(&st, Id::Num(1), None, &req);
            if let Some(e) = r.error {
                println!("{name}: refused: {}", e.message);
                failed.push(name);
                continue;
            }
            let done = loop {
                let s = st.create_job.lock().unwrap().as_ref().unwrap().status();
                if s.state != "running" {
                    break s;
                }
                std::thread::sleep(Duration::from_millis(250));
            };
            println!("== {name}: {} in {:.0?}", done.state, started.elapsed());
            for p in done.progress.iter().filter(|p| p.stage != Stage::Scaffold) {
                println!("   {:?}: {}", p.stage, p.message);
            }
            match (&done.result, &done.error) {
                (Some(c), _) => println!(
                    "   path {} confidence {} unexpected {:?}",
                    c.path.display(),
                    c.confidence,
                    c.unexpected
                ),
                (_, Some(e)) => {
                    println!("   ERROR {:?}: {} ({})", e.stage, e.message, e.cleanup);
                    failed.push(name);
                }
                _ => {}
            }
        }
        assert!(failed.is_empty(), "failed: {failed:?}");
    }
}
