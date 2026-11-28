//! CRT-D: the five coding agents TUMR Create offers, their state on this PC, and
//! `tumr agent open <agent> [folder]`.
//!
//! States (one per agent and project folder):
//! * `ready`: TUMR can start it and its config already starts the `tumr` MCP server;
//! * `not-configured`: TUMR can start it, but no config starts `tumr` yet (`tumr agent setup`);
//! * `not-installed`: nothing of it was found;
//! * `unsupported`: something of it was found, but TUMR has no way to start it.
//!
//! `open` reports each step as `verified` (TUMR saw it happen), `not-verified` (it may have
//! happened; TUMR cannot see it, or not yet), `manual` (the user does it; the agent offers no
//! supported way) or `failed`. It never reports more than it saw (owner constraint: no simulated
//! success).
//!
//! How each agent is started (checked on this PC, 2026-11-28):
//! * Claude Code, Codex CLI: a new PowerShell window in the project folder runs the CLI;
//! * Cursor, GitHub Copilot (VS Code), Antigravity: the editor's own launcher with the folder;
//! * Codex app (no CLI on `PATH`): the app starts, but it has no supported way to open a folder
//!   from outside (its `codex://` links open threads, not folders), so the folder is a manual step.

use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tumr_api::methods;

use crate::agent_clients::{self, Client};
use crate::agent_cmd::{call, find_client};
use crate::core_process::CoreTarget;
use crate::fail;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// One agent TUMR Create offers.
#[derive(Debug, Clone, Copy)]
pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    /// Its entry in [`agent_clients::CLIENTS`] (config files, setup).
    pub client: &'static str,
}

/// The five agents, in the order the wizards list them.
pub const AGENTS: [Agent; 5] = [
    Agent {
        id: "claude-code",
        name: "Claude Code",
        client: "claude-code",
    },
    Agent {
        id: "codex",
        name: "Codex",
        client: "codex",
    },
    Agent {
        id: "cursor",
        name: "Cursor",
        client: "cursor",
    },
    Agent {
        id: "antigravity",
        name: "Google Antigravity",
        client: "antigravity",
    },
    Agent {
        id: "copilot",
        name: "GitHub Copilot (VS Code)",
        client: "vscode",
    },
];

/// The agent for an id; `vscode` and `github-copilot` name Copilot too.
#[must_use]
pub fn find(id: &str) -> Option<&'static Agent> {
    let id = match id.trim().to_ascii_lowercase().as_str() {
        "vscode" | "github-copilot" | "copilot" => "copilot",
        other => return AGENTS.iter().find(|a| a.id == other),
    };
    AGENTS.iter().find(|a| a.id == id)
}

/// Ids for messages.
#[must_use]
pub fn ids() -> String {
    AGENTS.iter().map(|a| a.id).collect::<Vec<_>>().join(", ")
}

impl Agent {
    #[must_use]
    pub fn client(&self) -> &'static Client {
        agent_clients::find(self.client).unwrap_or(&agent_clients::CLIENTS[0])
    }
}

/// Where an agent stands on this PC for one project folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ready,
    NotConfigured,
    NotInstalled,
    Unsupported,
}

impl State {
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NotConfigured => "not-configured",
            Self::NotInstalled => "not-installed",
            Self::Unsupported => "unsupported",
        }
    }

    /// For the wizard's list.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "installed, connected to TUMR",
            Self::NotConfigured => "installed, not connected to TUMR yet",
            Self::NotInstalled => "not installed",
            Self::Unsupported => "found, but TUMR cannot start it",
        }
    }
}

/// How TUMR starts an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// A CLI in a new PowerShell window in the folder (`command` is resolved by PowerShell).
    Terminal {
        command: &'static str,
        program: PathBuf,
    },
    /// An editor launcher that takes the folder; `process` is the editor's program name.
    Editor {
        launcher: PathBuf,
        process: &'static str,
    },
    /// A packaged app started by its app id; it cannot be given a folder.
    App {
        app_id: String,
        process: &'static str,
    },
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// A command on `PATH` (`.exe`, `.cmd`, `.bat` or `.ps1`).
#[must_use]
pub fn which(command: &str) -> Option<PathBuf> {
    which_in(command, &std::env::var_os("PATH")?)
}

fn which_in(command: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|dir| {
        [".exe", ".cmd", ".bat", ".ps1"]
            .iter()
            .map(|ext| dir.join(format!("{command}{ext}")))
            .find(|p| p.is_file())
    })
}

/// The Codex app's package family (`OpenAI.Codex_2p2nqsd0c76g0`), from its data folder.
fn codex_app_family() -> Option<String> {
    let packages = env_dir("LOCALAPPDATA")?.join("Packages");
    std::fs::read_dir(packages)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .find(|n| n.starts_with("OpenAI.Codex_"))
}

fn antigravity_exe() -> Option<PathBuf> {
    which("antigravity").or_else(|| {
        let p = env_dir("LOCALAPPDATA")?
            .join("Programs")
            .join("antigravity")
            .join("Antigravity.exe");
        p.is_file().then_some(p)
    })
}

/// How TUMR would start the agent on this PC; `None` when it cannot.
#[must_use]
pub fn launcher(a: &Agent) -> Option<Launch> {
    match a.id {
        "claude-code" => which("claude").map(|program| Launch::Terminal {
            command: "claude",
            program,
        }),
        "codex" => which("codex")
            .map(|program| Launch::Terminal {
                command: "codex",
                program,
            })
            .or_else(|| {
                codex_app_family().map(|f| Launch::App {
                    app_id: format!("{f}!App"),
                    process: "ChatGPT.exe",
                })
            }),
        "cursor" => which("cursor").map(|launcher| Launch::Editor {
            launcher,
            process: "Cursor.exe",
        }),
        "antigravity" => antigravity_exe().map(|launcher| Launch::Editor {
            launcher,
            process: "Antigravity.exe",
        }),
        "copilot" => which("code").map(|launcher| Launch::Editor {
            launcher,
            process: "Code.exe",
        }),
        _ => None,
    }
}

/// The config file that starts `tumr` for this agent and folder (user scope first).
#[must_use]
pub fn configured_in(c: &Client, project: &Path) -> Option<PathBuf> {
    let user = c.user_config.and_then(|p| p.resolve(None));
    let proj = c.project_config.and_then(|p| p.resolve(Some(project)));
    [user, proj]
        .into_iter()
        .flatten()
        .find(|p| c.configured_in(p))
}

/// The agent's state for a project folder.
#[must_use]
pub fn state(a: &Agent, project: &Path) -> State {
    state_from(
        launcher(a).is_some(),
        a.client().installed(),
        configured_in(a.client(), project).is_some(),
    )
}

/// The rule behind [`state`].
#[must_use]
pub fn state_from(can_start: bool, found: bool, configured: bool) -> State {
    match (can_start, found, configured) {
        (true, _, true) => State::Ready,
        (true, _, false) => State::NotConfigured,
        (false, true, _) => State::Unsupported,
        (false, false, _) => State::NotInstalled,
    }
}

/// The state and launch method of a setup client (`tumr agent clients`). Claude Desktop is no
/// project agent: its state only says whether it is installed and connected.
#[must_use]
pub fn client_state(c: &Client, project: &Path) -> (State, Option<String>) {
    let configured = configured_in(c, project).is_some();
    match AGENTS.iter().find(|a| a.client == c.id) {
        Some(a) => (state(a, project), launcher(a).map(|l| describe(&l))),
        None => (state_from(c.installed(), c.installed(), configured), None),
    }
}

fn describe(l: &Launch) -> String {
    match l {
        Launch::Terminal { command, program } => format!(
            "new PowerShell window in the folder running `{command}` ({})",
            program.display()
        ),
        Launch::Editor { launcher, .. } => format!("{} <folder>", launcher.display()),
        Launch::App { app_id, .. } => {
            format!("the app ({app_id}); the folder is opened in the app by you")
        }
    }
}

// ---- open ----------------------------------------------------------------------------------------

/// One step of `open` and what TUMR saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub id: &'static str,
    pub label: &'static str,
    /// `verified`, `not-verified`, `manual` or `failed`.
    pub status: &'static str,
    pub detail: String,
}

fn step(id: &'static str, label: &'static str, status: &'static str, detail: String) -> Step {
    Step {
        id,
        label,
        status,
        detail,
    }
}

fn running(process: &str) -> Vec<u32> {
    tumr_platform::process::list_processes()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.exe_name.eq_ignore_ascii_case(process))
        .map(|p| p.pid)
        .collect()
}

/// A visible window of `process` whose title names the folder.
fn window_with(process: &str, folder: &str) -> Option<String> {
    let pids = running(process);
    let want = folder.to_lowercase();
    tumr_platform::window::visible_titles()
        .into_iter()
        .find(|(pid, title)| pids.contains(pid) && title.to_lowercase().contains(&want))
        .map(|(_, t)| t)
}

fn poll<T>(limit: Duration, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let end = Instant::now() + limit;
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if Instant::now() >= end {
            return None;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Starts the agent; returns the launch and workspace steps.
fn start(agent: &Agent, launch: &Launch, folder: &Path) -> (Step, Step) {
    match launch {
        Launch::Terminal { command, .. } => start_terminal(command, folder),
        Launch::Editor { launcher, process } => start_editor(agent, launcher, process, folder),
        Launch::App { app_id, process } => start_app(agent, app_id, process, folder),
    }
}

fn not_started(why: String) -> (Step, Step) {
    (
        step("app-launched", "app launched", "failed", why),
        step(
            "workspace-opened",
            "workspace opened",
            "failed",
            "not started".into(),
        ),
    )
}

/// A CLI agent in a new PowerShell window in the folder.
fn start_terminal(command: &str, folder: &Path) -> (Step, Step) {
    {
        // `start` gives PowerShell a console of its own: a child started directly would get
        // tumr's own output handles, and the agent's screen would go into tumr's output.
        // A Windows path cannot contain `"`, so quoting the folder is enough.
        let starter = Command::new("cmd.exe")
            .raw_arg(format!(
                "/d /c start \"TUMR - {command}\" /D \"{}\" powershell.exe -NoExit -NoLogo -Command {command}",
                folder.display()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
        let starter = match starter {
            Ok(c) => c.id(),
            Err(e) => return not_started(format!("cannot start PowerShell: {e}")),
        };
        // the new PowerShell's parent is the `start` shell, which ends at once
        let Some(pid) = poll(Duration::from_secs(10), || {
            tumr_platform::process::list_processes()
                .unwrap_or_default()
                .into_iter()
                .find(|p| {
                    p.parent_pid == starter && p.exe_name.eq_ignore_ascii_case("powershell.exe")
                })
                .map(|p| p.pid)
        }) else {
            return not_started("no PowerShell window was seen opening".into());
        };
        // the CLI runs as a child of the new PowerShell (claude: node.exe or claude.exe)
        let seen = poll(Duration::from_secs(8), || {
            if !tumr_platform::process::is_running(pid).unwrap_or(true) {
                return Some(false);
            }
            tumr_platform::process::descendants(pid)
                .unwrap_or_default()
                .iter()
                .any(|p| {
                    let n = p.exe_name.to_ascii_lowercase();
                    n.starts_with(command) || n == "node.exe"
                })
                .then_some(true)
        });
        match seen {
                Some(true) => (
                    step("app-launched", "app launched", "verified", format!("`{command}` runs in a new PowerShell window (pid {pid})")),
                    step("workspace-opened", "workspace opened", "verified", format!("`{command}` was started in {}", folder.display())),
                ),
                Some(false) => (
                    step("app-launched", "app launched", "failed", "the PowerShell window closed at once".into()),
                    step("workspace-opened", "workspace opened", "failed", "not started".into()),
                ),
                None => (
                    step("app-launched", "app launched", "not-verified", format!("a PowerShell window opened (pid {pid}), but `{command}` was not seen running in it")),
                    step("workspace-opened", "workspace opened", "not-verified", format!("the window was opened in {}", folder.display())),
                ),
            }
    }
}

/// An editor started through its launcher with the folder; its window title shows the folder.
fn start_editor(agent: &Agent, launcher: &Path, process: &str, folder: &Path) -> (Step, Step) {
    let name = folder
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    {
        let r = Command::new(launcher)
            .arg(folder)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .and_then(|mut c| {
                // the launcher hands the folder to the editor and exits
                poll(Duration::from_secs(20), || c.try_wait().ok().flatten())
                    .ok_or_else(|| std::io::Error::other("the launcher did not finish within 20 s"))
            });
        match r {
            Ok(s) if s.success() => {}
            Ok(s) => return not_started(format!("{} ended with {s}", launcher.display())),
            Err(e) => return not_started(format!("cannot start {}: {e}", launcher.display())),
        }
        let is_running = poll(Duration::from_secs(15), || {
            (!running(process).is_empty()).then_some(())
        })
        .is_some();
        let title = poll(Duration::from_secs(25), || window_with(process, &name));
        let launched_step = if is_running {
            step(
                "app-launched",
                "app launched",
                "verified",
                format!("{} runs ({process})", agent.name),
            )
        } else {
            step(
                "app-launched",
                "app launched",
                "not-verified",
                format!("{process} was not seen running"),
            )
        };
        let ws = match title {
            Some(t) => step(
                "workspace-opened",
                "workspace opened",
                "verified",
                format!("its window shows the folder: \"{t}\""),
            ),
            None => step(
                "workspace-opened",
                "workspace opened",
                "not-verified",
                format!("no {} window shows \"{name}\" in its title yet", agent.name),
            ),
        };
        (launched_step, ws)
    }
}

/// A packaged app started by its app id; the folder is a step for the user.
fn start_app(agent: &Agent, app_id: &str, process: &str, folder: &Path) -> (Step, Step) {
    {
        let r = Command::new("explorer.exe")
            .arg(format!("shell:AppsFolder\\{app_id}"))
            .stdin(Stdio::null())
            .spawn();
        let ws = step(
                "workspace-opened",
                "workspace opened",
                "manual",
                format!(
                    "the {} app has no supported way to be given a folder: in the app, open {} (or right-click the folder in File Explorer and pick its Codex entry)",
                    agent.name,
                    folder.display()
                ),
            );
        if let Err(e) = r {
            return (
                step(
                    "app-launched",
                    "app launched",
                    "failed",
                    format!("cannot start the app: {e}"),
                ),
                ws,
            );
        }
        let launched = poll(Duration::from_secs(20), || {
            (!running(process).is_empty()).then_some(())
        })
        .is_some();
        (
            if launched {
                step(
                    "app-launched",
                    "app launched",
                    "verified",
                    format!("the {} app runs ({process})", agent.name),
                )
            } else {
                step(
                    "app-launched",
                    "app launched",
                    "not-verified",
                    format!("{process} was not seen running"),
                )
            },
            ws,
        )
    }
}

/// Unix seconds now.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// The client's audit entries at or after `since`: (log-in seen, other call seen).
async fn seen_since(target: &CoreTarget, client: &str, since: i64) -> (bool, bool) {
    let Ok(Some(c)) = find_client(target, client).await else {
        return (false, false);
    };
    let Some(id) = c["id"].as_str() else {
        return (false, false);
    };
    let Ok(v) = call(
        target,
        methods::AUDIT_LIST,
        json!({ "client": id, "limit": 100 }),
    )
    .await
    else {
        return (false, false);
    };
    let recent: Vec<&Value> = v["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["at_unix"].as_i64().is_some_and(|t| t >= since))
        .collect();
    (
        recent.iter().any(|e| e["action"] == "auth.login"),
        recent.iter().any(|e| e["action"] != "auth.login"),
    )
}

/// Watches the audit log for up to `wait` seconds: (log-in seen, tool use seen).
async fn watch(target: &CoreTarget, client: &str, since: i64, wait: u64) -> (bool, bool) {
    let end = Instant::now() + Duration::from_secs(wait);
    let (mut login, mut used) = (false, false);
    loop {
        let (seen_login, seen_use) = seen_since(target, client, since).await;
        login |= seen_login;
        used |= seen_use;
        if login || Instant::now() >= end {
            return (login, used);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// What the user does when TUMR did not see the agent connect.
fn connect_hint(agent: &Agent) -> &'static str {
    match agent.id {
        "claude-code" => "Claude Code starts `tumr` after you accept its trust prompt for the folder (and approve the project server if it asks); `/mcp` shows it",
        "codex" => "Codex starts `tumr` when a session starts; `/mcp` (CLI) or the app's MCP settings show it",
        "cursor" => "make sure `tumr` is switched on in Cursor Settings → MCP",
        "antigravity" => "refresh or restart `tumr` under MCP Servers in Antigravity",
        "copilot" => "VS Code starts `tumr` when Chat in agent mode needs it (MCP: List Servers)",
        _ => "start the agent's MCP servers",
    }
}

/// `tumr agent open` options.
#[derive(Debug, Clone, Copy)]
pub struct OpenOptions {
    /// Seconds to watch for the MCP connection after the launch.
    pub wait: u64,
    pub json: bool,
}

/// Opens the agent in the folder and reports each step. `Err` only for bad input.
pub async fn open_steps(
    target: &CoreTarget,
    agent: &Agent,
    folder: &Path,
    opts: OpenOptions,
) -> Result<Vec<Step>, String> {
    if !folder.is_dir() {
        return Err(format!("{} is not a folder", folder.display()));
    }
    let client = agent.client();
    let Some(launch) = launcher(agent) else {
        let st = state(agent, folder);
        return Err(match st {
            State::Unsupported => format!(
                "{} was found on this PC, but TUMR has no way to start it",
                agent.name
            ),
            _ => format!("{} is not installed on this PC", agent.name),
        });
    };
    let mut steps = Vec::new();
    steps.push(match configured_in(client, folder) {
        Some(p) => step(
            "mcp-config",
            "MCP config written",
            "verified",
            format!("{} starts `tumr`", p.display()),
        ),
        None => step(
            "mcp-config",
            "MCP config written",
            "not-verified",
            format!(
                "no config of {} starts `tumr`; run `tumr agent setup {}`",
                agent.name, client.id
            ),
        ),
    });
    // an agent started from here must not keep tumr's output pipe open (a script or the desktop
    // app reading `tumr agent open --json` would wait for its end forever)
    let _ = tumr_platform::process::disinherit_std_handles();
    let since = now() - 1;
    let (launched, ws) = start(agent, &launch, folder);
    let ok = launched.status != "failed";
    steps.push(launched);
    steps.push(ws);
    if !ok {
        steps.push(step(
            "mcp-connected",
            "MCP connected",
            "not-verified",
            "the agent did not start".into(),
        ));
        steps.push(step(
            "session-ready",
            "session ready",
            "not-verified",
            "the agent did not start".into(),
        ));
        return Ok(steps);
    }
    if !opts.json {
        println!(
            "Waiting up to {} s for {} to connect to the runtime…",
            opts.wait, agent.name
        );
    }
    let (login, used) = watch(target, client.id, since, opts.wait).await;
    steps.push(if login {
        step(
            "mcp-connected",
            "MCP connected",
            "verified",
            format!("{} logged in to the runtime as '{}'", agent.name, client.id),
        )
    } else {
        step(
            "mcp-connected",
            "MCP connected",
            "not-verified",
            format!("not seen within {} s: {}", opts.wait, connect_hint(agent)),
        )
    });
    steps.push(if used {
        step(
            "session-ready",
            "session ready",
            "verified",
            "the agent already used a TUMR tool".into(),
        )
    } else {
        step(
            "session-ready",
            "session ready",
            "not-verified",
            "TUMR sees this when the agent first uses a TUMR tool (give it a task)".into(),
        )
    });
    Ok(steps)
}

fn mark(status: &str) -> &'static str {
    match status {
        "verified" => "[ok]",
        "manual" => "[you]",
        "failed" => "[failed]",
        _ => "[?]",
    }
}

/// Prints the steps (the wizard's summary uses it too).
pub fn print_steps(steps: &[Step]) {
    for s in steps {
        println!("  {:<9} {:<20} {}", mark(s.status), s.label, s.detail);
    }
    println!("  ([ok] TUMR saw it · [?] TUMR could not see it (yet) · [you] a step for you)");
}

#[must_use]
pub fn steps_json(steps: &[Step]) -> Value {
    json!(steps
        .iter()
        .map(|s| json!({ "step": s.id, "label": s.label, "status": s.status, "detail": s.detail }))
        .collect::<Vec<_>>())
}

/// `tumr agent open <agent> [folder] [--wait N] [--json]`.
pub async fn open(
    target: &CoreTarget,
    agent: &str,
    folder: Option<PathBuf>,
    o: OpenOptions,
) -> ExitCode {
    let Some(a) = find(agent) else {
        return fail(&format!("unknown agent '{agent}'; known: {}", ids()));
    };
    let folder = PathBuf::from(crate::session_cmd::absolute(
        &folder.unwrap_or_else(|| PathBuf::from(".")),
    ));
    if !o.json {
        println!("Opening {} in {}", a.name, folder.display());
    }
    match open_steps(target, a, &folder, o).await {
        Ok(steps) => {
            let launched = steps
                .iter()
                .any(|s| s.id == "app-launched" && s.status != "failed");
            if o.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "agent": a.id, "name": a.name, "folder": folder.display().to_string(),
                        "launched": launched, "steps": steps_json(&steps),
                    }))
                    .unwrap_or_default()
                );
            } else {
                print_steps(&steps);
            }
            if launched {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => fail(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_agents_each_with_a_setup_client() {
        assert_eq!(AGENTS.len(), 5);
        for a in &AGENTS {
            assert!(agent_clients::find(a.client).is_some(), "{}", a.id);
            assert_eq!(find(a.id).unwrap().id, a.id);
        }
        assert_eq!(find("vscode").unwrap().id, "copilot");
        assert_eq!(find("GitHub-Copilot").unwrap().client, "vscode");
        assert!(find("claude-desktop").is_none());
    }

    #[test]
    fn the_four_states() {
        assert_eq!(state_from(true, true, true), State::Ready);
        assert_eq!(state_from(true, false, true), State::Ready);
        assert_eq!(state_from(true, true, false), State::NotConfigured);
        assert_eq!(state_from(false, true, false), State::Unsupported);
        assert_eq!(state_from(false, true, true), State::Unsupported);
        assert_eq!(state_from(false, false, false), State::NotInstalled);
        let words: Vec<_> = [
            State::Ready,
            State::NotConfigured,
            State::NotInstalled,
            State::Unsupported,
        ]
        .iter()
        .map(|s| s.word())
        .collect();
        assert_eq!(
            words,
            ["ready", "not-configured", "not-installed", "unsupported"]
        );
    }

    #[test]
    fn which_finds_cmd_shims() {
        let dir = std::env::temp_dir().join(format!("tumr-which-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fake-agent-xyz.cmd"), "@echo off\r\n").unwrap();
        let path = std::env::join_paths([PathBuf::from(r"C:\no\such"), dir.clone()]).unwrap();
        assert_eq!(
            which_in("fake-agent-xyz", &path).unwrap(),
            dir.join("fake-agent-xyz.cmd")
        );
        assert!(which_in("no-such-agent-xyz", &path).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_refuses_a_missing_folder() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let target = CoreTarget::resolve().unwrap();
        let e = rt
            .block_on(open_steps(
                &target,
                find("cursor").unwrap(),
                Path::new(r"C:\no\such\folder\tumr"),
                OpenOptions {
                    wait: 0,
                    json: true,
                },
            ))
            .unwrap_err();
        assert!(e.contains("is not a folder"), "{e}");
    }

    #[test]
    fn steps_print_their_marks() {
        let s = [
            step("a", "x", "verified", String::new()),
            step("b", "y", "manual", String::new()),
        ];
        assert_eq!(mark(s[0].status), "[ok]");
        assert_eq!(mark(s[1].status), "[you]");
        assert_eq!(mark("not-verified"), "[?]");
        assert_eq!(steps_json(&s)[1]["status"], "manual");
    }
}
