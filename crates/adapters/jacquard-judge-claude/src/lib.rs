//! The Claude Judge (ADR-0004): Claude Sonnet 5.5 through the Claude CLI in
//! print mode, pinned by identifier, with the rubric as the whole system
//! prompt, no tools, no MCP servers, project settings only, no session
//! persistence, run from an empty directory so no CLAUDE.md or memory is
//! found. Images travel as base64 content blocks. The CLI's own report of
//! its tools, its MCP servers and the answering model is checked on every
//! call: a context that is not isolated is a failure, and the answering model
//! is returned for admission to compare against the pinned one.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use base64::Engine as _;
use jacquard_core::judgment::Verdict;
use jacquard_ports::{Answer, Failure, Judge, Render};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The model every Judgment is pinned to. Never an alias: on 2026-09-28 the
/// alias `sonnet` answered as `claude-sonnet-5`.
pub const MODEL: &str = "claude-sonnet-5-5";

/// The rubric, sent as the whole system prompt.
pub const RUBRIC: &str = include_str!("rubric.md");

/// The rubric's dimensions, all of which an answer must carry.
pub const DIMENSIONS: [&str; 5] = [
    "legibility",
    "integrity",
    "hierarchy",
    "consistency",
    "usability",
];

/// How long one Judgment may take before the CLI is killed.
pub const DEADLINE: Duration = Duration::from_secs(300);

/// The Judge.
#[derive(Clone, Debug)]
pub struct ClaudeJudge {
    binary: PathBuf,
    scratch: PathBuf,
}

impl ClaudeJudge {
    /// A Judge using `JACQUARD_CLAUDE`, else `claude` on the path, working
    /// in fresh empty directories under `scratch`.
    #[must_use]
    pub fn from_env(scratch: &Path) -> Self {
        let binary = std::env::var_os("JACQUARD_CLAUDE")
            .map_or_else(|| PathBuf::from("claude"), PathBuf::from);
        Self {
            binary,
            scratch: scratch.to_path_buf(),
        }
    }

    /// The CLI arguments, in order.
    #[must_use]
    pub fn arguments() -> Vec<String> {
        [
            "-p",
            "--model",
            MODEL,
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--tools",
            "",
            "--setting-sources",
            "project",
            "--strict-mcp-config",
            "--mcp-config",
            r#"{"mcpServers":{}}"#,
            "--no-session-persistence",
            "--system-prompt",
            RUBRIC,
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }
}

fn image(render: &Render) -> Value {
    json!({
        "type": "image",
        "source": {
            "type": "base64",
            "media_type": "image/png",
            "data": base64::engine::general_purpose::STANDARD.encode(&render.png),
        }
    })
}

/// The one stream-JSON user message: A's screenshots, B's, and the request.
#[must_use]
pub fn request(first: &[Render], second: &[Render]) -> String {
    let mut content = vec![json!({"type": "text", "text": "Render A:"})];
    for r in first {
        content.push(json!({"type": "text", "text": format!("A at {}:", r.viewport)}));
        content.push(image(r));
    }
    content.push(json!({"type": "text", "text": "Render B:"}));
    for r in second {
        content.push(json!({"type": "text", "text": format!("B at {}:", r.viewport)}));
        content.push(image(r));
    }
    content.push(json!({"type": "text", "text": "Compare A and B under the rubric. Answer with the JSON object only."}));
    json!({"type": "user", "message": {"role": "user", "content": content}}).to_string()
}

fn verdict(value: &Value) -> Option<Verdict> {
    match value.as_str()?.trim().to_ascii_lowercase().as_str() {
        "a" => Some(Verdict::First),
        "b" => Some(Verdict::Second),
        "tie" => Some(Verdict::Tie),
        _ => None,
    }
}

/// Reads the CLI's stream-JSON output into an Answer.
///
/// # Errors
///
/// Fails when the context was not isolated (any tool or MCP server
/// attached), when the CLI reported an error, or when the answer is not a
/// complete rubric verdict.
pub fn parse(output: &str) -> Result<Answer, Failure> {
    let mut isolated = None;
    let mut result = None;
    for line in output.lines().filter(|l| l.starts_with('{')) {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match event["type"].as_str() {
            Some("system") if event["subtype"] == "init" => {
                let empty = |key: &str| event[key].as_array().is_some_and(Vec::is_empty);
                isolated = Some(empty("tools") && empty("mcp_servers"));
            }
            Some("result") => result = Some(event),
            _ => {}
        }
    }
    match isolated {
        Some(true) => {}
        Some(false) => {
            return Err(Failure::Failed(
                "the Judge's context was not isolated: tools or MCP servers attached".into(),
            ));
        }
        None => return Err(Failure::Failed("the CLI reported no session".into())),
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
    let text = result["result"].as_str().unwrap_or_default();
    let object = text
        .find('{')
        .zip(text.rfind('}'))
        .and_then(|(a, b)| serde_json::from_str::<Value>(&text[a..=b]).ok())
        .ok_or_else(|| Failure::Failed(format!("no JSON verdict in: {text}")))?;
    let mut dimensions = BTreeMap::new();
    for name in DIMENSIONS {
        let v = verdict(&object["dimensions"][name]).ok_or_else(|| {
            Failure::Failed(format!("dimension `{name}` missing or not A, B or tie"))
        })?;
        dimensions.insert(name.to_string(), v);
    }
    Ok(Answer {
        verdict: verdict(&object["verdict"])
            .ok_or_else(|| Failure::Failed("verdict missing or not A, B or tie".into()))?,
        dimensions,
        reason: object["reason"].as_str().unwrap_or_default().to_string(),
        model: models.join("+"),
    })
}

impl Judge for ClaudeJudge {
    fn pinned_model(&self) -> &str {
        MODEL
    }

    fn rubric_hash(&self) -> String {
        use std::fmt::Write as _;
        let digest = Sha256::digest(RUBRIC.as_bytes());
        digest.iter().take(8).fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    fn judge(&self, first: &[Render], second: &[Render]) -> Result<Answer, Failure> {
        static CALL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = CALL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = self
            .scratch
            .join(format!("judge-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| Failure::Failed(e.to_string()))?;
        let out_path = dir.with_extension("out");
        let out = std::fs::File::create(&out_path).map_err(|e| Failure::Failed(e.to_string()))?;
        let mut child = Command::new(&self.binary)
            .args(Self::arguments())
            .current_dir(&dir)
            .stdin(Stdio::piped())
            .stdout(out)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Failure::Unavailable(format!("{}: {e}", self.binary.display())))?;
        let message = request(first, second);
        if let Some(mut stdin) = child.stdin.take() {
            writeln!(stdin, "{message}").map_err(|e| Failure::Failed(e.to_string()))?;
        }
        let deadline = Instant::now() + DEADLINE;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(200));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Failure::TimedOut(format!(
                        "no Judgment within {}s",
                        DEADLINE.as_secs()
                    )));
                }
                Err(e) => return Err(Failure::Failed(e.to_string())),
            }
        }
        let output = std::fs::read_to_string(&out_path).unwrap_or_default();
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_dir_all(&dir);
        parse(&output)
    }
}
