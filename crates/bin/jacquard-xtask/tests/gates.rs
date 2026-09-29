//! Every gate against its fixtures under `tests/fixtures/<gate>/`: each case
//! must print exactly its expected codes, every code a gate can print must
//! have a case, and the real repository must pass every gate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use jacquard_xtask::finding::Finding;
use jacquard_xtask::registry::Registry;
use jacquard_xtask::{layers, purity, receipts, repo_root, trust};
use serde::Deserialize;

fn fixtures(gate: &str) -> PathBuf {
    repo_root().join("tests/fixtures").join(gate)
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("directory entry").path())
        .collect();
    out.sort();
    out
}

fn codes(findings: &[Finding]) -> BTreeSet<String> {
    findings.iter().map(|f| f.code.to_string()).collect()
}

fn expected(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && *l != "ok")
        .map(str::to_string)
        .collect()
}

/// Runs one workspace-shaped gate over its fixture directories and checks
/// every case and the code coverage.
fn workspace_gate(gate: &str, all: &[&str], run: fn(&Path) -> Result<Vec<Finding>, String>) {
    let mut covered = BTreeSet::new();
    let mut failures = Vec::new();
    for case in entries(&fixtures(gate)).into_iter().filter(|p| p.is_dir()) {
        let want =
            expected(&std::fs::read_to_string(case.join("expected")).expect("expected file"));
        let got = match run(&case) {
            Ok(findings) => codes(&findings),
            Err(e) => panic!("{}: gate could not run: {e}", case.display()),
        };
        if got != want {
            failures.push(format!(
                "{}: expected {want:?}, got {got:?}",
                case.display()
            ));
        }
        covered.extend(want);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let all: BTreeSet<String> = all.iter().map(|c| (*c).to_string()).collect();
    assert_eq!(
        covered, all,
        "every {gate} code needs a fixture that expects it"
    );
}

#[test]
fn check_layers_fixtures() {
    workspace_gate("check-layers", layers::CODES, layers::run);
}

#[test]
fn check_core_purity_fixtures() {
    workspace_gate("check-core-purity", purity::CODES, purity::run);
}

#[derive(Deserialize)]
struct TrustCase {
    expected: Vec<String>,
    commit: trust::Commit,
}

#[test]
fn check_trust_boundary_fixtures() {
    let policy =
        trust::Policy::load(&repo_root().join("verification/trust-boundary.toml")).expect("policy");
    let mut covered = BTreeSet::new();
    let mut failures = Vec::new();
    for file in entries(&fixtures("check-trust-boundary")) {
        let case: TrustCase = toml::from_str(&std::fs::read_to_string(&file).expect("fixture"))
            .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let want: BTreeSet<String> = case.expected.into_iter().collect();
        let got = codes(&trust::check_commit(&policy, &case.commit));
        if got != want {
            failures.push(format!(
                "{}: expected {want:?}, got {got:?}",
                file.display()
            ));
        }
        covered.extend(want);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let all: BTreeSet<String> = trust::CODES.iter().map(|c| (*c).to_string()).collect();
    assert_eq!(
        covered, all,
        "every check-trust-boundary code needs a fixture"
    );
}

#[test]
fn receipts_fixtures() {
    let registry = Registry::load(&repo_root().join("verification/checks.toml")).expect("registry");
    let mut covered = BTreeSet::new();
    let mut failures = Vec::new();
    for file in entries(&fixtures("receipts")) {
        if file.extension().is_none_or(|e| e != "ndjson") {
            continue;
        }
        let want = expected(
            &std::fs::read_to_string(file.with_extension("expected")).expect("expected file"),
        );
        let text = std::fs::read_to_string(&file).expect("fixture");
        let got = codes(&receipts::validate_text(&registry, "fixture", &text));
        if got != want {
            failures.push(format!(
                "{}: expected {want:?}, got {got:?}",
                file.display()
            ));
        }
        covered.extend(want);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let all: BTreeSet<String> = receipts::CODES.iter().map(|c| (*c).to_string()).collect();
    assert_eq!(covered, all, "every receipt code needs a fixture");
}

#[test]
fn the_repository_passes_its_own_gates() {
    let root = repo_root();
    assert_eq!(layers::run(&root).expect("check-layers ran"), Vec::new());
    assert_eq!(
        purity::run(&root).expect("check-core-purity ran"),
        Vec::new()
    );
    let receipts = root.join("verification/receipts");
    assert_eq!(
        receipts::validate_dir(&root, &receipts).expect("receipts validate ran"),
        Vec::new()
    );
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn commit(dir: &Path, path: &str, text: &str, message: &str) {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, text).expect("write");
    git(dir, &["add", path]);
    git(dir, &["commit", "-q", "-m", message]);
}

/// The collector against a real repository: the root commit is the base and
/// stays out of range, a merge commit is skipped, an undeclared verifier
/// change and an escape hatch in implementation are found, and a base that
/// does not resolve is an error rather than an empty range.
#[test]
fn check_trust_boundary_reads_git() {
    let dir = std::env::temp_dir().join(format!("jacquard-trust-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    git(&dir, &["init", "-q", "-b", "main"]);
    std::fs::create_dir_all(dir.join("verification")).expect("mkdir");
    std::fs::copy(
        repo_root().join("verification/trust-boundary.toml"),
        dir.join("verification/trust-boundary.toml"),
    )
    .expect("copy policy");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "Root, outside any range"]);
    let root = git(&dir, &["rev-parse", "HEAD"]).trim().to_string();

    git(&dir, &["switch", "-q", "-c", "side"]);
    commit(
        &dir,
        "tests/a.rs",
        "fn a() {}\n",
        "Add a declared test\n\nTrust-Boundary: verifier",
    );
    git(&dir, &["switch", "-q", "main"]);
    commit(&dir, "docs/notes.md", "notes\n", "Add notes");
    git(
        &dir,
        &["merge", "-q", "--no-ff", "side", "-m", "Merge side"],
    );
    commit(&dir, "tests/b.rs", "fn b() {}\n", "Loosen a test");
    // Built from pieces: this source must not contain the pattern itself,
    // or check-trust-boundary would read the literal as an escape hatch.
    commit(
        &dir,
        "crates/app/src/lib.rs",
        concat!("#[al", "low(dead_code)]\nfn c() {}\n"),
        "Silence a lint",
    );

    let findings = trust::run(&dir, &root).expect("gate ran");
    let got: Vec<&str> = findings.iter().map(|f| f.code).collect();
    assert_eq!(
        got,
        ["undeclared-oracle-change", "escape-hatch-added"],
        "{findings:#?}"
    );
    assert!(findings[0].detail.contains("tests/b.rs"), "{findings:#?}");

    let head = git(&dir, &["rev-parse", "HEAD"]).trim().to_string();
    assert_eq!(
        trust::run(&dir, &head).expect("gate ran"),
        Vec::new(),
        "an empty range is clean"
    );
    let err = trust::run(&dir, "no-such-ref").expect_err("unresolved base fails");
    assert!(err.contains("unresolved-base"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The mutant harness against its fixture corpus: one entry per result, each
/// of which must come out as the corpus's `expected` table says, so every
/// code of the harness is shown able to appear.
#[test]
fn mutants_semantic_fixtures() {
    use jacquard_xtask::mutants;
    let dir = fixtures("mutants");
    let corpus = mutants::Corpus::load(&dir.join("corpus.toml")).expect("corpus");
    let mut results = std::collections::BTreeMap::new();
    let findings = mutants::run(&dir, &dir.join("corpus.toml"), |id, result| {
        results.insert(id.to_string(), result.to_string());
    })
    .expect("harness ran");
    assert_eq!(results, corpus.expected, "{findings:#?}");
    let reported: BTreeSet<String> = findings.iter().map(|f| f.code.to_string()).collect();
    let all: BTreeSet<String> = mutants::CODES.iter().map(|c| (*c).to_string()).collect();
    assert_eq!(
        reported, all,
        "every mutants code needs a fixture that produces it"
    );
}
