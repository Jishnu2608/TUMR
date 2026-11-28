//! CRT-F: TUMR Create parity. The same project is created once through the CLI wizard
//! (`tumr create … --json`) and once the way the desktop Create Project wizard does it
//! (`apps/desktop/ui/src/Create.tsx`): the Core's `project.create.check` / `start` / `status`
//! over the API pipe, then the `tumr agent connect|open … --json` command lines the desktop's
//! `agent_cli` bridge builds (`apps/desktop/src/agents.rs`, pinned by its own unit test). The two
//! projects, their metadata, the agent's MCP configuration and the launch must match.
//!
//! Isolation: each path gets its own fake user profile (`USERPROFILE`, `APPDATA`, `LOCALAPPDATA`,
//! `CODEX_HOME`), so nothing of the real agents' configuration is read or written, and a temporary
//! `TUMR_HOME` with its own Runtime Core.
//!
//! **Mocked:** the agent launch. `code` (GitHub Copilot's VS Code) is a fake `code.cmd` that only
//! records the arguments it was given; no editor starts. The launch test therefore checks that
//! both front ends hand the agent the same folder and report the same steps, not that VS Code
//! opens (that was checked live on a real PC in CRT-D).
//!
//! The default run creates Java projects (TUMR's own template: no tools needed). Set
//! `TUMR_PARITY_ALL=1` and `TUMR_PARITY_TOOLCHAINS=<managed toolchains folder>` to run the same
//! comparison for all five frameworks with their real creation tools.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{json, Value};
use tumr_core::launcher::CoreTarget;
use tumr_core::paths::CorePaths;

const TUMR: &str = env!("CARGO_BIN_EXE_tumr");

fn core_bin() -> PathBuf {
    let p = PathBuf::from(TUMR).with_file_name("tumr-core.exe");
    assert!(p.is_file(), "build tumr-core first: {}", p.display());
    p
}

/// A temporary Runtime Core home (stopped and removed on drop).
struct Home {
    root: PathBuf,
}

impl Home {
    fn new(tag: &str, toolchains: Option<&str>) -> Self {
        let root = std::env::temp_dir().join(format!("tumr-parity-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        if let Some(t) = toolchains {
            std::fs::write(
                root.join("config.toml"),
                format!("[storage]\ntoolchains_dir = '{t}'\n"),
            )
            .unwrap();
        }
        Self { root }
    }

    /// `tumr` with this home and the given fake user profile.
    fn tumr(&self, profile: &Profile, args: &[&str]) -> (i32, Value, String) {
        let mut cmd = Command::new(TUMR);
        cmd.args(args)
            .env("TUMR_HOME", &self.root)
            .env("TUMR_CORE_BIN", core_bin())
            .stdin(Stdio::null());
        profile.apply(&mut cmd);
        let o = cmd.output().unwrap();
        let out = String::from_utf8_lossy(&o.stdout).into_owned();
        let v = serde_json::from_str(out.trim()).unwrap_or(Value::Null);
        (
            o.status.code().unwrap_or(-1),
            v,
            String::from_utf8_lossy(&o.stderr).into_owned() + &out,
        )
    }

    fn target(&self) -> CoreTarget {
        let paths = CorePaths::with_root(&self.root);
        let pipe = paths.pipe_name().unwrap();
        CoreTarget { paths, pipe }
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = Command::new(TUMR)
            .arg("stop")
            .env("TUMR_HOME", &self.root)
            .stdin(Stdio::null())
            .output();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A fake user profile and a `PATH` holding the fake `code.cmd` (mocked agent launch).
struct Profile {
    dir: PathBuf,
}

impl Profile {
    fn new(root: &Path, tag: &str) -> Self {
        let dir = root.join(format!("profile-{tag}"));
        for d in ["home", "roaming", "local", "codex", "bin"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        // MOCKED agent: records its arguments, starts nothing
        std::fs::write(
            dir.join("bin").join("code.cmd"),
            "@echo off\r\n>>\"%~dp0calls.txt\" echo %*\r\nexit /b 0\r\n",
        )
        .unwrap();
        Self { dir }
    }

    fn apply(&self, cmd: &mut Command) {
        let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        cmd.env("USERPROFILE", self.dir.join("home"))
            .env("HOME", self.dir.join("home"))
            .env("APPDATA", self.dir.join("roaming"))
            .env("LOCALAPPDATA", self.dir.join("local"))
            .env("CODEX_HOME", self.dir.join("codex"))
            .env(
                "PATH",
                format!(
                    "{};{system}\\System32;{system}\\System32\\WindowsPowerShell\\v1.0;{}",
                    self.dir.join("bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            );
    }

    /// What the mocked `code.cmd` was called with.
    fn launches(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("bin").join("calls.txt"))
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }

    /// Every file under the fake profile except the mock itself: relative path → contents.
    fn config_files(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        walk(&self.dir, &self.dir, &mut out);
        out.retain(|k, _| !k.starts_with("bin/"));
        out
    }
}

/// Relative path (`/`) → contents (text) of every file under `dir`.
fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(base, &p, out);
        } else {
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&p).unwrap_or_default();
            out.insert(rel, String::from_utf8_lossy(&bytes).into_owned());
        }
    }
}

/// The project's files, with the creation time taken out of the metadata (it differs by
/// design) and the profile folder replaced (instruction files may name it).
fn project_files(dir: &Path, profile: &Profile) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    walk(dir, dir, &mut files);
    if let Some(m) = files.get_mut(".tumr/project.json") {
        let mut v: Value = serde_json::from_str(m).unwrap();
        v.as_object_mut().unwrap().remove("createdAt");
        *m = v.to_string();
    }
    let p = profile.dir.display().to_string();
    for v in files.values_mut() {
        *v = v.replace(&p, "<profile>");
    }
    files
}

/// An existing folder's real (long) path, without the `\\?\` prefix.
fn real(p: &Path) -> PathBuf {
    let s = std::fs::canonicalize(p).unwrap().display().to_string();
    PathBuf::from(s.trim_start_matches(r"\\?\"))
}

/// The desktop wizard's calls to the Core (`Create.tsx`: `CheckStep` and `CreateStep`).
async fn desktop_create(target: &CoreTarget, req: &Value) -> Value {
    let mut c = target.connect().await.unwrap().expect("Core running");
    let check = c
        .call_raw("project.create.check", req.clone())
        .await
        .expect("project.create.check");
    let fw = req["framework"].as_str().unwrap();
    c.call_raw(
        "project.plan",
        json!({ "path": check["target"], "framework": fw }),
    )
    .await
    .expect("project.plan");
    let started = c
        .call_raw("project.create.start", req.clone())
        .await
        .expect("project.create.start");
    loop {
        let st = c
            .call_raw(
                "project.create.status",
                json!({ "job_id": started["job_id"] }),
            )
            .await
            .unwrap();
        if st["state"] != "running" {
            return st;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// One framework through both front ends; returns a description of every difference.
#[allow(clippy::too_many_lines)] // one linear scenario, both paths side by side
fn compare(framework: &str, toolchains: Option<&str>) -> Vec<String> {
    let home = Home::new(framework, toolchains);
    let cli_profile = Profile::new(&home.root, "cli");
    let desk_profile = Profile::new(&home.root, "desktop");
    let cli_parent = home.root.join("cli apps");
    let desk_parent = home.root.join("desktop apps");
    std::fs::create_dir_all(&cli_parent).unwrap();
    std::fs::create_dir_all(&desk_parent).unwrap();
    let name = "Parity App";
    let mut diffs = Vec::new();

    // ---- Desktop: the wizard's request, then the bridge's command lines ----------------------
    // the same object Create.tsx sends (package id left to the engine's suggestion)
    let req = json!({
        "framework": framework,
        "name": name,
        "destination": { "mode": "new-in", "parent": desk_parent.display().to_string() },
        "agent": "copilot",
        "trust": true,
    });
    let (started, _, l0) = home.tumr(&desk_profile, &["start"]);
    assert_eq!(started, 0, "tumr start: {l0}");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let desk = rt.block_on(desktop_create(&home.target(), &req));
    assert_eq!(desk["state"], "done", "{framework}: desktop path: {desk}");
    let desk_path = PathBuf::from(desk["result"]["path"].as_str().unwrap());
    let desk_dir = desk_path.display().to_string();
    // agents.rs `connect_apply` and `open` (client id `vscode` for Copilot)
    let (c1, connect, l1) = home.tumr(
        &desk_profile,
        &[
            "agent",
            "connect",
            "vscode",
            "--yes",
            "--project",
            &desk_dir,
            "--json",
        ],
    );
    assert_eq!(c1, 0, "desktop connect: {l1}");
    assert_eq!(connect["applied"], true, "{l1}");
    // the desktop's open runs while the CLI path creates (each launch waits for the agent)
    let open_thread = {
        let root = home.root.clone();
        let dir = desk_dir.clone();
        let profile_dir = desk_profile.dir.clone();
        std::thread::spawn(move || {
            let mut cmd = Command::new(TUMR);
            cmd.args(["agent", "open", "vscode", &dir, "--json"])
                .env("TUMR_HOME", &root)
                .env("TUMR_CORE_BIN", core_bin())
                .stdin(Stdio::null());
            Profile { dir: profile_dir }.apply(&mut cmd);
            let o = cmd.output().unwrap();
            let out = String::from_utf8_lossy(&o.stdout).into_owned();
            (
                serde_json::from_str::<Value>(out.trim()).unwrap_or(Value::Null),
                out,
            )
        })
    };

    // ---- CLI: `tumr create` with flags, agent connected and opened --------------------------
    let (code, cli, log) = home.tumr(
        &cli_profile,
        &[
            "create",
            "--framework",
            framework,
            "--name",
            name,
            "--parent",
            cli_parent.to_str().unwrap(),
            "--agent",
            "copilot",
            "--open",
            "--yes",
            "--json",
        ],
    );
    assert_eq!(code, 0, "{framework}: tumr create failed: {log}");
    assert_eq!(cli["created"], true, "{log}");

    let (open, l2) = open_thread.join().unwrap();
    assert!(open["steps"].is_array(), "desktop open: {l2}");

    // ---- compare -------------------------------------------------------------------------------
    let cli_status = &cli["status"];
    let cli_path = PathBuf::from(cli_status["result"]["path"].as_str().unwrap());
    // both record the real (long) path, whatever form the parent was given in
    assert_eq!(cli_path, real(&cli_parent).join(name));
    assert_eq!(desk_path, real(&desk_parent).join(name));
    for key in ["framework", "package_id", "confidence", "unexpected_count"] {
        if cli_status["result"][key] != desk["result"][key] {
            diffs.push(format!(
                "result.{key}: cli {} / desktop {}",
                cli_status["result"][key], desk["result"][key]
            ));
        }
    }
    if cli_status["trusted"] != desk["trusted"] {
        diffs.push(format!(
            "trusted: cli {} / desktop {}",
            cli_status["trusted"], desk["trusted"]
        ));
    }
    let a = project_files(&cli_path, &cli_profile);
    let b = project_files(&desk_path, &desk_profile);
    for k in a
        .keys()
        .chain(b.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        match (a.get(k), b.get(k)) {
            (Some(x), Some(y)) if x == y => {}
            (Some(_), Some(_)) => diffs.push(format!("project file differs: {k}")),
            (Some(_), None) => diffs.push(format!("only from the CLI: {k}")),
            (None, _) => diffs.push(format!("only from the desktop: {k}")),
        }
    }
    let meta: Value = serde_json::from_str(&a[".tumr/project.json"]).unwrap();
    if meta["agent"] != "copilot" {
        diffs.push(format!("metadata agent: {}", meta["agent"]));
    }
    // MCP configuration and instructions outside the project (the fake profiles)
    let ca = cli_profile.config_files();
    let cb = desk_profile.config_files();
    if ca != cb {
        diffs.push(format!(
            "agent configuration differs: cli {:?} / desktop {:?}",
            ca.keys().collect::<Vec<_>>(),
            cb.keys().collect::<Vec<_>>()
        ));
    }
    if !ca.keys().any(|k| k.ends_with("Code/User/mcp.json")) {
        diffs.push(format!(
            "no VS Code MCP config written: {:?}",
            ca.keys().collect::<Vec<_>>()
        ));
    }
    // MOCKED launch: the same folder handed to the agent, the same steps reported
    let la = cli_profile.launches();
    let lb = desk_profile.launches();
    if la.len() != 1 || lb.len() != 1 {
        diffs.push(format!(
            "mocked launcher calls: cli {la:?} / desktop {lb:?}"
        ));
    } else {
        if !la[0].contains(&cli_path.display().to_string()) {
            diffs.push(format!("cli launch got {la:?}"));
        }
        if !lb[0].contains(&desk_dir) {
            diffs.push(format!("desktop launch got {lb:?}"));
        }
    }
    let steps = |v: &Value| -> Vec<(String, String)> {
        v.as_array()
            .into_iter()
            .flatten()
            .map(|s| {
                (
                    s["step"].as_str().unwrap_or("").to_string(),
                    s["status"].as_str().unwrap_or("").to_string(),
                )
            })
            .collect()
    };
    let sa = steps(&cli["agent"]["open"]);
    let sb = steps(&open["steps"]);
    if sa != sb || sa.len() != 5 {
        diffs.push(format!("launch steps: cli {sa:?} / desktop {sb:?}"));
    }
    diffs
}

#[test]
fn cli_and_desktop_create_the_same_java_project_with_a_mocked_agent_launch() {
    let diffs = compare("java", None);
    assert!(diffs.is_empty(), "differences:\n{}", diffs.join("\n"));
}

#[test]
#[ignore = "needs the real creation tools: TUMR_PARITY_ALL=1 TUMR_PARITY_TOOLCHAINS=<folder>"]
fn cli_and_desktop_agree_for_all_five_frameworks() {
    if std::env::var("TUMR_PARITY_ALL").as_deref() != Ok("1") {
        return;
    }
    let toolchains = std::env::var("TUMR_PARITY_TOOLCHAINS").ok();
    let mut all = Vec::new();
    for fw in ["expo", "react-native", "flutter", "kotlin", "java"] {
        let t0 = std::time::Instant::now();
        let d = compare(fw, toolchains.as_deref());
        eprintln!(
            "{fw}: {} difference(s) in {} s",
            d.len(),
            t0.elapsed().as_secs()
        );
        all.extend(d.into_iter().map(|x| format!("{fw}: {x}")));
    }
    assert!(all.is_empty(), "differences:\n{}", all.join("\n"));
}
