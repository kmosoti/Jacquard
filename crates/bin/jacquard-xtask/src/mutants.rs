//! `mutants semantic` (ADR-0007): for each entry of the corpus, apply one
//! wrong behavior to a copy of the tree and require that the entry's named
//! test, run alone, fails against it. Each named test must first pass on the
//! unmutated copy, or it proves nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::finding::Finding;
use crate::util;

/// Every failure code this gate can print.
pub const CODES: &[&str] = &[
    "mutant-survived",
    "mutant-not-applicable",
    "mutant-test-missing",
    "mutant-baseline-failed",
    "mutant-unviable",
];

/// `tests/semantic-mutants/corpus.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    /// The entries.
    pub mutant: Vec<Mutant>,
    /// For a fixture corpus only: the result each entry must produce.
    #[serde(default)]
    pub expected: BTreeMap<String, String>,
}

/// One wrong behavior and its named defender.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutant {
    /// A stable id.
    pub id: String,
    /// The invariant the wrong behavior breaks.
    pub invariant: String,
    /// The file to patch, relative to the root.
    pub file: String,
    /// Text that must occur exactly once in the file.
    pub find: String,
    /// Its replacement.
    pub replace: String,
    /// The package of the defending test.
    pub package: String,
    /// The test target (the file name under `tests/`, without `.rs`).
    pub test: String,
    /// The test function's name.
    pub name: String,
}

impl Corpus {
    /// Reads a corpus.
    ///
    /// # Errors
    ///
    /// Returns a message when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, String> {
        toml::from_str(&util::read(path)?).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// Applies a patch whose text must occur exactly once.
///
/// # Errors
///
/// Returns how many times the text occurs when that is not one.
pub fn apply(source: &str, find: &str, replace: &str) -> Result<String, usize> {
    match source.matches(find).count() {
        1 => Ok(source.replacen(find, replace, 1)),
        n => Err(n),
    }
}

/// What running one named test reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    /// The test ran and passed.
    Passed,
    /// The test ran and failed.
    Failed,
    /// The named test did not run.
    Missing,
    /// The code did not compile.
    Unviable,
}

/// Classifies a `cargo test -- --exact NAME` run from its output and status.
#[must_use]
pub fn classify(name: &str, success: bool, output: &str) -> Run {
    let ok = format!("test {name} ... ok");
    let failed = format!("test {name} ... FAILED");
    if output.contains(&failed) {
        Run::Failed
    } else if output.contains(&ok) {
        if success { Run::Passed } else { Run::Failed }
    } else if output.contains("error[E") || output.contains("could not compile") {
        Run::Unviable
    } else {
        Run::Missing
    }
}

fn scratch(root: &Path) -> PathBuf {
    let mut h = DefaultHasher::new();
    root.hash(&mut h);
    std::env::temp_dir().join(format!("jacquard-mutants-{:016x}", h.finish()))
}

fn copy_tree(root: &Path, tree: &Path) -> Result<(), String> {
    let listed = util::run(
        "git",
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        root,
    )?;
    let _ = std::fs::remove_dir_all(tree);
    for rel in listed.split('\0').filter(|s| !s.is_empty()) {
        let from = root.join(rel);
        if !from.is_file() {
            continue;
        }
        let to = tree.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::copy(&from, &to).map_err(|e| format!("{}: {e}", from.display()))?;
    }
    Ok(())
}

fn run_test(tree: &Path, target: &Path, m: &Mutant) -> Result<Run, String> {
    let output = Command::new("cargo")
        .args([
            "test", "-p", &m.package, "--test", &m.test, "--", "--exact", &m.name,
        ])
        .current_dir(tree)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .map_err(|e| format!("cargo: {e}"))?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(classify(&m.name, output.status.success(), &text))
}

/// Runs the corpus against a copy of the tree at `root`. `progress` is
/// called with each mutant's id and result.
///
/// # Errors
///
/// Returns a message when the corpus cannot be read or the tree copied.
pub fn run(
    root: &Path,
    corpus: &Path,
    mut progress: impl FnMut(&str, &str),
) -> Result<Vec<Finding>, String> {
    let corpus = Corpus::load(corpus)?;
    let base = scratch(root);
    let (tree, target) = (base.join("tree"), base.join("target"));
    copy_tree(root, &tree)?;
    let mut findings = Vec::new();
    let mut baseline: BTreeMap<(String, String, String), Run> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for m in &corpus.mutant {
        if !seen.insert(m.id.clone()) {
            return Err(format!("duplicate mutant id {}", m.id));
        }
        let key = (m.package.clone(), m.test.clone(), m.name.clone());
        let base_run = if let Some(r) = baseline.get(&key) {
            *r
        } else {
            let r = run_test(&tree, &target, m)?;
            baseline.insert(key, r);
            r
        };
        let path = tree.join(&m.file);
        let original = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", m.file))?;
        let at = |detail: String| format!("{} ({}): {detail}", m.id, m.invariant);
        let result = match base_run {
            Run::Passed => match apply(&original, &m.find, &m.replace) {
                Err(count) => Err(Finding::new(
                    "mutant-not-applicable",
                    at(format!(
                        "the text occurs {count} times in {}, not once",
                        m.file
                    )),
                )),
                Ok(mutated) => {
                    std::fs::write(&path, mutated).map_err(|e| e.to_string())?;
                    let outcome = run_test(&tree, &target, m);
                    std::fs::write(&path, &original).map_err(|e| e.to_string())?;
                    match outcome? {
                        Run::Failed => Ok(()),
                        Run::Passed => Err(Finding::new(
                            "mutant-survived",
                            at(format!(
                                "{}::{} passed against the wrong behavior",
                                m.test, m.name
                            )),
                        )),
                        Run::Missing => Err(Finding::new(
                            "mutant-test-missing",
                            at(format!("{} did not run", m.name)),
                        )),
                        Run::Unviable => Err(Finding::new(
                            "mutant-unviable",
                            at("the mutant does not compile".into()),
                        )),
                    }
                }
            },
            Run::Missing => Err(Finding::new(
                "mutant-test-missing",
                at(format!("{} did not run", m.name)),
            )),
            Run::Failed | Run::Unviable => Err(Finding::new(
                "mutant-baseline-failed",
                at(format!("{} does not pass without the mutant", m.name)),
            )),
        };
        match result {
            Ok(()) => progress(&m.id, "caught"),
            Err(finding) => {
                progress(&m.id, finding.code);
                findings.push(finding);
            }
        }
    }
    Ok(findings)
}
