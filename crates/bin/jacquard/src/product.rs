//! `jacquard product ...`: the factory over a Product (ADR-0008, ADR-0009).

use std::collections::{BTreeMap, BTreeSet};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use jacquard_app::compare;
use jacquard_core::judgment::Verdict;
use jacquard_core::probe::{self, Report};
use jacquard_core::viewport::{AUDIT, Viewport};
use jacquard_judge_claude::ClaudeJudge;
use jacquard_loom_claude::ClaudeLoom;
use jacquard_ports::{Judge, Loom, PageRenderer, Render};
use jacquard_renderer_playwright::{Playwright, Profile};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{option, root, scratch, viewports};

const PORT: u16 = 3091;

fn product_dir(name: &str) -> PathBuf {
    root().join("products").join(name)
}

fn records(name: &str) -> PathBuf {
    root().join("verification/products").join(name)
}

fn renders(name: &str, label: &str) -> PathBuf {
    root()
        .join("target/products")
        .join(name)
        .join("renders")
        .join(label)
}

#[derive(Deserialize)]
struct Pages {
    pages: Vec<String>,
}

fn pages(name: &str) -> Result<Vec<String>, String> {
    let path = records(name).join("pages.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(toml::from_str::<Pages>(&text)
        .map_err(|e| e.to_string())?
        .pages)
}

/// The Product's server, stopped when dropped.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn serve(name: &str) -> Result<Server, String> {
    let dir = product_dir(name);
    let binary = dir.join("target/bin/server");
    if !binary.is_file() {
        return Err(format!(
            "{} is not built: run its tools/build.sh",
            dir.display()
        ));
    }
    let child = Command::new(&binary)
        .current_dir(&dir)
        .env("SITE_ROOT", "target/site")
        .env("SITE_ADDR", format!("127.0.0.1:{PORT}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{}: {e}", binary.display()))?;
    let server = Server(child);
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", PORT)).is_ok() {
            return Ok(server);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!("the server did not listen on {PORT} within 30s"))
}

fn slug(page: &str) -> String {
    let s = page.trim_matches('/').replace('/', "_");
    if s.is_empty() { "root".into() } else { s }
}

fn key(page: &str, viewport: Viewport, rule: &str, subject: &str) -> String {
    let masked: String = subject
        .chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect();
    format!("{page} | {viewport} | {rule} | {masked}")
}

fn render_all(name: &str, label: &str, vps: &[Viewport]) -> Result<Vec<Value>, String> {
    let _server = serve(name)?;
    let renderer = Playwright::new(&product_dir(name).join("gfx/node_modules"), &scratch());
    let out = renders(name, label);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let mut findings = Vec::new();
    for page in pages(name)? {
        let url = format!("http://127.0.0.1:{PORT}{page}");
        let shots = renderer.render_url(&url, vps).map_err(|e| e.to_string())?;
        for r in shots {
            let file = out.join(format!("{}@{}.png", slug(&page), r.viewport));
            std::fs::write(&file, &r.png).map_err(|e| e.to_string())?;
            let found = probe::evaluate(&r.probe);
            println!("{page} {}: {} finding(s)", r.viewport, found.len());
            for f in found {
                findings.push(json!({
                    "key": key(&page, r.viewport, f.rule.name(), &f.subject),
                    "page": page, "viewport": r.viewport.to_string(), "rule": f.rule.name(),
                    "subject": f.subject, "detail": f.detail,
                }));
            }
        }
    }
    findings.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
    let text = serde_json::to_string_pretty(&findings).map_err(|e| e.to_string())?;
    std::fs::write(out.join("findings.json"), text).map_err(|e| e.to_string())?;
    println!("renders and findings -> {}", out.display());
    Ok(findings)
}

fn keys(findings: &[Value]) -> BTreeSet<String> {
    findings
        .iter()
        .filter_map(|f| f["key"].as_str().map(str::to_string))
        .collect()
}

fn probe_cmd(name: &str, args: &mut Vec<String>) -> Result<u8, String> {
    let record = args.iter().any(|a| a == "--record");
    let label = option(args, "--label").unwrap_or_else(|| "probe".into());
    let findings = render_all(name, &label, &AUDIT)?;
    let baseline_path = records(name).join("probe-baseline.json");
    if record {
        let text = serde_json::to_string_pretty(&findings).map_err(|e| e.to_string())?;
        std::fs::write(&baseline_path, text + "\n").map_err(|e| e.to_string())?;
        println!("recorded {} finding(s) as the baseline", findings.len());
        return Ok(0);
    }
    let text = std::fs::read_to_string(&baseline_path)
        .map_err(|e| format!("{}: {e}", baseline_path.display()))?;
    let baseline: Vec<Value> = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let (now, before) = (keys(&findings), keys(&baseline));
    let new: Vec<_> = now.difference(&before).collect();
    let fixed: Vec<_> = before.difference(&now).collect();
    for k in &fixed {
        println!("fixed  {k}");
    }
    for k in &new {
        println!("NEW    {k}");
    }
    println!(
        "baseline {} | now {} | fixed {} | new {}",
        before.len(),
        now.len(),
        fixed.len(),
        new.len()
    );
    Ok(u8::from(!new.is_empty()))
}

fn render_file(path: &Path, viewport: Viewport) -> Result<Render, String> {
    let png = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Render {
        viewport,
        png,
        probe: Report::default(),
    })
}

fn judge_cmd(name: &str, args: &mut Vec<String>) -> Result<u8, String> {
    let before = option(args, "--before").ok_or("judge needs --before LABEL")?;
    let after = option(args, "--after").ok_or("judge needs --after LABEL")?;
    let vps = viewports(Some(
        option(args, "--viewports")
            .as_deref()
            .unwrap_or("1440x900,390x844"),
    ))?;
    let only: Option<BTreeSet<String>> =
        option(args, "--pages").map(|p| p.split(',').map(str::to_string).collect());
    let judge = ClaudeJudge::from_env(&scratch());
    let out = records(name).join(format!("judge-{before}-vs-{after}.ndjson"));
    let _ = std::fs::remove_file(&out);
    let mut tally: BTreeMap<&str, u32> = BTreeMap::new();
    let mut lines = Vec::new();
    for page in pages(name)? {
        if only.as_ref().is_some_and(|o| !o.contains(&page)) {
            continue;
        }
        for vp in &vps {
            let file = |label: &str| renders(name, label).join(format!("{}@{vp}.png", slug(&page)));
            let (b, a) = (
                render_file(&file(&before), *vp)?,
                render_file(&file(&after), *vp)?,
            );
            let c = compare(&judge, std::slice::from_ref(&b), std::slice::from_ref(&a))
                .map_err(|e| e.to_string())?;
            // b = before shown first; the second comparison shows after first.
            let class = match (c.a_first.verdict, c.b_first.verdict) {
                (Verdict::Second, Verdict::First) => "improved",
                (Verdict::First, Verdict::Second) => "regressed",
                _ => "unchanged",
            };
            *tally.entry(class).or_default() += 1;
            println!("{page} {vp}: {class}  ({})", c.b_first.reason);
            lines.push(json!({
                "page": page, "viewport": vp.to_string(), "class": class,
                "before_first": format!("{:?}", c.a_first.verdict), "after_first": format!("{:?}", c.b_first.verdict),
                "model": c.a_first.model, "rubric": judge.rubric_hash(),
                "reason_before_first": c.a_first.reason, "reason_after_first": c.b_first.reason,
            }));
        }
    }
    let body: String = lines.iter().map(|l| l.to_string() + "\n").collect();
    std::fs::write(&out, body).map_err(|e| e.to_string())?;
    println!("{tally:?} -> {}", out.display());
    Ok(0)
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

fn perf_cmd(name: &str, args: &mut Vec<String>) -> Result<u8, String> {
    let label = option(args, "--label").ok_or("perf needs --label LABEL")?;
    let build = option(args, "--build")
        .ok_or("perf needs --build dev|release, the build that is served")?;
    let runs: u32 = option(args, "--runs").map_or(Ok(3), |r| {
        r.parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())
    })?;
    let _server = serve(name)?;
    let pw = Playwright::new(&product_dir(name).join("gfx/node_modules"), &scratch());
    let mut results = Vec::new();
    for page in ["/", "/observability"] {
        for profile in [Profile::Phone, Profile::Desktop] {
            let url = format!("http://127.0.0.1:{PORT}{page}");
            let rows = pw.measure(&url, profile, runs).map_err(|e| e.to_string())?;
            let metric = |k: &str| {
                median(
                    rows.iter()
                        .filter(|r| r["error"].is_null())
                        .filter_map(|r| r[k].as_f64())
                        .collect(),
                )
            };
            let summary = json!({
                "page": page, "profile": profile.name(),
                "median": {"lcp_ms": metric("lcp_ms"), "tbt_ms": metric("tbt_ms"), "cls": metric("cls"), "bytes": metric("bytes"), "requests": metric("requests"), "load_ms": metric("load_ms")},
                "runs": rows,
            });
            println!("{page} {}: {}", profile.name(), summary["median"]);
            results.push(summary);
        }
    }
    let record = json!({"label": label, "build": build, "results": results});
    let out = records(name).join(format!("perf-{label}.json"));
    std::fs::write(
        &out,
        serde_json::to_string_pretty(&record).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!("-> {}", out.display());
    Ok(0)
}

fn weave_cmd(name: &str, args: &mut Vec<String>) -> Result<u8, String> {
    let draft_path = option(args, "--draft").ok_or("weave needs --draft FILE")?;
    let out = PathBuf::from(option(args, "--out").ok_or("weave needs --out DIR")?);
    let draft = std::fs::read_to_string(&draft_path).map_err(|e| format!("{draft_path}: {e}"))?;
    let weave = ClaudeLoom::from_env()
        .weave(&draft, &product_dir(name), &out)
        .map_err(|e| e.to_string())?;
    let patch = out.with_extension("patch");
    std::fs::write(&patch, &weave.patch).map_err(|e| e.to_string())?;
    println!("model {}", weave.model);
    println!("changed {}", weave.changed.join(", "));
    println!("summary {}", weave.summary);
    println!("patch -> {}", patch.display());
    Ok(u8::from(weave.model != jacquard_loom_claude::MODEL))
}

fn check_cmd(name: &str) -> Result<u8, String> {
    let dir = product_dir(name);
    let steps: [(&str, &str, &[&str]); 4] = [
        ("fmt", ".", &["cargo", "fmt", "--all", "--check"]),
        (
            "clippy-workspace",
            ".",
            &[
                "cargo",
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
        ),
        (
            "rust-tests",
            ".",
            &["cargo", "test", "--workspace", "--locked"],
        ),
        (
            "check-wasm32",
            ".",
            &[
                "cargo",
                "check",
                "-p",
                "cockpit-leptos",
                "--no-default-features",
                "--features",
                "csr",
                "--target",
                "wasm32-unknown-unknown",
                "--locked",
            ],
        ),
    ];
    let mut failed = false;
    for (id, sub, cmd) in steps {
        let status = Command::new(cmd[0])
            .args(&cmd[1..])
            .current_dir(dir.join(sub))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("{}: {e}", cmd[0]))?;
        println!(
            "{id}: {}",
            if status.success() { "passed" } else { "failed" }
        );
        failed |= !status.success();
    }
    Ok(u8::from(failed))
}

/// Dispatches `jacquard product <command> [NAME] ...`.
pub fn run(args: &mut Vec<String>) -> Result<u8, String> {
    let command = if args.is_empty() {
        String::new()
    } else {
        args.remove(0)
    };
    let name = option(args, "--product").unwrap_or_else(|| "cockpit".into());
    match command.as_str() {
        "render" => {
            let label = option(args, "--label").ok_or("render needs --label LABEL")?;
            let vps = viewports(option(args, "--viewports").as_deref())?;
            render_all(&name, &label, &vps).map(|_| 0)
        }
        "probe" => probe_cmd(&name, args),
        "judge" => judge_cmd(&name, args),
        "perf" => perf_cmd(&name, args),
        "weave" => weave_cmd(&name, args),
        "check" => check_cmd(&name),
        _ => Err("product commands: render --label L | probe [--record] | judge --before L --after L | perf --label L --build B | weave --draft F --out D | check".into()),
    }
}
