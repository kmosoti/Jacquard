//! Receipts: one NDJSON line per run of a registered check, written by
//! `receipts record` and never by hand. A receipt proves the record's
//! structure and that the recorder ran the command; it is not signed.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::finding::Finding;
use crate::registry::Registry;
use crate::util;

/// Every failure code `receipts validate` can print.
pub const CODES: &[&str] = &[
    "receipt-malformed",
    "receipt-unregistered-check",
    "receipt-command-mismatch",
    "receipt-passed-without-evidence",
    "receipt-unknown-outcome",
];

/// The outcomes a receipt may carry. Only `passed` supports the word.
pub const OUTCOMES: &[&str] = &[
    "passed",
    "failed",
    "not-run",
    "interrupted",
    "inconclusive",
    "environment-unavailable",
];

/// One receipt.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// Format version, 1.
    pub version: u32,
    /// The registered check id.
    pub check_id: String,
    /// The command as run, words separated by single spaces.
    pub command: String,
    /// `git rev-parse HEAD` when the run started.
    pub revision: String,
    /// Whether tracked files outside `verification/receipts/` differed from the revision.
    pub dirty: bool,
    /// `rustc -V` when the run started.
    pub toolchain: String,
    /// One of [`OUTCOMES`].
    pub outcome: String,
    /// The process exit code, absent when it did not run or was killed.
    pub exit_code: Option<i32>,
    /// SHA-256 of standard output, a NUL byte, and standard error.
    pub output_sha256: Option<String>,
    /// Seconds since the Unix epoch when the run started.
    pub started_unix: u64,
    /// Wall time of the run.
    pub duration_ms: u64,
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Validates one receipt line.
#[must_use]
pub fn validate_line(registry: &Registry, line: &str) -> Vec<Finding> {
    let receipt: Receipt = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => return vec![Finding::new("receipt-malformed", e.to_string())],
    };
    let mut findings = Vec::new();
    if receipt.version != 1 || !is_hex(&receipt.revision, 40) {
        findings.push(Finding::new(
            "receipt-malformed",
            format!(
                "version {} revision `{}`",
                receipt.version, receipt.revision
            ),
        ));
    }
    if receipt
        .output_sha256
        .as_deref()
        .is_some_and(|h| !is_hex(h, 64))
    {
        findings.push(Finding::new(
            "receipt-malformed",
            "output_sha256 is not 64 hex digits",
        ));
    }
    if let Err(code) = registry.admits(&receipt.check_id, &receipt.command) {
        findings.push(Finding::new(
            code,
            format!("{}: `{}`", receipt.check_id, receipt.command),
        ));
    }
    if !OUTCOMES.contains(&receipt.outcome.as_str()) {
        findings.push(Finding::new(
            "receipt-unknown-outcome",
            format!("`{}`", receipt.outcome),
        ));
    }
    if receipt.outcome == "passed"
        && (receipt.exit_code != Some(0) || receipt.output_sha256.is_none())
    {
        findings.push(Finding::new(
            "receipt-passed-without-evidence",
            format!(
                "{} says passed with exit {:?} and output hash {:?}",
                receipt.check_id, receipt.exit_code, receipt.output_sha256
            ),
        ));
    }
    findings
}

/// Validates every line of a receipts file; details carry `name:line`.
#[must_use]
pub fn validate_text(registry: &Registry, name: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        for mut finding in validate_line(registry, line) {
            finding.detail = format!("{name}:{}: {}", index + 1, finding.detail);
            findings.push(finding);
        }
    }
    findings
}

/// Validates every `*.ndjson` file in a directory. A missing directory holds
/// no receipts.
///
/// # Errors
///
/// Returns a message when the registry or a file cannot be read.
pub fn validate_dir(root: &Path, dir: &Path) -> Result<Vec<Finding>, String> {
    let registry = Registry::load(&root.join("verification/checks.toml"))?;
    let mut findings = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(findings);
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ndjson"))
        .collect();
    files.sort();
    for file in files {
        let name = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .display()
            .to_string();
        findings.extend(validate_text(&registry, &name, &util::read(&file)?));
    }
    Ok(findings)
}

fn hash(stdout: &[u8], stderr: &[u8]) -> String {
    let digest = Sha256::new()
        .chain_update(stdout)
        .chain_update([0u8])
        .chain_update(stderr)
        .finalize();
    digest.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Appends one receipt line to a file, creating it and its directory.
///
/// # Errors
///
/// Returns a message when the file cannot be written.
pub fn append(out: &Path, receipt: &Receipt) -> Result<(), String> {
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let line = serde_json::to_string(receipt).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out)
        .map_err(|e| format!("{}: {e}", out.display()))?;
    writeln!(file, "{line}").map_err(|e| format!("{}: {e}", out.display()))
}

/// Runs a registered check's command in the repository, echoes its output,
/// and returns its receipt. A command that is not a use of the registered
/// check is refused before it runs. `argv` empty records `not-run`.
///
/// # Errors
///
/// Returns `receipt-unregistered-check` or `receipt-command-mismatch` with
/// the command, or a message when git or rustc cannot be read.
pub fn run_check(
    root: &Path,
    registry: &Registry,
    id: &str,
    argv: &[String],
) -> Result<Receipt, String> {
    let command = argv.join(" ");
    let revision = util::run("git", &["rev-parse", "HEAD"], root)?
        .trim()
        .to_string();
    let status = util::run(
        "git",
        &[
            "status",
            "--porcelain",
            "--untracked-files=no",
            "--",
            ".",
            ":(exclude)verification/receipts",
        ],
        root,
    )?;
    let toolchain = util::run("rustc", &["-V"], root)?.trim().to_string();
    let mut receipt = Receipt {
        version: 1,
        check_id: id.to_string(),
        command: command.clone(),
        revision,
        dirty: !status.trim().is_empty(),
        toolchain,
        outcome: "not-run".to_string(),
        exit_code: None,
        output_sha256: None,
        started_unix: now(),
        duration_ms: 0,
    };
    if argv.is_empty() {
        return Ok(receipt);
    }
    registry
        .admits(id, &command)
        .map_err(|code| format!("[{code}] {id}: `{command}`"))?;
    let started = Instant::now();
    match Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(root)
        .output()
    {
        Ok(output) => {
            std::io::stdout().write_all(&output.stdout).ok();
            std::io::stderr().write_all(&output.stderr).ok();
            receipt.exit_code = output.status.code();
            receipt.outcome = match output.status.code() {
                Some(0) => "passed",
                Some(_) => "failed",
                None => "interrupted",
            }
            .to_string();
            receipt.output_sha256 = Some(hash(&output.stdout, &output.stderr));
        }
        Err(e) => {
            eprintln!("{}: {e}", argv[0]);
            receipt.outcome = "environment-unavailable".to_string();
        }
    }
    receipt.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Ok(receipt)
}
