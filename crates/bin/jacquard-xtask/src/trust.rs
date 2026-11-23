//! `check-trust-boundary`: every commit since the merge base declares its
//! oracle changes (ADR-0001).
//!
//! A commit that touches the specification carries `Trust-Boundary:
//! specification`; one that touches the verifier carries `Trust-Boundary:
//! verifier`; one that adds an escape hatch carries `Trust-Boundary:
//! escape-hatch`. A declared commit touches no implementation path. The
//! gate detects; it does not judge. A weakening inside a declared commit is
//! the reviewer's to catch, and the escape-hatch scan is syntactic.

use std::path::Path;

use serde::Deserialize;

use crate::finding::Finding;
use crate::mutants::{self, Corpus};
pub use crate::trust_patch::parse_patch;
use crate::util;

/// Every failure code this gate can print for a commit.
pub const CODES: &[&str] = &[
    "undeclared-oracle-change",
    "mixed-oracle-and-implementation",
    "escape-hatch-added",
    "trailer-without-change",
    "unknown-trust-boundary-class",
];

/// The trailer key.
pub const TRAILER: &str = "Trust-Boundary:";

/// `verification/trust-boundary.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Path sets, matched in the order specification, verifier, implementation.
    pub classes: Classes,
    /// The escape-hatch scan.
    pub escape_hatch: EscapeHatch,
}

/// Path sets. An entry ending in `/` is a directory prefix; any other entry
/// is an exact path.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classes {
    /// The specification set.
    pub specification: Vec<String>,
    /// The verifier set.
    pub verifier: Vec<String>,
    /// The implementation set, after the two above.
    pub implementation: Vec<String>,
}

/// Which added lines count as an escape hatch.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EscapeHatch {
    /// File extensions scanned, without the dot.
    pub extensions: Vec<String>,
    /// Path prefixes never scanned (fixtures hold defects on purpose).
    pub skip: Vec<String>,
    /// Substrings that mark an escape hatch on an added line.
    pub patterns: Vec<String>,
}

/// The class of a changed path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// Part of the specification set.
    Specification,
    /// Part of the verifier set.
    Verifier,
    /// Part of the implementation set.
    Implementation,
    /// Anything else: documentation, the lockfile, ignore files.
    Other,
}

impl Policy {
    /// Reads the policy.
    ///
    /// # Errors
    ///
    /// Returns a message when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, String> {
        toml::from_str(&util::read(path)?).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Classifies a repository-relative path.
    #[must_use]
    pub fn classify(&self, path: &str) -> Class {
        let hit = |set: &[String]| {
            set.iter().any(|entry| {
                if entry.ends_with('/') {
                    path.starts_with(entry.as_str())
                } else {
                    path == entry
                }
            })
        };
        if hit(&self.classes.specification) {
            Class::Specification
        } else if hit(&self.classes.verifier) {
            Class::Verifier
        } else if hit(&self.classes.implementation) {
            Class::Implementation
        } else {
            Class::Other
        }
    }

    fn is_escape(&self, path: &str, line: &str) -> bool {
        let scanned = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| self.escape_hatch.extensions.iter().any(|x| x == e));
        scanned
            && !self
                .escape_hatch
                .skip
                .iter()
                .any(|s| path.starts_with(s.as_str()))
            && self
                .escape_hatch
                .patterns
                .iter()
                .any(|p| line.contains(p.as_str()))
    }
}

/// One commit as the gate sees it.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    /// The object id, or a fixture's name.
    #[serde(default)]
    pub id: String,
    /// The full message.
    pub message: String,
    /// Repository-relative paths the commit changes.
    pub paths: Vec<String>,
    /// Added lines, as (path, line).
    #[serde(default)]
    pub added: Vec<(String, String)>,
    /// Deleted or weakened production semantic mutants in `tests/semantic-mutants/corpus.toml`.
    #[serde(default)]
    pub mutants_escape: Vec<String>,
}

/// The `Trust-Boundary:` values in a message, one per trailer line.
#[must_use]
pub fn declarations(message: &str) -> Vec<String> {
    message
        .lines()
        .filter_map(|l| l.trim().strip_prefix(TRAILER))
        .map(|v| v.trim().to_string())
        .collect()
}

/// Checks one commit, returning its sorted findings.
#[must_use]
pub fn check_commit(policy: &Policy, commit: &Commit) -> Vec<Finding> {
    let short = commit.id.get(..12).unwrap_or(&commit.id);
    let at = |detail: String| format!("{short}: {detail}");
    let mut findings = Vec::new();
    let declared = declarations(&commit.message);
    for value in &declared {
        if !["specification", "verifier", "escape-hatch"].contains(&value.as_str()) {
            findings.push(Finding::new(
                "unknown-trust-boundary-class",
                at(format!(
                    "`{TRAILER} {value}` is not specification, verifier or escape-hatch"
                )),
            ));
        }
    }
    let has = |v: &str| declared.iter().any(|d| d == v);
    let first = |class: Class| commit.paths.iter().find(|p| policy.classify(p) == class);
    for (class, name) in [
        (Class::Specification, "specification"),
        (Class::Verifier, "verifier"),
    ] {
        match (first(class), has(name)) {
            (Some(path), false) => findings.push(Finding::new(
                "undeclared-oracle-change",
                at(format!(
                    "{path} is {name} but the message has no `{TRAILER} {name}`"
                )),
            )),
            (None, true) => findings.push(Finding::new(
                "trailer-without-change",
                at(format!("declares {name} but changes no {name} path")),
            )),
            _ => {}
        }
    }
    let escape = commit
        .added
        .iter()
        .find(|(path, line)| policy.is_escape(path, line))
        .map(|(path, line)| format!("{path} adds `{}`", line.trim()))
        .or_else(|| {
            commit
                .mutants_escape
                .first()
                .map(|reason| format!("{} {reason}", mutants::PRODUCTION_CORPUS))
        });
    match (escape, has("escape-hatch")) {
        (Some(detail), false) => findings.push(Finding::new(
            "escape-hatch-added",
            at(format!("{detail} without `{TRAILER} escape-hatch`")),
        )),
        (None, true) => findings.push(Finding::new(
            "trailer-without-change",
            at("declares escape-hatch but adds none".to_string()),
        )),
        _ => {}
    }
    let declares_any = ["specification", "verifier", "escape-hatch"]
        .iter()
        .any(|v| has(v));
    if let Some(path) = first(Class::Implementation).filter(|_| declares_any) {
        findings.push(Finding::new(
            "mixed-oracle-and-implementation",
            at(format!(
                "a declared oracle commit also changes implementation path {path}"
            )),
        ));
    }
    findings.sort();
    findings
}

/// Reads every commit (including merge commits via combined diff) from the
/// merge base of `base` and `HEAD` to `HEAD`, oldest first.
///
/// # Errors
///
/// Returns a message when the base cannot be resolved or git fails. A base
/// that cannot be resolved is a failure, never an empty range.
pub fn collect(repo: &Path, base: &str) -> Result<Vec<Commit>, String> {
    let git = |args: &[&str]| util::run("git", args, repo);
    let spec = format!("{base}^{{commit}}");
    git(&["rev-parse", "--verify", "--quiet", &spec])
        .map_err(|_| format!("[unresolved-base] `{base}` does not name a commit"))?;
    let merge_base = git(&["merge-base", base, "HEAD"])?.trim().to_string();
    let range = format!("{merge_base}..HEAD");
    let ids = git(&["rev-list", "--reverse", &range])?;
    let mut commits = Vec::new();
    for id in ids.split_whitespace() {
        let message = git(&["log", "-1", "--format=%B", id])?;
        let patch = git(&[
            "diff-tree",
            "--cc",
            "-r",
            "-p",
            "--root",
            "--unified=0",
            "--no-commit-id",
            "--no-renames",
            id,
        ])?;
        let (paths, added) = parse_patch(&patch);
        let mutants_escape = if paths.iter().any(|p| p == mutants::PRODUCTION_CORPUS) {
            check_mutants_corpus_commit(&git, id)?
        } else {
            Vec::new()
        };
        commits.push(Commit {
            id: id.to_string(),
            message,
            paths,
            added,
            mutants_escape,
        });
    }
    Ok(commits)
}

fn check_mutants_corpus_commit(
    git: &impl Fn(&[&str]) -> Result<String, String>,
    id: &str,
) -> Result<Vec<String>, String> {
    let parents_line = git(&["rev-list", "--parents", "-n", "1", id])?;
    let parents: Vec<&str> = parents_line.split_whitespace().skip(1).collect();
    if parents.is_empty() {
        return Ok(Vec::new());
    }
    let after_spec = format!("{id}:{}", mutants::PRODUCTION_CORPUS);
    let Ok(after_raw) = git(&["show", &after_spec]) else {
        return Ok(vec!["deleted production semantic-mutant corpus".into()]);
    };
    let after = match Corpus::parse(&after_raw, &after_spec) {
        Ok(c) => c,
        Err(e) => return Ok(vec![format!("invalid or empty corpus ({e})")]),
    };
    let mut violations = Vec::new();
    for parent in parents {
        let before_spec = format!("{parent}:{}", mutants::PRODUCTION_CORPUS);
        if let Ok(before_raw) = git(&["show", &before_spec])
            && let Ok(before) = Corpus::parse(&before_raw, &before_spec)
        {
            violations.extend(mutants::diff_corpora(&before, &after));
        }
    }
    Ok(violations)
}

/// Runs the gate on a repository against a base.
///
/// # Errors
///
/// Returns a message when the policy cannot be read, the base cannot be
/// resolved, or git fails.
pub fn run(root: &Path, base: &str) -> Result<Vec<Finding>, String> {
    let policy = Policy::load(&root.join("verification/trust-boundary.toml"))?;
    let mut findings = Vec::new();
    for commit in collect(root, base)? {
        findings.extend(check_commit(&policy, &commit));
    }
    Ok(findings)
}
