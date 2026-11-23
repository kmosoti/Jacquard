//! The driven ports of Jacquard: traits the application layer calls and
//! adapters implement ([ADR-0004](../../../docs/decisions/ADR-0004-factory-ports.md)).
//! Port methods take and return core types or plain data; no port exposes a
//! technology's types.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path};

use jacquard_core::check::Outcome;
use jacquard_core::judgment::Verdict;
use jacquard_core::probe::Report;
use jacquard_core::viewport::Viewport;

/// Maximum number of files accepted in a single [`SwatchSource`].
pub const MAX_SWATCH_FILES: usize = 512;

/// Maximum total byte size accepted across all files in a [`SwatchSource`].
pub const MAX_SWATCH_BYTES: usize = 32 * 1024 * 1024;

/// One candidate interface's source: relative path to bytes, and the entry
/// page the renderer opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwatchSource {
    /// Files by relative path.
    pub files: BTreeMap<String, Vec<u8>>,
    /// The relative path of the page to open.
    pub entry: String,
}

impl SwatchSource {
    /// Validates that a relative path within a Swatch is strictly normalized
    /// and cannot escape a root directory on Unix or Windows.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::Failed`] if `path` is empty, contains NUL or control
    /// characters, backslashes, `:`/drive prefixes, leading/trailing/empty
    /// segments, `.` or `..` components, or non-normal path components.
    pub fn validate_path(path: &str) -> Result<(), Failure> {
        if path.is_empty() {
            return Err(Failure::Failed("swatch path must not be empty".into()));
        }
        if path.contains('\0') || path.chars().any(char::is_control) {
            return Err(Failure::Failed(format!(
                "swatch path {path:?} contains NUL or control characters"
            )));
        }
        if path.starts_with('/') || path.contains('\\') || path.contains(':') {
            return Err(Failure::Failed(format!(
                "swatch path {path:?} must be relative without '/', '\\', or ':' prefixes"
            )));
        }
        for seg in path.split('/') {
            if seg.is_empty() || seg == "." || seg == ".." {
                return Err(Failure::Failed(format!(
                    "swatch path {path:?} contains invalid segment {seg:?}"
                )));
            }
        }
        let mut comps = Path::new(path).components();
        if !comps.all(|c| matches!(c, Component::Normal(_))) {
            return Err(Failure::Failed(format!(
                "swatch path {path:?} contains non-normal path components"
            )));
        }
        Ok(())
    }

    /// Validates all file paths, total count/byte budgets, and that `entry`
    /// is present in `files`.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::Failed`] if any path is invalid, budgets are exceeded,
    /// or `entry` does not name a file in `files`.
    pub fn validate(&self) -> Result<(), Failure> {
        Self::validate_path(&self.entry)?;
        if self.files.len() > MAX_SWATCH_FILES {
            return Err(Failure::Failed(format!(
                "swatch contains {} files, exceeding limit {MAX_SWATCH_FILES}",
                self.files.len()
            )));
        }
        let mut total_bytes: usize = 0;
        for (path, bytes) in &self.files {
            Self::validate_path(path)?;
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .filter(|&n| n <= MAX_SWATCH_BYTES)
                .ok_or_else(|| {
                    Failure::Failed(format!(
                        "swatch total size exceeds limit {MAX_SWATCH_BYTES} bytes"
                    ))
                })?;
        }
        if !self.files.contains_key(&self.entry) {
            return Err(Failure::Failed(format!(
                "entry {} is not among the files",
                self.entry
            )));
        }
        Ok(())
    }
}

/// A Swatch rendered at one viewport.
#[derive(Clone, Debug, PartialEq)]
pub struct Render {
    /// The viewport.
    pub viewport: Viewport,
    /// A PNG screenshot of the viewport.
    pub png: Vec<u8>,
    /// What the page measured of itself.
    pub probe: Report,
}

/// Why an effect did not happen. `Unavailable` means the environment lacks
/// what the adapter needs, and is reported as such, never as a failure of
/// the thing being examined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// A tool, binary or credential the adapter needs is missing.
    Unavailable(String),
    /// The effect ran and did not produce a usable result.
    Failed(String),
    /// The effect did not finish in time.
    TimedOut(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Unavailable(m) => write!(f, "environment unavailable: {m}"),
            Failure::Failed(m) => write!(f, "failed: {m}"),
            Failure::TimedOut(m) => write!(f, "timed out: {m}"),
        }
    }
}

impl std::error::Error for Failure {}

/// What a Loom wove: the edited copy's changes, as a patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Weave {
    /// Paths the Loom changed, relative to the copy's root.
    pub changed: Vec<String>,
    /// A unified diff from the base to the edited copy.
    pub patch: String,
    /// The model that actually wove, as the transport reported it.
    pub model: String,
    /// The Loom's own account of what it changed; data, not evidence.
    pub summary: String,
}

/// Weaves a change to a Product from a Draft (ADR-0008). Proposes; never
/// verifies; never sees a Judgment.
pub trait Loom {
    /// Copies `base` to `out`, lets the generator edit the copy under the
    /// Draft, and returns what changed.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when nothing was woven.
    fn weave(
        &self,
        draft: &str,
        base: &std::path::Path,
        out: &std::path::Path,
    ) -> Result<Weave, Failure>;
}

/// Renders a Swatch into Renders at viewports.
pub trait Renderer {
    /// Renders the Swatch at each viewport, in order.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when the Swatch does not render at any viewport;
    /// that is the render gate's rejection.
    fn render(&self, swatch: &SwatchSource, viewports: &[Viewport])
    -> Result<Vec<Render>, Failure>;
}

/// Renders a live page, served by a Product, at viewports (ADR-0008).
pub trait PageRenderer {
    /// Renders the page at `url` at each viewport, in order.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when the page did not load or settle.
    fn render_url(&self, url: &str, viewports: &[Viewport]) -> Result<Vec<Render>, Failure>;
}

/// Runs one registered Check.
pub trait Checker {
    /// Runs the Check and returns its Outcome.
    fn check(&self, check_id: &str) -> Outcome;
}

/// A Judge's answer for one presentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// The overall verdict.
    pub verdict: Verdict,
    /// The verdict per rubric dimension.
    pub dimensions: BTreeMap<String, Verdict>,
    /// The Judge's short reason.
    pub reason: String,
    /// The model that actually answered, as the transport reported it.
    pub model: String,
}

/// Compares two sets of Renders presented in a stated order.
pub trait Judge {
    /// The model identifier this Judge is pinned to.
    fn pinned_model(&self) -> &str;

    /// A stable hash of the rubric, for admission records.
    fn rubric_hash(&self) -> String;

    /// Judges `first` against `second`, shown in that order. The Judge sees
    /// the screenshots and the rubric, nothing else.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when no verdict could be read.
    fn judge(&self, first: &[Render], second: &[Render]) -> Result<Answer, Failure>;
}

/// Appends records and reads them back. Never rewrites.
pub trait Ledger {
    /// Appends one JSON record to the stream named `kind`.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when the record was not stored.
    fn append(&self, kind: &str, record: &str) -> Result<(), Failure>;

    /// Reads every record of the stream named `kind`, oldest first.
    ///
    /// # Errors
    ///
    /// Returns a [`Failure`] when the stream cannot be read.
    fn read(&self, kind: &str) -> Result<Vec<String>, Failure>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_source() -> SwatchSource {
        SwatchSource {
            entry: "index.html".into(),
            files: BTreeMap::from([
                ("index.html".into(), b"<p>ok</p>".to_vec()),
                ("css/app.css".into(), b"p{}".to_vec()),
            ]),
        }
    }

    #[test]
    fn valid_swatch_source_passes() {
        assert!(valid_source().validate().is_ok());
    }

    #[test]
    fn invalid_paths_and_missing_entry_are_rejected() {
        for bad in [
            "",
            ".",
            "..",
            "../evil.html",
            "a/../b.html",
            "a/./b.html",
            "/etc/passwd",
            "//host/share/a",
            "a//b.html",
            "a/",
            r"a\b.html",
            r"..\evil.html",
            "C:foo.html",
            "C:/foo.html",
            r"\\?\C:\foo.html",
            "index.html\0.png",
            "bad\npath.html",
        ] {
            let mut s = valid_source();
            s.files.insert(bad.into(), b"x".to_vec());
            assert!(s.validate().is_err(), "expected rejection for path {bad:?}");
        }
        let mut missing = valid_source();
        missing.entry = "missing.html".into();
        assert!(missing.validate().is_err());
    }
}
