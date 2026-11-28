//! Runs one creation tool: no console window, no input, TUMR's environment, its output passed on
//! line by line, the whole process tree in a kill-on-close Job Object (Cancel and the time limit
//! end every process the tool started, such as the `node` behind `npx.cmd`).

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::Toolset;

/// How long one creation tool may run (npx may download its package first).
pub const TIME_LIMIT: Duration = Duration::from_secs(15 * 60);

/// Lines of output kept for the error message.
const TAIL: usize = 25;

/// Runs `exe args` in `cwd`; `Err` names the tool, its exit code and the last lines it printed.
pub fn run(
    what: &str,
    exe: &Path,
    args: &[&str],
    cwd: &Path,
    tools: &Toolset,
    out: &mut dyn FnMut(&str),
    cancel: &AtomicBool,
) -> Result<(), String> {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .current_dir(cwd)
        .envs(&tools.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let job = tumr_platform::job::KillOnCloseJob::new()
        .map_err(|e| format!("cannot prepare {what}: {e}"))?;
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start {what} ({}): {e}", exe.display()))?;
    if let Err(e) = job.assign(&child) {
        let _ = child.kill();
        return Err(format!("cannot watch {what}: {e}"));
    }

    let (rx, readers) = read_lines(&mut child);

    let started = Instant::now();
    let mut tail: Vec<String> = Vec::new();
    let mut keep = |line: String, out: &mut dyn FnMut(&str)| {
        out(&line);
        tail.push(line);
        if tail.len() > TAIL {
            tail.remove(0);
        }
    };
    let status = loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => keep(line, out),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
        if cancel.load(Ordering::SeqCst) {
            let _ = job.terminate(1);
            let _ = child.wait();
            let _ = job.wait_empty(Duration::from_secs(10));
            return Err(format!("cancelled while {what} was running"));
        }
        if started.elapsed() > TIME_LIMIT {
            let _ = job.terminate(1);
            let _ = child.wait();
            let _ = job.wait_empty(Duration::from_secs(10));
            return Err(format!(
                "{what} did not finish within {} minutes",
                TIME_LIMIT.as_secs() / 60
            ));
        }
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {}
            Err(e) => return Err(format!("{what}: {e}")),
        }
    };
    // the tool ended: end anything it left behind, then read the rest of its output
    let _ = job.terminate(0);
    let _ = job.wait_empty(Duration::from_secs(10));
    for r in readers {
        let _ = r.join();
    }
    while let Ok(line) = rx.try_recv() {
        keep(line, out);
    }
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{what} failed (exit code {}). Last output:\n{}",
            status
                .code()
                .map_or_else(|| "none".into(), |c| c.to_string()),
            tail.join("\n")
        ))
    }
}

/// Reads the child's output and error streams line by line, cleaned, into one channel.
fn read_lines(
    child: &mut std::process::Child,
) -> (mpsc::Receiver<String>, Vec<std::thread::JoinHandle<()>>) {
    let (tx, rx) = mpsc::channel::<String>();
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|stream| {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(stream);
            let mut buf = Vec::new();
            while r.read_until(b'\n', &mut buf).unwrap_or(0) > 0 {
                let line = clean(&String::from_utf8_lossy(&buf));
                if !line.is_empty() && tx.send(line).is_err() {
                    break;
                }
                buf.clear();
            }
        })
    })
    .collect();
    drop(tx);
    (rx, readers)
}

/// A line without colour codes, spinner carriage returns and trailing space.
fn clean(raw: &str) -> String {
    let last = raw
        .trim_end_matches(['\r', '\n'])
        .rsplit('\r')
        .next()
        .unwrap_or("");
    let mut s = String::with_capacity(last.len());
    let mut chars = last.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI: ESC [ ... final byte in @..~
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            continue;
        }
        s.push(c);
    }
    s.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_strips_colours_and_spinners() {
        assert_eq!(
            clean("\u{1b}[32m✔\u{1b}[39m Copying template\r\n"),
            "✔ Copying template"
        );
        assert_eq!(clean("- Copying\r✔ Copied\n"), "✔ Copied");
        assert_eq!(clean("   \n"), "");
    }

    fn cmd() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".into()))
    }

    #[test]
    fn output_is_passed_on_and_failure_names_the_tool() {
        let mut lines = Vec::new();
        let dir = std::env::temp_dir();
        run(
            "echo",
            &cmd(),
            &["/c", "echo one&& echo two"],
            &dir,
            &Toolset::default(),
            &mut |l| lines.push(l.to_string()),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(lines, ["one", "two"]);
        let e = run(
            "the test tool",
            &cmd(),
            &["/c", "echo boom&& exit 3"],
            &dir,
            &Toolset::default(),
            &mut |_| {},
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(
            e.contains("the test tool failed (exit code 3)") && e.contains("boom"),
            "{e}"
        );
    }

    #[test]
    fn cancel_ends_the_tool() {
        let cancel = AtomicBool::new(true);
        let start = Instant::now();
        let e = run(
            "ping",
            &cmd(),
            &["/c", "ping -n 30 127.0.0.1"],
            &std::env::temp_dir(),
            &Toolset::default(),
            &mut |_| {},
            &cancel,
        )
        .unwrap_err();
        assert!(e.contains("cancelled"));
        assert!(start.elapsed() < Duration::from_secs(10));
    }
}
