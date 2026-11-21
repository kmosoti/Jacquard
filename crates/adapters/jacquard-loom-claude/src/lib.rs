//! The Claude Loom (ADR-0008): Claude Sonnet 5.5, pinned as
//! `claude-sonnet-5-5`, as a coding agent over a copy of a Product. The CLI
//! runs in `--restricted` mode, which removes every tool that runs commands
//! or code and confines file access to the working directory, with exactly
//! Read, Glob, Grep, Edit and Write, no MCP servers, and edits accepted
//! without prompting. The session's own report of its tools is checked:
//! any other tool attached is a failure. The copy is a fresh Git repository
//! so the Weave is exactly `git diff` of what the Loom changed.
#![forbid(unsafe_code)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use jacquard_ports::{Failure, Loom, Weave};
use serde_json::{Value, json};

/// The model every Weave is pinned to.
pub const MODEL: &str = "claude-sonnet-5-5";

/// The tools the Loom has, and the only ones it may have.
pub const TOOLS: [&str; 5] = ["Read", "Glob", "Grep", "Edit", "Write"];

/// The Loom's standing rules, appended to the CLI's own system prompt.
pub const RULES: &str = include_str!("rules.md");

/// How long one Weave may take before the CLI is killed.
pub const DEADLINE: Duration = Duration::from_mins(30);

/// The Loom.
#[derive(Clone, Debug)]
pub struct ClaudeLoom {
    binary: PathBuf,
}

impl ClaudeLoom {
    /// A Loom using `JACQUARD_CLAUDE`, else `claude` on the path.
    #[must_use]
    pub fn from_env() -> Self {
        let binary = std::env::var_os("JACQUARD_CLAUDE")
            .map_or_else(|| PathBuf::from("claude"), PathBuf::from);
        Self { binary }
    }

    /// The CLI arguments, in order.
    #[must_use]
    pub fn arguments() -> Vec<String> {
        let mut args: Vec<String> = [
            "-p",
            "--model",
            MODEL,
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--restricted",
            "--permission-mode",
            "acceptEdits",
            "--strict-mcp-config",
            "--mcp-config",
            r#"{"mcpServers":{}}"#,
            "--no-session-persistence",
            "--append-system-prompt",
            RULES,
            "--tools",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        args.extend(TOOLS.iter().map(|t| (*t).to_string()));
        args
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String, Failure> {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=jacquard-loom",
            "-c",
            "user.email=loom@jacquard.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| Failure::Unavailable(format!("git: {e}")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(Failure::Failed(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// Copies the files `git ls-files` lists under `base` (tracked, and
/// untracked but not ignored) into `out`, and commits them there.
///
/// # Errors
///
/// Fails when a file cannot be listed or copied.
pub fn prepare(base: &Path, out: &Path) -> Result<(), Failure> {
    let listed = git(
        base,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            ".",
        ],
    )?;
    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out).map_err(|e| Failure::Failed(e.to_string()))?;
    for rel in listed.split('\0').filter(|s| !s.is_empty()) {
        let from = base.join(rel);
        if !from.is_file() {
            continue;
        }
        let to = out.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Failure::Failed(e.to_string()))?;
        }
        std::fs::copy(&from, &to)
            .map_err(|e| Failure::Failed(format!("{}: {e}", from.display())))?;
    }
    git(out, &["init", "-q", "-b", "base"])?;
    git(out, &["add", "-A"])?;
    git(out, &["commit", "-q", "-m", "base"])?;
    Ok(())
}

/// Reads the CLI's stream-JSON into the model that answered and its final
/// text.
///
/// # Errors
///
/// Fails when the session had any tool beyond [`TOOLS`], any MCP server, or
/// ended in an error.
pub fn parse(output: &str) -> Result<(String, String), Failure> {
    let mut checked = false;
    let mut result = None;
    for line in output.lines().filter(|l| l.starts_with('{')) {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match event["type"].as_str() {
            Some("system") if event["subtype"] == "init" => {
                let mut tools: Vec<&str> = event["tools"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                tools.sort_unstable();
                let mut allowed = TOOLS.to_vec();
                allowed.sort_unstable();
                if tools != allowed {
                    return Err(Failure::Failed(format!(
                        "the Loom's session had tools {tools:?}, not {allowed:?}"
                    )));
                }
                if event["mcp_servers"]
                    .as_array()
                    .is_some_and(|m| !m.is_empty())
                {
                    return Err(Failure::Failed(
                        "the Loom's session had MCP servers attached".into(),
                    ));
                }
                checked = true;
            }
            Some("result") => result = Some(event),
            _ => {}
        }
    }
    if !checked {
        return Err(Failure::Failed("the CLI reported no session".into()));
    }
    let result = result.ok_or_else(|| Failure::Failed("the CLI reported no result".into()))?;
    if result["is_error"].as_bool() != Some(false) {
        return Err(Failure::Failed(format!(
            "the CLI reported an error: {}",
            result["result"]
        )));
    }
    let models: Vec<String> = result["modelUsage"]
        .as_object()
        .map_or_else(Vec::new, |m| m.keys().cloned().collect());
    Ok((
        models.join("+"),
        result["result"].as_str().unwrap_or_default().to_string(),
    ))
}

impl Loom for ClaudeLoom {
    fn weave(&self, draft: &str, base: &Path, out: &Path) -> Result<Weave, Failure> {
        prepare(base, out)?;
        let log = out.with_extension("stream.jsonl");
        let file = std::fs::File::create(&log).map_err(|e| Failure::Failed(e.to_string()))?;
        let mut child = Command::new(&self.binary)
            .args(Self::arguments())
            .current_dir(out)
            .stdin(Stdio::piped())
            .stdout(file)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Failure::Unavailable(format!("{}: {e}", self.binary.display())))?;
        let message = json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": draft}]}});
        if let Some(mut stdin) = child.stdin.take() {
            writeln!(stdin, "{message}").map_err(|e| Failure::Failed(e.to_string()))?;
        }
        let deadline = Instant::now() + DEADLINE;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(500));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Failure::TimedOut(format!(
                        "no Weave within {}s",
                        DEADLINE.as_secs()
                    )));
                }
                Err(e) => return Err(Failure::Failed(e.to_string())),
            }
        }
        let (model, summary) = parse(&std::fs::read_to_string(&log).unwrap_or_default())?;
        git(out, &["add", "-A"])?;
        let patch = git(out, &["diff", "--cached", "--binary"])?;
        let changed: Vec<String> = git(out, &["diff", "--cached", "--name-only"])?
            .lines()
            .map(str::to_string)
            .collect();
        if changed.is_empty() {
            return Err(Failure::Failed(format!(
                "the Loom changed nothing: {summary}"
            )));
        }
        Ok(Weave {
            changed,
            patch,
            model,
            summary,
        })
    }
}
