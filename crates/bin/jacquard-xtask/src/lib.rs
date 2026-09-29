//! Repository tooling for Jacquard: the gates, the check registry and receipts.
//!
//! Every gate is a pure function over data plus a thin collector that reads
//! the repository, so the fixtures under `tests/fixtures/` drive the same
//! function the command runs. A gate that cannot run reports failure.
#![forbid(unsafe_code)]

pub mod finding;
pub mod layers;
pub mod mutants;
pub mod purity;
pub mod receipts;
pub mod registry;
pub mod trust;
pub mod util;

use std::path::{Path, PathBuf};

/// The repository root, fixed at compile time from this crate's location.
#[must_use]
pub fn repo_root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    here.canonicalize().unwrap_or(here)
}
