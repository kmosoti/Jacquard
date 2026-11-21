//! The `jacquard` command line: the only place adapters meet ports.
//!
//! Exit status: 0 success, 1 a check or admission said no, 2 could not run.

mod product;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jacquard_app::admission::{self, Pair};
use jacquard_app::{compare, probe_gate, swatch_id};
use jacquard_core::admission::{Decision, Presentation};
use jacquard_core::check::Outcome;
use jacquard_core::judgment::Verdict;
use jacquard_core::probe::Rule;
use jacquard_core::viewport::{AUDIT, Viewport};
use jacquard_judge_claude::ClaudeJudge;
use jacquard_ledger_fs::FsLedger;
use jacquard_ports::{Answer, Judge, Ledger, Render, Renderer, SwatchSource};
use jacquard_renderer_chrome::Chrome;
use serde::Deserialize;
use serde_json::json;

const USAGE: &str = "usage: jacquard <command>
  render DIR [--viewports audit|WxH,...] [--out DIR]
  compare DIR-A DIR-B [--viewports ...] [--ledger DIR]
  rank FILE [--seed N]
  admission verify [--set DIR]
  admission run [--set DIR] [--limit N]
  product render|probe|judge|perf|weave|check ...";

fn root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    here.canonicalize().unwrap_or(here)
}

fn scratch() -> PathBuf {
    std::env::temp_dir().join("jacquard")
}

fn option(args: &mut Vec<String>, name: &str) -> Option<String> {
    let at = args.iter().position(|a| a == name)?;
    if at + 1 >= args.len() {
        return None;
    }
    let value = args.remove(at + 1);
    args.remove(at);
    Some(value)
}

fn viewports(spec: Option<&str>) -> Result<Vec<Viewport>, String> {
    match spec {
        None | Some("audit") => Ok(AUDIT.to_vec()),
        Some(list) => list
            .split(',')
            .map(|v| Viewport::parse(v).ok_or_else(|| format!("`{v}` is not WIDTHxHEIGHT")))
            .collect(),
    }
}

fn load(dir: &Path) -> Result<SwatchSource, String> {
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(dir)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .into_owned();
                files.insert(
                    rel,
                    std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?,
                );
            }
        }
    }
    Ok(SwatchSource {
        files,
        entry: "index.html".into(),
    })
}

fn print_findings(label: &str, renders: &[Render]) -> Outcome {
    let (outcome, per) = probe_gate(renders);
    for (viewport, findings) in per {
        if findings.is_empty() {
            println!("{label} {viewport}: clean");
        }
        for f in findings {
            println!("{label} {viewport}: [{}] {}", f.rule.name(), f.detail);
        }
    }
    outcome
}

fn render_cmd(args: &mut Vec<String>) -> Result<u8, String> {
    let vps = viewports(option(args, "--viewports").as_deref())?;
    let out = option(args, "--out");
    let dir = PathBuf::from(args.first().ok_or("render needs a DIR")?);
    let source = load(&dir)?;
    let id = swatch_id(&source);
    let hex = hex(&id.0[..6]);
    let renders = Chrome::from_env(&scratch())
        .render(&source, &vps)
        .map_err(|e| e.to_string())?;
    if let Some(out) = out {
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        for r in &renders {
            let path = Path::new(&out).join(format!("{hex}-{}.png", r.viewport));
            std::fs::write(&path, &r.png).map_err(|e| e.to_string())?;
            println!("wrote {}", path.display());
        }
    }
    Ok(u8::from(
        print_findings(&format!("swatch {hex}"), &renders) != Outcome::Passed,
    ))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn rank_cmd(args: &mut Vec<String>) -> Result<u8, String> {
    let seed =
        option(args, "--seed").map_or(Ok(0), |s| s.parse::<u64>().map_err(|e| e.to_string()))?;
    let file = args
        .first()
        .ok_or("rank needs a FILE of comparison records")?;
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let mut recorded = Vec::new();
    for (n, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("{file}:{}: {e}", n + 1))?;
        if v["record"] != "comparison" {
            continue;
        }
        let verdict = match v["verdict"].as_str() {
            Some("First") => Verdict::First,
            Some("Second") => Verdict::Second,
            Some("Tie") => Verdict::Tie,
            other => return Err(format!("{file}:{}: verdict {other:?}", n + 1)),
        };
        let id = |k: &str| {
            v[k].as_str()
                .map(str::to_string)
                .ok_or(format!("{file}:{}: no `{k}`", n + 1))
        };
        recorded.push(jacquard_app::ranking::Recorded {
            first: id("first")?,
            second: id("second")?,
            verdict,
        });
    }
    let rate = jacquard_app::ranking::rating;
    println!(
        "{:<5} {:<12} {:>7} {:>18} {:>4}",
        "rank", "swatch", "rating", "95% interval", "n"
    );
    for r in jacquard_app::ranking::rank(&recorded, seed) {
        let e = r.estimate;
        let rank = e.rank.map_or_else(|| "-".to_string(), |k| k.to_string());
        println!(
            "{rank:<5} {:<12} {:>7.0} [{:>7.0},{:>7.0}] {:>4}{}",
            r.id.get(..12).unwrap_or(&r.id),
            rate(e.strength),
            rate(e.low),
            rate(e.high),
            e.comparisons,
            if e.rank.is_none() {
                "  below the floor"
            } else {
                ""
            }
        );
    }
    Ok(0)
}

fn letter(v: Verdict, first: &str, second: &str) -> String {
    match v {
        Verdict::First => first.into(),
        Verdict::Second => second.into(),
        Verdict::Tie => "tie".into(),
    }
}

fn compare_cmd(args: &mut Vec<String>) -> Result<u8, String> {
    let vps = viewports(Some(
        option(args, "--viewports")
            .as_deref()
            .unwrap_or("1440x900,390x844"),
    ))?;
    let ledger = option(args, "--ledger").map(|d| FsLedger::new(Path::new(&d)));
    let [a, b] = [0, 1].map(|i| args.get(i).cloned());
    let (a, b) = (
        a.ok_or("compare needs DIR-A")?,
        b.ok_or("compare needs DIR-B")?,
    );
    let chrome = Chrome::from_env(&scratch());
    let (sa, sb) = (load(Path::new(&a))?, load(Path::new(&b))?);
    let (ida, idb) = (hex(&swatch_id(&sa).0), hex(&swatch_id(&sb).0));
    let ra = chrome.render(&sa, &vps).map_err(|e| e.to_string())?;
    let rb = chrome.render(&sb, &vps).map_err(|e| e.to_string())?;
    let judge = ClaudeJudge::from_env(&scratch());
    let c = compare(&judge, &ra, &rb).map_err(|e| e.to_string())?;
    let show = |label: &str, x: &Answer, first: &str, second: &str| {
        println!(
            "{label}: {} ({}), {}",
            letter(x.verdict, first, second),
            x.model,
            x.reason
        );
    };
    show("A shown first", &c.a_first, "A", "B");
    show("B shown first", &c.b_first, "B", "A");
    if let Some(ledger) = ledger {
        for (first, second, answer) in [(&ida, &idb, &c.a_first), (&idb, &ida, &c.b_first)] {
            let record = json!({"record": "comparison", "first": first, "second": second, "verdict": format!("{:?}", answer.verdict), "model": answer.model, "rubric": judge.rubric_hash()});
            ledger
                .append("comparisons", &record.to_string())
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(0)
}

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
            let source = load(&dir.join("swatches").join(&p.base))?;
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

fn admission_cmd(args: &mut Vec<String>) -> Result<u8, String> {
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

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let command = args.remove(0);
    let result = match command.as_str() {
        "render" => render_cmd(&mut args),
        "compare" => compare_cmd(&mut args),
        "rank" => rank_cmd(&mut args),
        "admission" => admission_cmd(&mut args),
        "product" => product::run(&mut args),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("ERROR {message}");
            ExitCode::from(2)
        }
    }
}
