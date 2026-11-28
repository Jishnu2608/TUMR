//! CRT-C: `tumr create`, the PowerShell wizard of TUMR Create. Every value can also be given as a
//! flag; anything missing is asked for when a person is at the terminal, and is a usage error
//! otherwise. The project itself is made by the Runtime Core's `project.create` job (the same one
//! the desktop wizard uses); this file only asks, checks, follows progress and connects the agent.
//!
//! Steps: framework · name · package id · destination (a new folder inside a parent, or an
//! existing empty folder; paths with spaces, quoted or not) · agent · trust · environment check
//! (offers to install a missing creation tool, with its licences) · summary · creation with live
//! progress (Ctrl+C cancels and rolls back) · trust confirmation when unexpected files were found
//! (CRT-D1) · agent connection (CRT-D5: user scope once if missing, plus project instructions,
//! backed up) · optional agent launch (CRT-D) · summary.
//!
//! Exit codes: 0 created · 1 creation failed (rolled back) or the runtime failed · 2 invalid
//! input or a missing value without a terminal · 3 cancelled (Ctrl+C, or "no" at the summary) ·
//! 4 a tool the framework's creator needs is not installed.

use std::io::{IsTerminal as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use serde_json::{json, Value};
use tumr_api::client::Client as ApiClient;
use tumr_api::methods;
use tumr_create::request::{check_name, check_package_id, default_package_id, validate};
use tumr_create::{CreateRequest, Destination, Framework};
use tumr_resolver::plan::{Action, Plan};

use crate::agent_open::{self, Agent, State};
use crate::core_process::CoreTarget;

pub const EXIT_FAILED: u8 = 1;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_CANCELLED: u8 = 3;
pub const EXIT_TOOLS: u8 = 4;

/// The flags of `tumr create` (one field per flag).
#[derive(Debug, Default, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct CreateArgs {
    pub framework: Option<String>,
    pub name: Option<String>,
    pub package_id: Option<String>,
    /// An existing, empty folder that becomes the project.
    pub path: Option<PathBuf>,
    /// A folder in which TUMR creates `<name>`.
    pub parent: Option<PathBuf>,
    /// An agent id, or `none`.
    pub agent: Option<String>,
    pub no_trust: bool,
    /// Install a missing creation tool without asking first (licences are still shown).
    pub install: bool,
    /// `Some(true)`: open the agent afterwards; `Some(false)`: do not; `None`: ask.
    pub open: Option<bool>,
    /// Never ask: values with a default take it, confirmations are given (the summary, the
    /// agent's config changes); trust despite unexpected files still needs a person.
    pub yes: bool,
    pub json: bool,
}

/// Why the wizard stopped.
#[derive(Debug)]
struct Stop {
    code: u8,
    message: String,
}

fn stop(code: u8, message: impl Into<String>) -> Stop {
    Stop {
        code,
        message: message.into(),
    }
}

// Tests answer the wizard's questions from a script instead of a terminal.
#[cfg(test)]
thread_local! {
    static SCRIPT: std::cell::RefCell<Option<std::collections::VecDeque<String>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn scripted() -> bool {
    SCRIPT.with(|s| s.borrow().is_some())
}

#[cfg(not(test))]
fn scripted() -> bool {
    false
}

/// A person can answer questions.
fn interactive(a: &CreateArgs) -> bool {
    !a.json && !a.yes && (scripted() || std::io::stdin().is_terminal())
}

fn prompt(question: &str) -> Result<String, Stop> {
    print!("{question}");
    let _ = std::io::stdout().flush();
    #[cfg(test)]
    if scripted() {
        let next = SCRIPT.with(|s| {
            s.borrow_mut()
                .as_mut()
                .and_then(std::collections::VecDeque::pop_front)
        });
        println!("{}", next.as_deref().unwrap_or("<end of input>"));
        return next.ok_or_else(|| {
            stop(
                EXIT_CANCELLED,
                "stopped (end of input); nothing was created",
            )
        });
    }
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) => Err(stop(
            EXIT_CANCELLED,
            "stopped (end of input); nothing was created",
        )),
        Ok(_) => Ok(line.trim().to_string()),
        Err(e) => Err(stop(EXIT_FAILED, format!("cannot read the answer: {e}"))),
    }
}

/// Yes/no with a default (Enter).
fn confirm(question: &str, default: bool) -> Result<bool, Stop> {
    let hint = if default { "[Y/n]" } else { "[y/N]" };
    let a = prompt(&format!("{question} {hint} "))?.to_ascii_lowercase();
    Ok(match a.as_str() {
        "" => default,
        "y" | "yes" => true,
        _ => false,
    })
}

/// A path as typed or pasted: surrounding quotes removed, made absolute (from the current
/// folder). `~` is not expanded.
#[must_use]
pub fn clean_path(raw: &str) -> PathBuf {
    let t = raw.trim();
    let t = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(t)
        .trim();
    long_form(&PathBuf::from(crate::session_cmd::absolute(Path::new(t))))
}

/// The path with its existing part in its real, long form (an 8.3 name such as `JOHNSM~1`
/// becomes the folder's real name), so trust and recent projects record the path TUMR finds
/// later. The part that does not exist yet is kept as typed.
fn long_form(p: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut cur = p.to_path_buf();
    loop {
        if let Ok(real) = std::fs::canonicalize(&cur) {
            let shown = real.display().to_string();
            let mut out = PathBuf::from(shown.strip_prefix(r"\\?\").unwrap_or(&shown));
            for part in rest.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (
            cur.file_name().map(std::ffi::OsStr::to_os_string),
            cur.parent(),
        ) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                cur = parent.to_path_buf();
            }
            _ => return p.to_path_buf(),
        }
    }
}

// ---- the questions -----------------------------------------------------------------------------

fn ask_framework(a: &CreateArgs) -> Result<Framework, Stop> {
    if let Some(f) = &a.framework {
        return Framework::parse(f).ok_or_else(|| {
            stop(
                EXIT_USAGE,
                format!(
                    "unknown framework '{f}'; choose expo, react-native, flutter, kotlin or java"
                ),
            )
        });
    }
    if !interactive(a) {
        return Err(stop(
            EXIT_USAGE,
            "--framework is needed (expo, react-native, flutter, kotlin or java)",
        ));
    }
    println!("Framework:");
    for (i, f) in Framework::ALL.iter().enumerate() {
        println!("  {}. {:<13} {}", i + 1, f.name(), f.description());
    }
    loop {
        let ans = prompt("Choose 1-5 (or its name): ")?;
        let by_number = ans
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| Framework::ALL.get(i).copied());
        let by_name = Framework::parse(&ans.replace(' ', "-")).or_else(|| {
            Framework::ALL
                .into_iter()
                .find(|f| f.name().eq_ignore_ascii_case(&ans))
        });
        if let Some(f) = by_number.or(by_name) {
            return Ok(f);
        }
        println!("  Please type a number from 1 to 5.");
    }
}

fn ask_name(a: &CreateArgs) -> Result<String, Stop> {
    if let Some(n) = &a.name {
        check_name(n).map_err(|e| stop(EXIT_USAGE, format!("--name: {e}")))?;
        return Ok(n.trim().to_string());
    }
    if !interactive(a) {
        return Err(stop(EXIT_USAGE, "--name is needed"));
    }
    loop {
        let n = prompt("Project name: ")?;
        match check_name(&n) {
            Ok(()) => return Ok(n),
            Err(e) => println!("  {e}"),
        }
    }
}

fn ask_package(a: &CreateArgs, name: &str) -> Result<String, Stop> {
    let suggested = default_package_id(name);
    if let Some(p) = &a.package_id {
        check_package_id(p).map_err(|e| stop(EXIT_USAGE, format!("--package-id: {e}")))?;
        return Ok(p.trim().to_string());
    }
    if !interactive(a) {
        return Ok(suggested);
    }
    loop {
        let p = prompt(&format!("Android package id [{suggested}]: "))?;
        let p = if p.is_empty() { suggested.clone() } else { p };
        match check_package_id(&p) {
            Ok(()) => return Ok(p),
            Err(e) => println!("  {e}"),
        }
    }
}

fn request(
    f: Framework,
    name: &str,
    pkg: &str,
    d: Destination,
    agent: Option<&Agent>,
    trust: bool,
) -> CreateRequest {
    CreateRequest {
        framework: f,
        name: name.to_string(),
        package_id: Some(pkg.to_string()),
        destination: d,
        agent: agent.map(|a| a.id.to_string()),
        trust,
    }
}

/// Where the project goes; checked against the disk (empty folder, no project there, …).
fn ask_destination(
    a: &CreateArgs,
    f: Framework,
    name: &str,
    pkg: &str,
) -> Result<Destination, Stop> {
    let check =
        |d: Destination| validate(&request(f, name, pkg, d.clone(), None, false)).map(|_| d);
    if a.path.is_some() && a.parent.is_some() {
        return Err(stop(
            EXIT_USAGE,
            "give --path (an existing empty folder) or --parent (a new folder inside it), not both",
        ));
    }
    if let Some(p) = &a.path {
        let path = clean_path(&p.display().to_string()).display().to_string();
        return check(Destination::ExistingEmpty { path }).map_err(|e| stop(EXIT_USAGE, e));
    }
    if let Some(p) = &a.parent {
        let parent = clean_path(&p.display().to_string()).display().to_string();
        return check(Destination::NewIn { parent }).map_err(|e| stop(EXIT_USAGE, e));
    }
    let here = clean_path(".").display().to_string();
    if !interactive(a) {
        // default: a new folder named after the project in the current folder
        return check(Destination::NewIn { parent: here }).map_err(|e| stop(EXIT_USAGE, e));
    }
    println!("Where should the project go?");
    println!("  1. A new folder \"{name}\" inside a folder you choose");
    println!("  2. An existing, empty folder");
    loop {
        let choice = prompt("Choose 1 or 2 [1]: ")?;
        let d = match choice.as_str() {
            "" | "1" => {
                let p = prompt(&format!("Parent folder [{here}]: "))?;
                let parent = if p.is_empty() {
                    here.clone()
                } else {
                    clean_path(&p).display().to_string()
                };
                Destination::NewIn { parent }
            }
            "2" => {
                let p = prompt("Empty folder: ")?;
                if p.is_empty() {
                    continue;
                }
                Destination::ExistingEmpty {
                    path: clean_path(&p).display().to_string(),
                }
            }
            _ => {
                println!("  Please type 1 or 2.");
                continue;
            }
        };
        match check(d) {
            Ok(d) => return Ok(d),
            Err(e) => println!("  {e}"),
        }
    }
}

fn ask_agent(a: &CreateArgs, folder_hint: &Path) -> Result<Option<&'static Agent>, Stop> {
    if let Some(id) = &a.agent {
        if id.eq_ignore_ascii_case("none") {
            return Ok(None);
        }
        return agent_open::find(id).map(Some).ok_or_else(|| {
            stop(
                EXIT_USAGE,
                format!(
                    "unknown agent '{id}'; known: {}, or none",
                    agent_open::ids()
                ),
            )
        });
    }
    if !interactive(a) {
        return Ok(None);
    }
    println!("Coding agent for this project:");
    for (i, ag) in agent_open::AGENTS.iter().enumerate() {
        let s = agent_open::state(ag, folder_hint);
        println!("  {}. {:<26} {}", i + 1, ag.name, s.label());
    }
    println!("  6. None (choose later)");
    loop {
        let ans = prompt("Choose 1-6 [6]: ")?;
        match ans.as_str() {
            "" | "6" => return Ok(None),
            n => {
                if let Some(ag) = n
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| n.checked_sub(1))
                    .and_then(|i| agent_open::AGENTS.get(i))
                {
                    return Ok(Some(ag));
                }
                if let Some(ag) = agent_open::find(n) {
                    return Ok(Some(ag));
                }
                println!("  Please type a number from 1 to 6.");
            }
        }
    }
}

// ---- environment check ---------------------------------------------------------------------------

/// The creation tools of the plan: (found, missing) titles, and what running needs later.
#[must_use]
pub fn tools_from_plan(f: Framework, plan: &Plan) -> (Vec<String>, Vec<String>, Vec<String>) {
    let needed = f.creation_tools();
    let (mut found, mut missing, mut later) = (Vec::new(), Vec::new(), Vec::new());
    for i in &plan.items {
        let tool = needed.contains(&i.component.as_str());
        match (&i.action, tool) {
            (Action::Reuse { version, .. }, true) => found.push(match version {
                Some(v) => format!("{} {v}", i.title),
                None => i.title.clone(),
            }),
            (Action::Install, true) => missing.push(i.title.clone()),
            (Action::Install, false) => later.push(i.title.clone()),
            (Action::Reuse { .. }, false) => {}
        }
    }
    (found, missing, later)
}

async fn plan_for(client: &mut ApiClient, f: Framework, target: &Path) -> Result<Plan, Stop> {
    let v: Value = client
        .call(
            methods::PROJECT_PLAN,
            json!({ "path": target.display().to_string(), "framework": f.id() }),
        )
        .await
        .map_err(|e| stop(EXIT_FAILED, format!("cannot check the environment: {e}")))?;
    serde_json::from_value(v["plan"].clone()).map_err(|e| {
        stop(
            EXIT_FAILED,
            format!("unexpected plan from the runtime: {e}"),
        )
    })
}

/// Checks (and on approval installs) the framework's creation tools.
async fn environment(
    target: &CoreTarget,
    client: &mut ApiClient,
    a: &CreateArgs,
    f: Framework,
    dir: &Path,
) -> Result<(), Stop> {
    let say = !a.json;
    if say {
        println!("\nChecking the environment for {}…", f.name());
    }
    let plan = plan_for(client, f, dir).await?;
    let (found, missing, later) = tools_from_plan(f, &plan);
    if say {
        if f.creation_tools().is_empty() {
            println!(
                "  {} is created from TUMR's own template: no tool needed.",
                f.name()
            );
        }
        for t in &found {
            println!("  [ok] {t}");
        }
        for t in &missing {
            println!("  [missing] {t}");
        }
        if !later.is_empty() {
            println!("  To build and run it later TUMR also needs: {} (`tumr run` offers to install them).", later.join(", "));
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let what = missing.join(", ");
    let go = a.install
        || (interactive(a)
            && confirm(
                &format!("\nInstall {what} now (into TUMR's managed folder)?"),
                true,
            )?);
    if !go {
        return Err(stop(
            EXIT_TOOLS,
            format!("{} needs {what} first; nothing was created. Install it with `tumr create … --install`, in a terminal", f.name()),
        ));
    }
    let params = json!({ "path": dir.display().to_string(), "framework": f.id() });
    if crate::install_cmd::install_params(target, params, a.yes || a.install).await
        != ExitCode::SUCCESS
    {
        return Err(stop(
            EXIT_TOOLS,
            format!("{what} was not installed; nothing was created"),
        ));
    }
    let (_, still, _) = tools_from_plan(f, &plan_for(client, f, dir).await?);
    if still.is_empty() {
        Ok(())
    } else {
        Err(stop(
            EXIT_TOOLS,
            format!(
                "still missing after the install: {}; nothing was created",
                still.join(", ")
            ),
        ))
    }
}

// ---- creation ------------------------------------------------------------------------------------

/// Follows the job; Ctrl+C cancels it. Returns its final status.
async fn follow(client: &mut ApiClient, job: &str, say: bool) -> Result<Value, Stop> {
    let mut shown = 0usize;
    let mut cancelled = false;
    loop {
        let st: Value = client
            .call(methods::PROJECT_CREATE_STATUS, json!({ "job_id": job }))
            .await
            .map_err(|e| stop(EXIT_FAILED, format!("lost the creation's status: {e}")))?;
        let progress = st["progress"].as_array().cloned().unwrap_or_default();
        for p in progress.iter().skip(shown) {
            if say {
                println!(
                    "  [{}] {}",
                    p["stage"].as_str().unwrap_or("?"),
                    p["message"].as_str().unwrap_or("")
                );
            }
        }
        shown = progress.len();
        if st["state"] != "running" {
            return Ok(st);
        }
        tokio::select! {
            () = tokio::time::sleep(Duration::from_millis(300)) => {}
            _ = tokio::signal::ctrl_c(), if !cancelled => {
                cancelled = true;
                if say {
                    println!("\nCancelling: TUMR stops the tool and removes what this creation made…");
                }
                let _ = client.call_raw(methods::PROJECT_CREATE_CANCEL, json!({ "job_id": job })).await;
            }
        }
    }
}

/// CRT-D1: unexpected files hold trust until the user confirms.
async fn confirm_trust(
    client: &mut ApiClient,
    a: &CreateArgs,
    st: &Value,
    path: &str,
) -> Result<bool, Stop> {
    let list: Vec<&str> = st["result"]["unexpected"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let count = st["result"]["unexpected_count"].as_u64().unwrap_or(0);
    if !a.json {
        println!(
            "\nThe project holds {count} file(s) the {} creator does not normally make:",
            st["framework"].as_str().unwrap_or("?")
        );
        for f in &list {
            println!("  {f}");
        }
        if usize::try_from(count).unwrap_or(0) > list.len() {
            println!(
                "  … and {} more",
                usize::try_from(count).unwrap_or(0) - list.len()
            );
        }
    }
    if !interactive(a)
        || !confirm(
            "Check them. Trust the project anyway (its build scripts may then run)?",
            false,
        )?
    {
        if !a.json {
            println!("Not trusted. Trust it later with: tumr trust \"{path}\"");
        }
        return Ok(false);
    }
    client
        .call_raw(
            methods::PROJECT_TRUST,
            json!({ "path": path, "trusted": true }),
        )
        .await
        .map_err(|e| stop(EXIT_FAILED, format!("cannot trust the project: {e}")))?;
    Ok(true)
}

// ---- agent ---------------------------------------------------------------------------------------

/// CRT-D5: the user-scope entry once if it is missing, plus the project's instructions; shown,
/// asked, backed up. Returns `applied`, `up-to-date`, `skipped` or `declined`.
async fn connect_agent(
    target: &CoreTarget,
    a: &CreateArgs,
    ag: &Agent,
    project: &Path,
) -> Result<&'static str, String> {
    use crate::agent_setup as setup;
    let c = ag.client();
    let exe = setup::current_exe()?;
    let changes = setup::new_project_plan(c, project, &exe)?;
    let has_token = crate::mcp_cmd::token_path(target, c.id).is_file();
    if changes.is_empty() && has_token {
        return Ok("up-to-date");
    }
    if !a.json {
        println!(
            "\nConnecting {} to TUMR (user scope once, plus this project's instructions):",
            ag.name
        );
        setup::show(&changes);
    }
    if !a.yes {
        if !interactive(a) {
            return Ok("skipped");
        }
        if !confirm(
            "Apply these changes (other servers and settings are kept)?",
            true,
        )
        .map_err(|s| s.message)?
        {
            return Ok("declined");
        }
    }
    crate::agent_cmd::ensure_client(
        target,
        c.id,
        &crate::agent_cmd::AGENT_SCOPES.map(String::from),
    )
    .await?;
    setup::apply_backed(&changes, project)?;
    Ok("applied")
}

// ---- the command ---------------------------------------------------------------------------------

/// The answers, before anything is written.
#[derive(Debug)]
struct Choices {
    framework: Framework,
    name: String,
    package_id: String,
    destination: Destination,
    target: PathBuf,
    agent: Option<&'static Agent>,
    trust: bool,
}

fn gather(a: &CreateArgs) -> Result<Choices, Stop> {
    if interactive(a) {
        println!("TUMR Create: a new, minimal mobile project (Ctrl+C stops at any time; nothing is written before the summary).\n");
    }
    let framework = ask_framework(a)?;
    let name = ask_name(a)?;
    let package_id = ask_package(a, &name)?;
    let destination = ask_destination(a, framework, &name, &package_id)?;
    let target = validate(&request(
        framework,
        &name,
        &package_id,
        destination.clone(),
        None,
        false,
    ))
    .map_err(|e| stop(EXIT_USAGE, e))?
    .target;
    let hint = target.parent().unwrap_or(&target).to_path_buf();
    let agent = ask_agent(a, &hint)?;
    let trust = if a.no_trust {
        false
    } else if interactive(a) {
        confirm(
            "Trust the new project so TUMR may build it (only this folder)?",
            true,
        )?
    } else {
        true
    };
    Ok(Choices {
        framework,
        name,
        package_id,
        destination,
        target,
        agent,
        trust,
    })
}

fn print_summary(c: &Choices) {
    println!("\nSummary:");
    println!(
        "  Framework   {} ({})",
        c.framework.name(),
        c.framework.description()
    );
    println!("  Name        {}", c.name);
    println!("  Package id  {}", c.package_id);
    println!(
        "  Folder      {}{}",
        c.target.display(),
        match c.destination {
            Destination::NewIn { .. } => " (new)",
            Destination::ExistingEmpty { .. } => " (existing, empty)",
        }
    );
    println!("  Agent       {}", c.agent.map_or("none", |g| g.name));
    println!(
        "  Trust       {}",
        if c.trust {
            "yes, this folder only"
        } else {
            "no"
        }
    );
}

/// Runs everything after the questions; fills `out` for `--json`.
async fn run_steps(
    target: &CoreTarget,
    a: &CreateArgs,
    c: &Choices,
    out: &mut Value,
) -> Result<(), Stop> {
    let (mut client, _) = target
        .connect_or_start()
        .await
        .map_err(|e| stop(EXIT_FAILED, e))?;
    environment(target, &mut client, a, c.framework, &c.target).await?;
    if !a.json {
        print_summary(c);
        if let Some(ag) = c.agent {
            let s = agent_open::state(ag, c.target.parent().unwrap_or(&c.target));
            if matches!(s, State::NotInstalled | State::Unsupported) {
                println!("  Note: {} is {} on this PC; TUMR records it and skips connecting and opening.", ag.name, s.label());
            }
        }
    }
    if !a.yes && interactive(a) && !confirm("\nCreate the project?", true)? {
        return Err(stop(EXIT_CANCELLED, "nothing was created"));
    }
    let req = request(
        c.framework,
        &c.name,
        &c.package_id,
        c.destination.clone(),
        c.agent,
        c.trust,
    );
    let started: Value = client
        .call(
            methods::PROJECT_CREATE_START,
            serde_json::to_value(&req).unwrap_or_default(),
        )
        .await
        .map_err(|e| {
            let m = e.to_string();
            let code = if m.contains("needs") {
                EXIT_TOOLS
            } else {
                EXIT_FAILED
            };
            stop(code, m)
        })?;
    let job = started["job_id"].as_str().unwrap_or_default().to_string();
    if !a.json {
        println!("\nCreating {} ({job})…", c.name);
    }
    let st = follow(&mut client, &job, !a.json).await?;
    out["status"] = st.clone();
    match st["state"].as_str() {
        Some("done") => {}
        Some("cancelled") => {
            return Err(stop(
                EXIT_CANCELLED,
                format!(
                    "cancelled; {}",
                    st["error"]["cleanup"].as_str().unwrap_or("")
                ),
            ));
        }
        _ => {
            return Err(stop(
                EXIT_FAILED,
                format!(
                    "the creation failed at {}: {} ({})",
                    st["error"]["stage"].as_str().unwrap_or("?"),
                    st["error"]["message"].as_str().unwrap_or("?"),
                    st["error"]["cleanup"].as_str().unwrap_or("")
                ),
            ));
        }
    }
    let path = st["result"]["path"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let mut trusted = st["trusted"] == true;
    if st["trust_needs_confirmation"] == true {
        trusted = confirm_trust(&mut client, a, &st, &path).await?;
    }
    out["trusted"] = json!(trusted);
    drop(client);
    agent_steps(target, a, c, Path::new(&path), out).await;
    Ok(())
}

/// Connect and (optionally) open the agent; problems here never undo the project.
async fn agent_steps(
    target: &CoreTarget,
    a: &CreateArgs,
    c: &Choices,
    project: &Path,
    out: &mut Value,
) {
    let Some(ag) = c.agent else { return };
    let before = agent_open::state(ag, project);
    out["agent"] = json!({ "id": ag.id, "state_before": before.word() });
    if matches!(before, State::NotInstalled | State::Unsupported) {
        out["agent"]["connect"] = json!("skipped");
        return;
    }
    let connect = match connect_agent(target, a, ag, project).await {
        Ok(r) => r,
        Err(e) => {
            if !a.json {
                eprintln!(
                    "tumr: could not connect {}: {e}. Try `tumr agent setup {}` later.",
                    ag.name, ag.client
                );
            }
            "failed"
        }
    };
    out["agent"]["connect"] = json!(connect);
    let open = match a.open {
        Some(yes) => yes,
        None => {
            interactive(a)
                && confirm(&format!("\nOpen {} in the project now?", ag.name), true)
                    .unwrap_or(false)
        }
    };
    if !open {
        return;
    }
    let opts = agent_open::OpenOptions {
        wait: 20,
        json: a.json,
    };
    if !a.json {
        println!("\nOpening {}…", ag.name);
    }
    match agent_open::open_steps(target, ag, project, opts).await {
        Ok(steps) => {
            if !a.json {
                agent_open::print_steps(&steps);
            }
            out["agent"]["open"] = agent_open::steps_json(&steps);
        }
        Err(e) => {
            if !a.json {
                eprintln!("tumr: {e}");
            }
            out["agent"]["open_error"] = json!(e);
        }
    }
}

fn final_summary(c: &Choices, out: &Value) {
    let st = &out["status"];
    println!(
        "\nDone: {} project \"{}\" in {}",
        c.framework.name(),
        c.name,
        st["result"]["path"].as_str().unwrap_or("?")
    );
    println!(
        "  Detected by TUMR with confidence {}",
        st["result"]["confidence"]
    );
    println!(
        "  Trusted      {}",
        if out["trusted"] == true {
            "yes"
        } else {
            "no (tumr trust <folder>)"
        }
    );
    if let Some(w) = st["result"]["warnings"]
        .as_array()
        .filter(|w| !w.is_empty())
    {
        for x in w {
            println!("  Warning: {}", x.as_str().unwrap_or(""));
        }
    }
    if let Some(ag) = c.agent {
        println!(
            "  Agent        {} (connection: {})",
            ag.name,
            out["agent"]["connect"].as_str().unwrap_or("not done")
        );
    }
    println!(
        "\nNext: cd \"{}\" then `tumr run` (it offers to install what building and running need).",
        st["result"]["path"].as_str().unwrap_or(".")
    );
}

/// `tumr create …`
pub async fn create(target: &CoreTarget, a: CreateArgs) -> ExitCode {
    let mut out = json!({});
    let result = match gather(&a) {
        Ok(c) => run_steps(target, &a, &c, &mut out).await.map(|()| c),
        Err(s) => Err(s),
    };
    match result {
        Ok(c) => {
            if a.json {
                out["created"] = json!(true);
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            } else {
                final_summary(&c, &out);
            }
            ExitCode::SUCCESS
        }
        Err(s) => {
            if a.json {
                out["created"] = json!(false);
                out["exit_code"] = json!(s.code);
                out["error"] = json!(s.message);
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            } else {
                eprintln!("tumr: {}", s.message);
            }
            ExitCode::from(s.code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tumr_resolver::plan::PlanItem;

    #[test]
    fn pasted_paths_lose_their_quotes_and_become_absolute() {
        let p = clean_path(r#"  "D:\My Apps\Hello"  "#);
        assert_eq!(p, PathBuf::from(r"D:\My Apps\Hello"));
        assert_eq!(clean_path(r"'C:\a b'"), PathBuf::from(r"C:\a b"));
        // the existing part gets its real long form (8.3 names such as JOHNSM~1 resolved), the
        // rest is kept as typed
        let tmp = std::env::temp_dir();
        let real = std::fs::canonicalize(&tmp).unwrap().display().to_string();
        let real = PathBuf::from(real.strip_prefix(r"\\?\").unwrap_or(&real));
        assert_eq!(
            long_form(&tmp.join("not yet").join("App")),
            real.join("not yet").join("App")
        );
        assert!(clean_path("rel dir").is_absolute());
    }

    fn args() -> CreateArgs {
        CreateArgs {
            json: true,
            ..CreateArgs::default()
        }
    }

    /// Runs `gather` with the wizard reading `answers` as if typed.
    fn wizard(answers: &[&str], a: &CreateArgs) -> Result<Choices, Stop> {
        SCRIPT.with(|s| *s.borrow_mut() = Some(answers.iter().map(|x| (*x).to_string()).collect()));
        let r = gather(a);
        SCRIPT.with(|s| *s.borrow_mut() = None);
        r
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tumr-crtc-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_wizard_asks_again_after_bad_answers() {
        let parent = temp("parent").join("My Apps");
        std::fs::create_dir_all(&parent).unwrap();
        let quoted = format!("\"{}\"", parent.display());
        let c = wizard(
            &[
                "7",           // not a framework
                "flutter",     // by id
                "1 bad",       // invalid name
                "Hello World", // ok
                "nodots",      // invalid package id
                "",            // the suggestion
                "3",           // not a destination choice
                "1",           // new folder in a parent
                &quoted,       // pasted with quotes, has a space
                "9",           // not an agent
                "2",           // Codex
                "n",           // do not trust
            ],
            &CreateArgs::default(),
        )
        .unwrap();
        assert_eq!(c.framework, Framework::Flutter);
        assert_eq!(c.name, "Hello World");
        assert_eq!(c.package_id, "com.example.helloworld");
        assert_eq!(c.target, long_form(&parent.join("Hello World")));
        assert_eq!(c.agent.unwrap().id, "codex");
        assert!(!c.trust);
        let _ = std::fs::remove_dir_all(parent.parent().unwrap());
    }

    #[test]
    fn the_wizard_takes_an_existing_empty_folder_and_refuses_a_full_one() {
        let root = temp("existing");
        let full = root.join("full");
        std::fs::create_dir_all(&full).unwrap();
        std::fs::write(full.join("a.txt"), "x").unwrap();
        let empty = root.join("empty one");
        std::fs::create_dir_all(&empty).unwrap();
        let full_s = full.display().to_string();
        let empty_s = empty.display().to_string();
        let c = wizard(
            &[
                "5",
                "Notes",
                "com.acme.notes",
                "2",
                &full_s,
                "2",
                &empty_s,
                "",
                "",
            ],
            &CreateArgs::default(),
        )
        .unwrap();
        assert_eq!(c.framework, Framework::Java);
        assert_eq!(c.target, long_form(&empty));
        assert!(matches!(c.destination, Destination::ExistingEmpty { .. }));
        assert!(c.agent.is_none(), "Enter = none");
        assert!(c.trust, "Enter = trust (CRT-D1: ticked by default)");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn flags_skip_their_questions_and_end_of_input_cancels() {
        let root = temp("flags");
        let a = CreateArgs {
            framework: Some("kotlin".into()),
            name: Some("Calc".into()),
            parent: Some(root.clone()),
            agent: Some("none".into()),
            no_trust: true,
            ..CreateArgs::default()
        };
        // only the package id is asked
        let c = wizard(&[""], &a).unwrap();
        assert_eq!(c.target, long_form(&root.join("Calc")));
        // the person stops typing (Ctrl+Z / closed input): cancelled, nothing written
        let e = wizard(&[], &CreateArgs::default()).unwrap_err();
        assert_eq!(e.code, EXIT_CANCELLED);
        assert!(!root.join("Calc").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn confirm_uses_its_default_on_enter() {
        SCRIPT.with(|s| {
            *s.borrow_mut() = Some(
                ["", "", "y", "no"]
                    .iter()
                    .map(|x| (*x).to_string())
                    .collect(),
            );
        });
        assert!(confirm("q", true).unwrap());
        assert!(!confirm("q", false).unwrap());
        assert!(confirm("q", false).unwrap());
        assert!(!confirm("q", true).unwrap());
        SCRIPT.with(|s| *s.borrow_mut() = None);
    }

    #[test]
    fn missing_values_without_a_terminal_are_usage_errors() {
        let e = gather(&args()).unwrap_err();
        assert_eq!(e.code, EXIT_USAGE);
        assert!(e.message.contains("--framework"), "{}", e.message);
        let a = CreateArgs {
            framework: Some("flutter".into()),
            ..args()
        };
        assert!(gather(&a).unwrap_err().message.contains("--name"));
    }

    #[test]
    fn invalid_flags_are_refused_before_anything_is_written() {
        let base = CreateArgs {
            framework: Some("java".into()),
            name: Some("Hello".into()),
            ..args()
        };
        let bad = [
            CreateArgs {
                framework: Some("swift".into()),
                ..base.clone()
            },
            CreateArgs {
                name: Some("1bad".into()),
                ..base.clone()
            },
            CreateArgs {
                package_id: Some("nodots".into()),
                ..base.clone()
            },
            CreateArgs {
                agent: Some("notepad".into()),
                ..base.clone()
            },
            CreateArgs {
                path: Some(r"C:\Windows".into()),
                ..base.clone()
            },
            CreateArgs {
                path: Some("x".into()),
                parent: Some("y".into()),
                ..base.clone()
            },
        ];
        for b in bad {
            let e = gather(&b).unwrap_err();
            assert_eq!(e.code, EXIT_USAGE, "{b:?}: {}", e.message);
        }
    }

    #[test]
    fn flags_alone_make_a_complete_choice() {
        let dir = std::env::temp_dir().join(format!("tumr-create-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = CreateArgs {
            framework: Some("react-native".into()),
            name: Some("Hello App".into()),
            parent: Some(dir.clone()),
            agent: Some("vscode".into()),
            ..args()
        };
        let c = gather(&a).unwrap();
        assert_eq!(c.framework, Framework::ReactNative);
        assert_eq!(c.package_id, "com.example.helloapp");
        assert_eq!(c.target, long_form(&dir.join("Hello App")));
        assert_eq!(c.agent.unwrap().id, "copilot");
        assert!(c.trust, "trust is on unless --no-trust (CRT-D1)");
        let none = CreateArgs {
            agent: Some("none".into()),
            no_trust: true,
            ..a
        };
        let c = gather(&none).unwrap();
        assert!(c.agent.is_none() && !c.trust);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn item(component: &str, action: &Action) -> PlanItem {
        serde_json::from_value(json!({
            "component": component, "title": component.to_uppercase(), "action": action,
            "approx_download_bytes": 0, "approx_disk_bytes": 0, "reason": "", "sources": []
        }))
        .unwrap()
    }

    #[test]
    fn the_plan_splits_into_creation_tools_and_later_needs() {
        let mut plan: Plan = serde_json::from_value(json!({
            "project": "", "profile": "", "pack_version": "", "items": [], "licences": [],
            "admin_actions": [], "conflicts": [], "notes": [], "total_download_bytes": 0,
            "total_disk_bytes": 0, "disk_free_bytes": 0, "disk_ok": true
        }))
        .unwrap();
        plan.items = vec![
            item("flutter", &Action::Install),
            item(
                "git",
                &Action::Reuse {
                    path: "C:\\git".into(),
                    version: Some("2.50".into()),
                },
            ),
            item("jdk", &Action::Install),
            item(
                "node",
                &Action::Reuse {
                    path: "C:\\node".into(),
                    version: None,
                },
            ),
        ];
        let (found, missing, later) = tools_from_plan(Framework::Flutter, &plan);
        assert_eq!(found, ["GIT 2.50"]);
        assert_eq!(missing, ["FLUTTER"]);
        assert_eq!(later, ["JDK"]);
        let (f, m, _) = tools_from_plan(Framework::Java, &plan);
        assert!(f.is_empty() && m.is_empty(), "Java needs no creation tool");
    }
}
