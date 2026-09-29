//! The Chrome renderer (ADR-0004): Chrome's own `--headless --screenshot`
//! and `--dump-dom`, one process each per viewport, with a layout probe
//! injected before `</body>`. The probe measures; `jacquard-core` decides.
//!
//! The page is framed in an iframe of exactly the viewport's size, in both
//! processes, because `--dump-dom` ignores `--window-size` (on Chrome 151 a
//! 390 by 844 request laid out at 500 by 757) and the probe must measure the
//! layout the screenshot shows. The probe reports the viewport it received,
//! and a render whose viewport differs from the request is refused.
//!
//! The binary is `JACQUARD_CHROME`, else the cached Chrome for Testing.
//! Every launch holds the machine-wide browser lock and is killed after
//! [`LAUNCH_DEADLINE`].
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use jacquard_core::probe::{ContrastSample, Report, Target};
use jacquard_core::viewport::Viewport;
use jacquard_ports::{Failure, Render, Renderer, SwatchSource};
use serde::Deserialize;

/// The probe script injected into every page.
pub const PROBE: &str = include_str!("probe.js");

/// The id of the element the probe writes its report into.
pub const PROBE_ID: &str = "jacquard-probe";

const BROWSER_LOCK: &str = "/tmp/installation-browser.lock";

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
    /// writing pages under `scratch`.
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

    fn launch(&self, viewport: Viewport, extra: &[String], url: &str) -> Result<Vec<u8>, Failure> {
        let mut args: Vec<String> = vec![
            "--headless".into(),
            "--no-sandbox".into(),
            "--disable-gpu".into(),
            "--hide-scrollbars".into(),
            "--mute-audio".into(),
            "--no-first-run".into(),
            "--force-device-scale-factor=1".into(),
            "--font-render-hinting=none".into(),
            "--run-all-compositor-stages-before-draw".into(),
            format!("--virtual-time-budget={}", self.budget_ms),
            format!("--window-size={},{}", viewport.width, viewport.height),
        ];
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
        std::fs::create_dir_all(&self.scratch).map_err(|e| Failure::Failed(e.to_string()))?;
        let launch = unique("chrome");
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

const FRAME: &str = "__jacquard_frame";

/// The page that frames the entry at exactly the viewport's size and
/// receives the probe's report.
#[must_use]
pub fn wrapper(entry: &str, viewport: Viewport) -> String {
    let (w, h) = (viewport.width, viewport.height);
    let src = entry.replace('"', "%22");
    let listen = format!(
        "addEventListener(\"message\",function(e){{if(typeof e.data===\"string\"&&e.data.indexOf(\"jacquard-probe:\")===0){{\
         var t=document.createElement(\"script\");t.type=\"application/json\";t.id=\"{PROBE_ID}\";t.textContent=e.data.slice(15);\
         document.head.appendChild(t);}}}});"
    );
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><script>{listen}</script>\
         <style>html,body{{margin:0;padding:0;overflow:hidden;background:#fff}}\
         iframe{{display:block;border:0;width:{w}px;height:{h}px}}</style></head>\
         <body><iframe src=\"{src}\"></iframe></body></html>"
    )
}

/// Injects the probe before the last `</body>`, or at the end.
#[must_use]
pub fn inject(html: &str) -> String {
    let tag = format!("<script>{PROBE}</script>");
    match html.rfind("</body>") {
        Some(at) => format!("{}{tag}{}", &html[..at], &html[at..]),
        None => format!("{html}{tag}"),
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

/// Reads the probe report out of a dumped DOM.
///
/// # Errors
///
/// Returns a failure when the page never wrote a report, which means the
/// page did not finish loading within the budget or broke the script.
pub fn read_probe(dom: &str, viewport: Viewport) -> Result<Report, Failure> {
    let open = format!("id=\"{PROBE_ID}\">");
    let start = dom.find(&open).map(|at| at + open.len()).ok_or_else(|| {
        Failure::Failed(format!(
            "no probe report at {viewport}: the page did not settle"
        ))
    })?;
    let end = dom[start..]
        .find("</script>")
        .map(|at| start + at)
        .ok_or_else(|| Failure::Failed("unterminated probe report".into()))?;
    let raw: RawReport = serde_json::from_str(&dom[start..end])
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
        let dir = self.scratch.join(unique("swatch"));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, bytes) in &swatch.files {
            let file = dir.join(path);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Failure::Failed(e.to_string()))?;
            }
            let bytes = if *path == swatch.entry {
                inject(&String::from_utf8_lossy(bytes)).into_bytes()
            } else {
                bytes.clone()
            };
            std::fs::write(&file, bytes).map_err(|e| Failure::Failed(e.to_string()))?;
        }
        let entry = dir.join(&swatch.entry);
        if !entry.is_file() {
            return Err(Failure::Failed(format!(
                "entry {} is not among the files",
                swatch.entry
            )));
        }
        let mut renders = Vec::new();
        for viewport in viewports {
            let frame = dir.join(format!("{FRAME}-{viewport}.html"));
            std::fs::write(&frame, wrapper(&swatch.entry, *viewport))
                .map_err(|e| Failure::Failed(e.to_string()))?;
            let url = format!("file://{}", frame.display());
            let shot = dir.join(format!("shot-{viewport}.png"));
            self.launch(
                *viewport,
                &[format!("--screenshot={}", shot.display())],
                &url,
            )?;
            let png = std::fs::read(&shot)
                .map_err(|e| Failure::Failed(format!("no screenshot at {viewport}: {e}")))?;
            let dom = self.launch(*viewport, &["--dump-dom".to_string()], &url)?;
            let probe = read_probe(&String::from_utf8_lossy(&dom), *viewport)?;
            renders.push(Render {
                viewport: *viewport,
                png,
                probe,
            });
        }
        let _ = std::fs::remove_dir_all(&dir);
        Ok(renders)
    }
}
