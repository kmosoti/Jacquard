//! `cargo xtask <command>`: the gates, the check registry and receipts.
//!
//! Exit status: 0 when the gate finds nothing, 1 when it prints findings, 2
//! when it cannot run. A gate that cannot run never reports success.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jacquard_xtask::finding::{self, Finding};
use jacquard_xtask::registry::Registry;
use jacquard_xtask::{layers, mutants, purity, receipts, repo_root, trust};

const USAGE: &str = "usage: cargo xtask <command>
  check-layers                      [--root DIR]
  check-core-purity                 [--root DIR]
  check-trust-boundary --base REF   [--root DIR]
  receipts validate                 [--dir DIR]
  receipts record CHECK-ID --out FILE -- COMMAND...
  checks --profile NAME [--base REF] [--out FILE]
  mutants semantic                  [--corpus FILE] [--base REF]";

fn option(args: &mut Vec<String>, name: &str) -> Option<String> {
    let at = args.iter().position(|a| a == name)?;
    if at + 1 >= args.len() {
        return None;
    }
    let value = args.remove(at + 1);
    args.remove(at);
    Some(value)
}

fn gate(word: &str, result: Result<Vec<Finding>, String>) -> u8 {
    match result {
        Ok(findings) => u8::try_from(finding::report(word, &findings)).unwrap_or(1),
        Err(message) => {
            println!("ERROR [gate-could-not-run] {message}");
            2
        }
    }
}

fn record(root: &Path, args: &mut Vec<String>) -> Result<u8, String> {
    let split = args
        .iter()
        .position(|a| a == "--")
        .ok_or("record needs `-- COMMAND...`")?;
    let words: Vec<String> = args.split_off(split).into_iter().skip(1).collect();
    let out = option(args, "--out").ok_or("record needs --out FILE")?;
    let id = args.first().ok_or("record needs a CHECK-ID")?.clone();
    if words.is_empty() {
        return Err("record needs a command after `--`".to_string());
    }
    let registry = Registry::load(&root.join("verification/checks.toml"))?;
    let receipt = receipts::run_check(root, &registry, &id, &words)?;
    receipts::append(&root.join(&out), &receipt)?;
    println!("{id}: {} -> {out}", receipt.outcome);
    Ok(u8::from(receipt.outcome != "passed"))
}

fn checks(root: &Path, args: &mut Vec<String>) -> Result<u8, String> {
    let profile = option(args, "--profile").ok_or("checks needs --profile NAME")?;
    let base = option(args, "--base");
    let out = option(args, "--out")
        .unwrap_or_else(|| format!("target/verification/receipts/{profile}.ndjson"));
    let registry = Registry::load(&root.join("verification/checks.toml"))?;
    let mut failed = false;
    for (id, check) in registry.checks.iter().filter(|(_, c)| c.profile == profile) {
        let command = match &base {
            Some(base) => check.command.replace("<ref>", base),
            None => check.command.clone(),
        };
        let words: Vec<String> = if command.contains('<') {
            Vec::new()
        } else {
            command.split(' ').map(str::to_string).collect()
        };
        println!("== {id}: {command}");
        let receipt = receipts::run_check(root, &registry, id, &words)?;
        receipts::append(&root.join(&out), &receipt)?;
        println!("== {id}: {}", receipt.outcome);
        failed |= receipt.outcome != "passed";
    }
    println!("receipts -> {out}");
    Ok(u8::from(failed))
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let command = args.remove(0);
    let root = option(&mut args, "--root").map_or_else(repo_root, PathBuf::from);
    let code = match command.as_str() {
        "check-layers" => gate("FORBIDDEN", layers::run(&root)),
        "check-core-purity" => gate("IMPURE", purity::run(&root)),
        "check-trust-boundary" => match option(&mut args, "--base") {
            Some(base) => gate("UNDECLARED", trust::run(&root, &base)),
            None => gate(
                "UNDECLARED",
                Err("check-trust-boundary needs --base REF".to_string()),
            ),
        },
        "receipts" if args.first().map(String::as_str) == Some("validate") => {
            args.remove(0);
            let dir = option(&mut args, "--dir")
                .map_or_else(|| root.join("verification/receipts"), PathBuf::from);
            gate("REJECTED", receipts::validate_dir(&root, &dir))
        }
        "receipts" if args.first().map(String::as_str) == Some("record") => {
            args.remove(0);
            record(&root, &mut args).unwrap_or_else(|message| {
                println!("ERROR {message}");
                2
            })
        }
        "mutants" if args.first().map(String::as_str) == Some("semantic") => {
            args.remove(0);
            let corpus = option(&mut args, "--corpus").map_or_else(
                || root.join("tests/semantic-mutants/corpus.toml"),
                PathBuf::from,
            );
            let base = option(&mut args, "--base");
            let res = base
                .as_deref()
                .map_or(Ok(()), |b| mutants::verify_against_base(&root, b))
                .and_then(|()| {
                    mutants::run(&root, &corpus, |id, result| eprintln!("{id}: {result}"))
                });
            gate("SURVIVED", res)
        }
        "checks" => checks(&root, &mut args).unwrap_or_else(|message| {
            println!("ERROR {message}");
            2
        }),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    };
    ExitCode::from(code)
}
