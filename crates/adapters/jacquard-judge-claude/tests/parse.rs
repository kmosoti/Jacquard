//! The CLI's stream-JSON, from fixtures shaped on a recorded run (2026-09-28,
//! Claude CLI 2.1.283): a complete verdict parses; a context with tools or
//! MCP servers attached, a CLI error, prose without JSON, or an incomplete
//! rubric is a failure; the answering model is reported as the CLI named it.

use jacquard_core::judgment::Verdict;
use jacquard_judge_claude::{ClaudeJudge, DIMENSIONS, MODEL, parse, request};
use jacquard_ports::{Failure, Judge, Render};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}.jsonl", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn failed(name: &str) -> String {
    match parse(&fixture(name)) {
        Err(Failure::Failed(m)) => m,
        other => panic!("{name}: expected a failure, got {other:?}"),
    }
}

#[test]
fn a_complete_verdict_parses() {
    let answer = parse(&fixture("good")).expect("parsed");
    assert_eq!(answer.verdict, Verdict::First);
    assert_eq!(answer.model, MODEL);
    assert_eq!(answer.dimensions.len(), DIMENSIONS.len());
    assert_eq!(answer.dimensions["consistency"], Verdict::Tie);
}

#[test]
fn a_fenced_verdict_in_lower_case_parses() {
    assert_eq!(
        parse(&fixture("fenced")).expect("parsed").verdict,
        Verdict::Second
    );
}

#[test]
fn another_model_is_reported_not_hidden() {
    assert_eq!(
        parse(&fixture("other-model")).expect("parsed").model,
        "claude-sonnet-5"
    );
}

#[test]
fn an_unisolated_context_is_a_failure() {
    assert!(failed("tools-attached").contains("not isolated"));
    assert!(failed("mcp-attached").contains("not isolated"));
    assert!(failed("no-session").contains("no session"));
}

#[test]
fn incomplete_or_erroneous_answers_are_failures() {
    assert!(failed("missing-dimension").contains("usability"));
    assert!(failed("prose-only").contains("no JSON verdict"));
    assert!(failed("cli-error").contains("error"));
}

#[test]
fn the_request_carries_only_labels_images_and_the_ask() {
    let render = Render {
        viewport: jacquard_core::viewport::Viewport {
            width: 390,
            height: 844,
        },
        png: b"\x89PNG fake".to_vec(),
        probe: jacquard_core::probe::Report {
            clipped: vec!["SECRET-PROBE-FINDING".into()],
            ..Default::default()
        },
    };
    let line = request(std::slice::from_ref(&render), std::slice::from_ref(&render));
    assert!(!line.contains('\n'), "one stream-JSON line");
    assert!(
        !line.contains("SECRET-PROBE-FINDING"),
        "the Judge never sees probe data"
    );
    let v: serde_json::Value = serde_json::from_str(&line).expect("json");
    let kinds: Vec<&str> = v["message"]["content"]
        .as_array()
        .expect("content")
        .iter()
        .map(|c| c["type"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        kinds,
        ["text", "text", "image", "text", "text", "image", "text"]
    );
}

#[test]
fn the_cli_is_pinned_and_stripped() {
    let args = ClaudeJudge::arguments();
    let after = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .map(|i| args[i + 1].as_str())
    };
    assert_eq!(after("--model"), Some(MODEL));
    assert_eq!(after("--tools"), Some(""));
    assert_eq!(after("--mcp-config"), Some(r#"{"mcpServers":{}}"#));
    assert!(args.iter().any(|a| a == "--strict-mcp-config"));
    assert!(args.iter().any(|a| a == "--no-session-persistence"));
    assert_eq!(after("--setting-sources"), Some("project"));
    let judge = ClaudeJudge::from_env(std::path::Path::new("/tmp"));
    assert_eq!(judge.pinned_model(), MODEL);
    assert_eq!(judge.rubric_hash().len(), 16);
}
