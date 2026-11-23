//! Semantic-mutant inventory integrity and `Trust-Boundary` protection tests.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use jacquard_xtask::{mutants, repo_root, trust};

fn run_git(dir: &Path, args: &[&str]) -> String {
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

fn commit_file(dir: &Path, path: &str, text: &str, message: &str) {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, text).expect("write");
    run_git(dir, &["add", path]);
    run_git(dir, &["commit", "-q", "-m", message]);
}

#[test]
fn production_mutants_corpus_is_non_empty_and_complete() {
    let err = mutants::Corpus::parse("mutant = []\n", "empty")
        .expect_err("empty corpus must be rejected");
    assert!(err.contains("must not be empty"), "{err}");

    let corpus = mutants::Corpus::load(&repo_root().join(mutants::PRODUCTION_CORPUS))
        .expect("production corpus loads");
    let ids: BTreeSet<&str> = corpus.mutant.iter().map(|m| m.id.as_str()).collect();
    for required in [
        "admission-tie-counts-correct",
        "admission-accuracy-exclusive",
        "admission-flip-rate-exclusive",
        "probe-contrast-loosened",
        "probe-inline-not-exempt",
        "probe-every-target-exempt",
        "judge-accepts-tools",
        "judge-hides-model",
        "renderer-accepts-wrong-viewport",
        "compare-one-order",
        "swatch-id-without-length-prefix",
        "rank-ignores-ties",
        "rank-below-floor",
        "rank-depends-on-record-order",
        "loom-accepts-a-shell",
        "page-renderer-accepts-wrong-viewport",
    ] {
        assert!(
            ids.contains(required),
            "production semantic-mutant corpus missing required ID {required}"
        );
    }
}

#[test]
fn deleted_or_weakened_mutant_requires_escape_hatch() {
    let dir = std::env::temp_dir().join(format!("jacquard-mutant-del-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    run_git(&dir, &["init", "-q", "-b", "main"]);
    std::fs::create_dir_all(dir.join("verification")).expect("mkdir");
    std::fs::copy(
        repo_root().join("verification/trust-boundary.toml"),
        dir.join("verification/trust-boundary.toml"),
    )
    .expect("copy policy");
    let corpus_initial = r#"[[mutant]]
id = "m-one"
invariant = "one"
file = "crates/fx/src/lib.rs"
find = "a"
replace = "b"
package = "fx"
test = "t"
name = "n"

[[mutant]]
id = "m-two"
invariant = "two"
file = "crates/fx/src/lib.rs"
find = "c"
replace = "d"
package = "fx"
test = "t"
name = "n2"
"#;
    commit_file(
        &dir,
        mutants::PRODUCTION_CORPUS,
        corpus_initial,
        "Initial corpus\n\nTrust-Boundary: verifier",
    );
    let base = run_git(&dir, &["rev-parse", "HEAD"]).trim().to_string();

    // Deleting a mutant entry without Trust-Boundary: escape-hatch must fail.
    let corpus_deleted = r#"[[mutant]]
id = "m-one"
invariant = "one"
file = "crates/fx/src/lib.rs"
find = "a"
replace = "b"
package = "fx"
test = "t"
name = "n"
"#;
    commit_file(
        &dir,
        mutants::PRODUCTION_CORPUS,
        corpus_deleted,
        "Drop m-two quietly\n\nTrust-Boundary: verifier",
    );
    let findings = trust::run(&dir, &base).expect("trust ran");
    assert_eq!(
        findings.iter().map(|f| f.code).collect::<Vec<_>>(),
        ["escape-hatch-added"],
        "{findings:#?}"
    );
    assert!(mutants::verify_against_base(&dir, &base).is_err());

    // Empty corpus check fails closed too.
    let after_del = run_git(&dir, &["rev-parse", "HEAD"]).trim().to_string();
    commit_file(
        &dir,
        mutants::PRODUCTION_CORPUS,
        "mutant = []\n",
        "Empty corpus",
    );
    assert!(
        mutants::verify_against_base(&dir, &after_del).is_err(),
        "empty corpus must be rejected"
    );

    // Declaring Trust-Boundary: escape-hatch satisfies the declaration gate.
    run_git(&dir, &["reset", "--hard", &base]);
    commit_file(
        &dir,
        mutants::PRODUCTION_CORPUS,
        corpus_deleted,
        "Drop m-two explicitly\n\nTrust-Boundary: verifier\nTrust-Boundary: escape-hatch",
    );
    assert_eq!(
        trust::run(&dir, &base).expect("trust ran"),
        Vec::new(),
        "declared escape-hatch allows intentional corpus removal"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
