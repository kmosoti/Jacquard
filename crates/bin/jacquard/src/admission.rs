//! `jacquard admission verify|run`: Judge admission verification and runs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jacquard_app::admission::{self, Pair};
use jacquard_core::admission::{Decision, Presentation};
use jacquard_core::probe::Rule;
use jacquard_core::viewport::Viewport;
use jacquard_judge_claude::ClaudeJudge;
use jacquard_ledger_fs::FsLedger;
use jacquard_ports::{Judge, Ledger, Render, Renderer, SwatchSource};
use jacquard_renderer_chrome::Chrome;
use serde::Deserialize;
use serde_json::json;

use crate::{option, root, scratch, swatch_loader};

#[derive(Deserialize)]
struct Defects {
    defects: BTreeMap<String, Defect>,
}

#[derive(Deserialize)]
struct Defect {
    rule: String,
    css: String,
}

#[derive(Deserialize)]
struct Set {
    pair: Vec<SetPair>,
}

#[derive(Deserialize)]
struct SetPair {
    id: String,
    base: String,
    defect: String,
    viewport: String,
}

fn with_defect(source: &SwatchSource, css: &str) -> SwatchSource {
    let mut variant = source.clone();
    if let Some(bytes) = variant.files.get_mut(&source.entry) {
        let html = String::from_utf8_lossy(bytes);
        let style = format!("<style>{css}</style>");
        *bytes = match html.rfind("</head>") {
            Some(at) => format!("{}{style}{}", &html[..at], &html[at..]),
            None => format!("{style}{html}"),
        }
        .into_bytes();
    }
    variant
}

fn render_set(dir: &Path) -> Result<Vec<Pair>, String> {
    let read =
        |name: &str| std::fs::read_to_string(dir.join(name)).map_err(|e| format!("{name}: {e}"));
    let defects: Defects = toml::from_str(&read("defects.toml")?).map_err(|e| e.to_string())?;
    let set: Set = toml::from_str(&read("set.toml")?).map_err(|e| e.to_string())?;
    let chrome = Chrome::from_env(&scratch());
    let mut bases: BTreeMap<(String, Viewport), (SwatchSource, Vec<Render>)> = BTreeMap::new();
    let mut pairs = Vec::new();
    for p in set.pair {
        let viewport =
            Viewport::parse(&p.viewport).ok_or_else(|| format!("{}: bad viewport", p.id))?;
        let defect = defects
            .defects
            .get(&p.defect)
            .ok_or_else(|| format!("{}: no defect {}", p.id, p.defect))?;
        let rule = Rule::from_name(&defect.rule)
            .ok_or_else(|| format!("{}: no rule {}", p.id, defect.rule))?;
        let key = (p.base.clone(), viewport);
        if !bases.contains_key(&key) {
            let source = swatch_loader::load(&dir.join("swatches").join(&p.base))?;
            let renders = chrome
                .render(&source, &[viewport])
                .map_err(|e| format!("{}: {e}", p.base))?;
            bases.insert(key.clone(), (source, renders));
        }
        let (source, base) = &bases[&key];
        let variant = chrome
            .render(&with_defect(source, &defect.css), &[viewport])
            .map_err(|e| format!("{}: {e}", p.id))?;
        eprintln!("rendered {}", p.id);
        pairs.push(Pair {
            id: p.id,
            rule,
            base: base.clone(),
            variant,
        });
    }
    Ok(pairs)
}

fn verify_all(pairs: &[Pair]) -> bool {
    let mut ok = true;
    for pair in pairs {
        match admission::verify(pair) {
            Ok(()) => println!("known  {} ({})", pair.id, pair.rule.name()),
            Err(why) => {
                ok = false;
                println!("NOT KNOWN {why}");
            }
        }
    }
    ok
}

/// Executes `jacquard admission <verify|run>`.
///
/// # Errors
///
/// Returns an error if the admission set cannot be loaded, rendered, or run.
pub fn run(args: &mut Vec<String>) -> Result<u8, String> {
    let dir =
        option(args, "--set").map_or_else(|| root().join("verification/admission"), PathBuf::from);
    let limit = option(args, "--limit")
        .map(|n| n.parse::<usize>().map_err(|e| e.to_string()))
        .transpose()?;
    let sub = args.first().cloned().unwrap_or_default();
    let mut pairs = render_set(&dir)?;
    let known = verify_all(&pairs);
    match sub.as_str() {
        "verify" => Ok(u8::from(!known)),
        "run" if !known => {
            Err("the set has pairs whose answer is not known; fix the set first".into())
        }
        "run" => {
            if let Some(n) = limit {
                pairs.truncate(n);
            }
            let judge = ClaudeJudge::from_env(&scratch());
            let run_name = format!("{}-{}", judge.rubric_hash(), judge.pinned_model());
            let ledger = FsLedger::new(&dir.join("reports"));
            let mut stored = Ok(());
            let result = admission::run(&judge, &pairs, |r| {
                let (verdict, model, reason, error) = match &r.answer {
                    Ok(a) => (
                        Some(format!("{:?}", a.verdict)),
                        a.model.clone(),
                        a.reason.clone(),
                        None,
                    ),
                    Err(e) => (None, String::new(), String::new(), Some(e.to_string())),
                };
                let order = match r.presentation {
                    Presentation::BaseFirst => "base-first",
                    Presentation::VariantFirst => "variant-first",
                };
                println!(
                    "{} {order}: {}",
                    r.pair,
                    verdict.as_deref().or(error.as_deref()).unwrap_or("")
                );
                let record = json!({"record": "judgment", "pair": r.pair, "presentation": order, "verdict": verdict, "model": model, "reason": reason, "error": error});
                if stored.is_ok() {
                    stored = ledger.append(&run_name, &record.to_string());
                }
            });
            stored.map_err(|e| e.to_string())?;
            let report = &result.report;
            let decision = match &report.decision {
                Decision::Admitted => "admitted".to_string(),
                Decision::Rejected(why) => format!("rejected {why:?}"),
                Decision::Inconclusive => "inconclusive".to_string(),
            };
            let summary = json!({"record": "admission", "rubric": judge.rubric_hash(), "model": judge.pinned_model(), "pairs": report.pairs, "correct": report.correct, "flipped": report.flipped, "wrong": report.wrong, "decision": decision, "limited_to": limit});
            ledger
                .append(&run_name, &summary.to_string())
                .map_err(|e| e.to_string())?;
            println!("{summary}");
            Ok(u8::from(report.decision != Decision::Admitted))
        }
        _ => Err(format!("unknown admission command `{sub}`")),
    }
}
