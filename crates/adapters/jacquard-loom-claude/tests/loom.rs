//! The Loom's session report: exactly Read, Glob, Grep, Edit and Write and
//! no MCP server, or the Weave is refused; the CLI arguments pin the model
//! and restrict the session; `prepare` makes the copy a clean repository.

use jacquard_loom_claude::{ClaudeLoom, MODEL, TOOLS, parse, prepare};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}.jsonl",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

#[test]
fn a_restricted_session_parses() {
    let (model, summary) = parse(&fixture("good")).expect("parsed");
    assert_eq!(model, MODEL);
    assert!(summary.contains("status column"));
}

#[test]
fn a_shell_an_mcp_server_or_an_error_is_refused() {
    assert!(
        parse(&fixture("with-bash"))
            .unwrap_err()
            .to_string()
            .contains("Bash")
    );
    assert!(
        parse(&fixture("with-mcp"))
            .unwrap_err()
            .to_string()
            .contains("MCP")
    );
    assert!(parse(&fixture("cli-error")).is_err());
    assert!(parse(&fixture("no-session")).is_err());
}

#[test]
fn the_cli_is_pinned_restricted_and_given_exactly_five_tools() {
    let args = ClaudeLoom::arguments();
    let after = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .map(|i| args[i + 1].as_str())
    };
    assert_eq!(after("--model"), Some(MODEL));
    assert!(args.iter().any(|a| a == "--restricted"));
    assert_eq!(after("--permission-mode"), Some("acceptEdits"));
    let at = args.iter().position(|a| a == "--tools").expect("--tools");
    assert_eq!(&args[at + 1..], TOOLS.map(String::from));
    assert!(!args.iter().any(|a| a == "Bash"));
}

#[test]
fn prepare_copies_listed_files_into_a_committed_repository() {
    let base = std::env::temp_dir().join(format!("jacquard-loom-base-{}", std::process::id()));
    let out = std::env::temp_dir().join(format!("jacquard-loom-out-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("src")).expect("mkdir");
    std::fs::write(base.join("src/a.css"), "p{}").expect("write");
    let run = |dir: &std::path::Path, args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git")
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    run(&base, &["init", "-q"]);
    prepare(&base, &out).expect("prepared");
    assert_eq!(
        std::fs::read_to_string(out.join("src/a.css")).expect("copied"),
        "p{}"
    );
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&out)
        .output()
        .expect("git");
    assert!(status.stdout.is_empty(), "the copy starts clean");
    let _ = std::fs::remove_dir_all(&base);
    let _ = std::fs::remove_dir_all(&out);
}
