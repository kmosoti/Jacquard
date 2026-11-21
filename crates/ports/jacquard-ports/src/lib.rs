//! The driven ports of Jacquard: traits the application layer calls and
//! adapters implement ([ADR-0004](../../../docs/decisions/ADR-0004-factory-ports.md)).
//! Port methods take and return core types or plain data; no port exposes a
//! technology's types.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use jacquard_core::check::Outcome;
use jacquard_core::judgment::Verdict;
use jacquard_core::probe::Report;
use jacquard_core::viewport::Viewport;

/// One candidate interface's source: relative path to bytes, and the entry
/// page the renderer opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwatchSource {
    /// Files by relative path.
    pub files: BTreeMap<String, Vec<u8>>,
    /// The relative path of the page to open.
    pub entry: String,
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
