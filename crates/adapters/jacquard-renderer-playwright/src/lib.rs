//! The Playwright page renderer (ADR-0008): a Product's own Playwright and
//! Chromium, driven by a Node script, render a served page at each
//! viewport. It waits for the scene to report itself ready (`webgl`,
//! `webgpu` or `schematic`), lets the camera settle, injects the layout
//! probe the Chrome renderer injects (the same file), and screenshots the
//! viewport. Service workers are blocked so every render is of the network,
//! not a cache. The probe's rules stay in `jacquard-core`.
#![forbid(unsafe_code)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use jacquard_core::probe::{ContrastSample, Report, Target};
use jacquard_core::viewport::Viewport;
use jacquard_ports::{Failure, PageRenderer, Render};
use serde::Deserialize;
use serde_json::json;

/// The Node render driver.
pub const SCRIPT: &str = include_str!("render.mjs");

/// The Node performance driver.
pub const PERF: &str = include_str!("perf.mjs");

/// The layout probe: the Chrome renderer's file, so both measure alike.
pub const PROBE: &str = include_str!("../../jacquard-renderer-chrome/src/probe.js");

const BROWSER_LOCK: &str = "/tmp/jacquard-browser.lock";

/// How long one page, at every viewport, may take before it is killed.
pub const DEADLINE: Duration = Duration::from_secs(600);

/// Renders pages with a Product's Playwright.
#[derive(Clone, Debug)]
pub struct Playwright {
    module: PathBuf,
    chrome: PathBuf,
    scratch: PathBuf,
    settle_ms: u32,
}

impl Playwright {
    /// A renderer using `playwright-core` from a Product's `node_modules`
    /// and the cached Chrome for Testing (or `JACQUARD_CHROME`).
    #[must_use]
    pub fn new(node_modules: &Path, scratch: &Path) -> Self {
        let chrome = std::env::var_os("JACQUARD_CHROME").map_or_else(
            || {
                let home = std::env::var_os("HOME").unwrap_or_default();
                Path::new(&home).join(".cache/ms-playwright/chromium-1234/chrome-linux64/chrome")
            },
            PathBuf::from,
        );
        Self {
            module: node_modules.join("playwright-core/index.mjs"),
            chrome,
            scratch: scratch.to_path_buf(),
            settle_ms: 2500,
        }
    }
}

#[derive(Deserialize)]
struct RawContrast {
    element: String,
    ratio: f64,
    large: bool,
}

#[derive(Deserialize)]
struct RawTarget {
    element: String,
    width: f64,
    height: f64,
    inline: bool,
}

#[derive(Deserialize)]
struct RawReport {
    viewport_width: u32,
    viewport_height: u32,
    document_width: u32,
    clipped: Vec<String>,
    overlaps: Vec<(String, String)>,
    contrast: Vec<RawContrast>,
    targets: Vec<RawTarget>,
}

#[derive(Deserialize)]
struct Line {
    viewport: (u32, u32),
    png: Option<String>,
    probe: Option<RawReport>,
    error: Option<String>,
}

/// Turns one driver line into a Render, refusing a page that received
/// another viewport than the one asked for.
///
/// # Errors
///
/// Fails on a driver error, a missing screenshot or probe, or a mismatched
/// viewport.
pub fn parse_line(line: &str, url: &str) -> Result<Render, Failure> {
    let l: Line =
        serde_json::from_str(line).map_err(|e| Failure::Failed(format!("driver output: {e}")))?;
    let viewport = Viewport {
        width: l.viewport.0,
        height: l.viewport.1,
    };
    if let Some(error) = l.error {
        return Err(Failure::Failed(format!("{url} at {viewport}: {error}")));
    }
    let raw = l
        .probe
        .ok_or_else(|| Failure::Failed(format!("{url} at {viewport}: no probe report")))?;
    if (raw.viewport_width, raw.viewport_height) != (viewport.width, viewport.height) {
        return Err(Failure::Failed(format!(
            "{url}: asked for {viewport}, the page received {}x{}",
            raw.viewport_width, raw.viewport_height
        )));
    }
    let png_path = l
        .png
        .ok_or_else(|| Failure::Failed(format!("{url} at {viewport}: no screenshot")))?;
    let png = std::fs::read(&png_path).map_err(|e| Failure::Failed(format!("{png_path}: {e}")))?;
    let _ = std::fs::remove_file(&png_path);
    Ok(Render {
        viewport,
        png,
        probe: Report {
            viewport: Some(viewport),
            document_width: raw.document_width,
            clipped: raw.clipped,
            overlaps: raw.overlaps,
            contrast: raw
                .contrast
                .into_iter()
                .map(|c| ContrastSample {
                    element: c.element,
                    ratio: c.ratio,
                    large: c.large,
                })
                .collect(),
            targets: raw
                .targets
                .into_iter()
                .map(|t| Target {
                    element: t.element,
                    width: t.width,
                    height: t.height,
                    inline: t.inline,
                })
                .collect(),
        },
    })
}

/// A performance profile of the milestone 05 protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// 390x844, CPU 4x, 150 ms latency, 1.6 Mbit/s down.
    Phone,
    /// 1440x900, unthrottled.
    Desktop,
}

impl Profile {
    /// The profile's stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Profile::Phone => "phone",
            Profile::Desktop => "desktop",
        }
    }
}

impl Playwright {
    /// Loads `url` cold `runs` times under `profile` and returns one JSON
    /// object per run: `load_ms`, `lcp_ms`, `cls`, `tbt_ms`, `bytes`,
    /// `requests`, or `error`.
    ///
    /// # Errors
    ///
    /// Fails when the driver cannot be started or produces no runs.
    pub fn measure(
        &self,
        url: &str,
        profile: Profile,
        runs: u32,
    ) -> Result<Vec<serde_json::Value>, Failure> {
        let (viewport, throttle) = match profile {
            Profile::Phone => ([390, 844], true),
            Profile::Desktop => ([1440, 900], false),
        };
        let job = json!({
            "playwright": self.module,
            "chrome": self.chrome,
            "url": url,
            "profile": profile.name(),
            "viewport": viewport,
            "throttle": throttle,
            "runs": runs,
        });
        let output = self.drive(PERF, &job, url)?;
        let rows: Vec<serde_json::Value> = output
            .lines()
            .filter(|l| l.starts_with('{'))
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        if rows.is_empty() {
            return Err(Failure::Failed(format!(
                "{url}: the performance driver reported nothing"
            )));
        }
        Ok(rows)
    }

    /// Runs a Node driver on one job under the browser lock and a deadline.
    fn drive(
        &self,
        script_text: &str,
        job: &serde_json::Value,
        url: &str,
    ) -> Result<String, Failure> {
        for (what, path) in [("playwright-core", &self.module), ("Chrome", &self.chrome)] {
            if !path.is_file() {
                return Err(Failure::Unavailable(format!(
                    "no {what} at {}",
                    path.display()
                )));
            }
        }
        std::fs::create_dir_all(&self.scratch).map_err(|e| Failure::Failed(e.to_string()))?;
        let name = unique("drive");
        let script = self.scratch.join(format!("{name}.mjs"));
        std::fs::write(&script, script_text).map_err(|e| Failure::Failed(e.to_string()))?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(BROWSER_LOCK)
            .map_err(|e| Failure::Unavailable(format!("{BROWSER_LOCK}: {e}")))?;
        lock.lock()
            .map_err(|e| Failure::Unavailable(format!("{BROWSER_LOCK}: {e}")))?;
        let out_path = self.scratch.join(format!("{name}.out"));
        let out = std::fs::File::create(&out_path).map_err(|e| Failure::Failed(e.to_string()))?;
        let mut child = Command::new("node")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(out)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Failure::Unavailable(format!("node: {e}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            writeln!(stdin, "{job}").map_err(|e| Failure::Failed(e.to_string()))?;
        }
        let deadline = Instant::now() + DEADLINE;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Failure::TimedOut(format!(
                        "{url}: no result within {}s",
                        DEADLINE.as_secs()
                    )));
                }
                Err(e) => return Err(Failure::Failed(e.to_string())),
            }
        }
        drop(lock);
        let output = std::fs::read_to_string(&out_path).unwrap_or_default();
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&script);
        Ok(output)
    }
}

fn unique(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

impl PageRenderer for Playwright {
    fn render_url(&self, url: &str, viewports: &[Viewport]) -> Result<Vec<Render>, Failure> {
        std::fs::create_dir_all(&self.scratch).map_err(|e| Failure::Failed(e.to_string()))?;
        let name = unique("page");
        let probe = self.scratch.join(format!("{name}-probe.js"));
        std::fs::write(&probe, PROBE).map_err(|e| Failure::Failed(e.to_string()))?;
        let job = json!({
            "playwright": self.module,
            "chrome": self.chrome,
            "probe": probe,
            "url": url,
            "viewports": viewports.iter().map(|v| [v.width, v.height]).collect::<Vec<_>>(),
            "out_prefix": self.scratch.join(&name),
            "settle_ms": self.settle_ms,
        });
        let output = self.drive(SCRIPT, &job, url);
        let _ = std::fs::remove_file(&probe);
        let output = output?;
        let renders: Result<Vec<Render>, Failure> = output
            .lines()
            .filter(|l| l.starts_with('{'))
            .map(|l| parse_line(l, url))
            .collect();
        let renders = renders?;
        if renders.len() == viewports.len() {
            Ok(renders)
        } else {
            Err(Failure::Failed(format!(
                "{url}: {} of {} renders",
                renders.len(),
                viewports.len()
            )))
        }
    }
}
