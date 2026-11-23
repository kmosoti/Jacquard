//! The Chrome renderer (ADR-0004): Chrome's own `--headless --screenshot`
//! and `--dump-dom`, one process each per viewport, served over an ephemeral
//! `127.0.0.1:0` loopback listener with an isolated `--user-data-dir`, blocked
//! non-loopback egress, and an outer-realm layout probe bound by a per-render
//! 128-bit nonce. The probe measures; `jacquard-core` decides.
//!
//! The page is framed in an iframe of exactly the viewport's size, in both
//! processes, because `--dump-dom` ignores `--window-size` (on Chrome 151 a
//! 390 by 844 request laid out at 500 by 757) and the probe must measure the
//! layout the screenshot shows. The probe reports the viewport it received,
//! and a render whose viewport differs from the request is refused.
//!
//! The binary is `JACQUARD_CHROME`, else the cached Chrome for Testing.
//! Sandbox stays enabled by default unless `JACQUARD_ALLOW_NO_SANDBOX=1`.
//! Every launch holds the machine-wide browser lock and is killed after
//! [`LAUNCH_DEADLINE`].
#![forbid(unsafe_code)]

mod hmac;
mod sandbox;
mod server;
mod wrapper;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use jacquard_core::probe::{ContrastSample, Report, Target};
use jacquard_core::viewport::Viewport;
use jacquard_ports::{Failure, Render, Renderer, SwatchSource};
use serde::Deserialize;

pub use hmac::sign_probe_hex;
use sandbox::{
    TempDir, create_private_dir, ensure_no_secrets, generate_nonce, generate_secret_bytes,
    materialize_contained,
};
use server::LoopbackServer;
pub use wrapper::{PROBE_ID_PREFIX, probe_wrapper, shot_wrapper};

/// The probe script evaluated from the outer wrapper or Playwright.
pub const PROBE: &str = include_str!("probe.js");

const BROWSER_LOCK: &str = "/tmp/jacquard-browser.lock";

/// A name no other render or launch in this process uses, so that
/// concurrent calls never share a directory or an output file.
fn unique(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// How long one Chrome launch may take before it is killed.
pub const LAUNCH_DEADLINE: Duration = Duration::from_secs(60);

/// Renders with a Chrome binary.
#[derive(Clone, Debug)]
pub struct Chrome {
    binary: PathBuf,
    scratch: PathBuf,
    budget_ms: u32,
}

impl Chrome {
    /// A renderer using `JACQUARD_CHROME` or the cached Chrome for Testing,
    /// writing scratch artifacts under `scratch`.
    #[must_use]
    pub fn from_env(scratch: &Path) -> Self {
        let binary = std::env::var_os("JACQUARD_CHROME").map_or_else(
            || {
                let home = std::env::var_os("HOME").unwrap_or_default();
                Path::new(&home).join(".cache/ms-playwright/chromium-1234/chrome-linux64/chrome")
            },
            PathBuf::from,
        );
        Self {
            binary,
            scratch: scratch.to_path_buf(),
            budget_ms: 3000,
        }
    }

    /// The binary this renderer launches.
    #[must_use]
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Builds the Chrome CLI arguments for `viewport` and `user_data_dir`,
    /// keeping the sandbox enabled unless `allow_no_sandbox` is true and
    /// blocking all non-loopback network egress.
    #[must_use]
    pub fn isolation_args(
        &self,
        viewport: Viewport,
        user_data_dir: &Path,
        allow_no_sandbox: bool,
    ) -> Vec<String> {
        let mut args = vec!["--headless".into()];
        if allow_no_sandbox {
            args.push("--no-sandbox".into());
        }
        args.extend([
            "--disable-gpu".into(),
            "--hide-scrollbars".into(),
            "--mute-audio".into(),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--disable-background-networking".into(),
            "--disable-extensions".into(),
            "--proxy-server=direct://".into(),
            "--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE 127.0.0.1 , EXCLUDE localhost".into(),
            format!("--user-data-dir={}", user_data_dir.display()),
            "--force-device-scale-factor=1".into(),
            "--font-render-hinting=none".into(),
            "--run-all-compositor-stages-before-draw".into(),
            format!("--virtual-time-budget={}", self.budget_ms),
            format!("--window-size={},{}", viewport.width, viewport.height),
        ]);
        args
    }

    fn launch(&self, viewport: Viewport, extra: &[String], url: &str) -> Result<Vec<u8>, Failure> {
        create_private_dir(&self.scratch)?;
        let launch = unique("chrome");
        let user_data = TempDir(self.scratch.join(format!("{launch}-profile")));
        create_private_dir(&user_data.0)?;
        let allow_no_sandbox =
            std::env::var_os("JACQUARD_ALLOW_NO_SANDBOX").is_some_and(|v| v == "1");
        let mut args = self.isolation_args(viewport, &user_data.0, allow_no_sandbox);
        args.extend_from_slice(extra);
        args.push(url.to_string());

        // Held for the whole launch: several software-rendered browsers at
        // once have taken this machine down.
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(BROWSER_LOCK)
            .map_err(|e| Failure::Unavailable(format!("{BROWSER_LOCK}: {e}")))?;
        lock.lock()
            .map_err(|e| Failure::Unavailable(format!("{BROWSER_LOCK}: {e}")))?;

        let out_path = self.scratch.join(format!("{launch}.out"));
        let err_path = self.scratch.join(format!("{launch}.err"));
        let file = |p: &Path| std::fs::File::create(p).map_err(|e| Failure::Failed(e.to_string()));
        let mut child = Command::new(&self.binary)
            .args(&args)
            .stdout(file(&out_path)?)
            .stderr(file(&err_path)?)
            .spawn()
            .map_err(|e| Failure::Unavailable(format!("{}: {e}", self.binary.display())))?;
        let deadline = Instant::now() + LAUNCH_DEADLINE;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&out_path);
                    let _ = std::fs::remove_file(&err_path);
                    return Err(Failure::TimedOut(format!(
                        "chrome did not finish at {viewport} within {}s",
                        LAUNCH_DEADLINE.as_secs()
                    )));
                }
                Err(e) => return Err(Failure::Failed(e.to_string())),
            }
        };
        drop(lock);
        let stdout = std::fs::read(&out_path).unwrap_or_default();
        let stderr = std::fs::read_to_string(&err_path).unwrap_or_default();
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&err_path);
        if !status.success() {
            return Err(Failure::Failed(format!(
                "chrome exited {:?}: {}",
                status.code(),
                stderr.trim()
            )));
        }
        Ok(stdout)
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

fn is_valid_nonce(nonce: &str) -> bool {
    nonce.len() == 32
        && nonce
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Reads and HMAC-verifies the unforgeable nonced probe report out of a dumped outer DOM.
///
/// # Errors
///
/// Returns a failure when `nonce` is invalid, missing, duplicated, outside
/// `<head>`, HMAC-SHA256 verification fails, or the report mismatches `viewport`.
pub fn read_probe(
    dom: &str,
    viewport: Viewport,
    nonce: &str,
    mac_key: &[u8; 32],
) -> Result<Report, Failure> {
    if !is_valid_nonce(nonce) {
        return Err(Failure::Failed("invalid probe nonce".into()));
    }
    let (sig_hex, json_slice) = hmac::extract_probe_tag(dom, nonce)?;
    let mac_msg = format!("{PROBE_ID_PREFIX}{nonce}:{json_slice}");
    if !hmac::verify_hmac_hex(mac_key, mac_msg.as_bytes(), &sig_hex) {
        return Err(Failure::Failed(format!(
            "probe HMAC verification failed at {viewport}"
        )));
    }
    let raw: RawReport = serde_json::from_str(json_slice)
        .map_err(|e| Failure::Failed(format!("probe report: {e}")))?;
    if (raw.viewport_width, raw.viewport_height) != (viewport.width, viewport.height) {
        return Err(Failure::Failed(format!(
            "asked for {viewport}, the page received {}x{}",
            raw.viewport_width, raw.viewport_height
        )));
    }
    Ok(Report {
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
    })
}

impl Renderer for Chrome {
    fn render(
        &self,
        swatch: &SwatchSource,
        viewports: &[Viewport],
    ) -> Result<Vec<Render>, Failure> {
        if !self.binary.is_file() {
            return Err(Failure::Unavailable(format!(
                "no Chrome at {}",
                self.binary.display()
            )));
        }
        swatch.validate()?;
        let nonce = generate_nonce()?;
        let mac_key = generate_secret_bytes()?;
        ensure_no_secrets(swatch, &[&nonce])?;

        create_private_dir(&self.scratch)?;
        let work_dir = TempDir(self.scratch.join(unique("swatch")));
        create_private_dir(&work_dir.0)?;
        materialize_contained(swatch, &work_dir.0.join("swatch"))?;

        let server = LoopbackServer::start(swatch, viewports, &nonce, &mac_key)?;
        let mut renders = Vec::new();
        for viewport in viewports {
            let url = server.shot_url(*viewport);
            let probe_url = server.probe_url(*viewport);
            let shot = work_dir.0.join(format!("shot-{viewport}.png"));
            self.launch(
                *viewport,
                &[format!("--screenshot={}", shot.display())],
                &url,
            )?;
            let png = std::fs::read(&shot)
                .map_err(|e| Failure::Failed(format!("no screenshot at {viewport}: {e}")))?;
            let dom = self.launch(*viewport, &["--dump-dom".to_string()], &probe_url)?;
            let probe = read_probe(&String::from_utf8_lossy(&dom), *viewport, &nonce, &mac_key)?;
            renders.push(Render {
                viewport: *viewport,
                png,
                probe,
            });
        }
        Ok(renders)
    }
}
