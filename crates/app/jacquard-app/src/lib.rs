//! The use cases of Jacquard, orchestrating the core over the ports. This
//! layer performs the effects the core returns as data and never depends on
//! an adapter.
//!
//! The cascade, in order, each tier cheaper than the next and only survivors
//! moving on:
//!
//! 1. Render gate: a Swatch that does not build, render or hydrate is rejected; no Judge sees it.
//! 2. Checks: every registered deterministic Check, each with a Receipt. These outrank everything below.
//! 3. Structured local checks: fixed yes/no questions answered by a small local model, treated as a filter with a measured false-positive rate.
//! 4. Judge: pairwise, both orders, rendered evidence only, decomposed rubric.
//! 5. Votes: the owner's blind pairwise preferences.
//!
//! Rankings are computed over 4 and 5; acceptance is decided by 1 and 2.
#![forbid(unsafe_code)]

pub mod admission;

use jacquard_core::check::Outcome;
use jacquard_core::probe::{self, Finding};
use jacquard_core::swatch::SwatchId;
use jacquard_core::viewport::Viewport;
use jacquard_ports::{Answer, Failure, Judge, Render, SwatchSource};
use sha2::{Digest, Sha256};

/// The Swatch's identity: SHA-256 over its entry and every file, in path
/// order, each length-prefixed so that no two sources share an encoding.
#[must_use]
pub fn swatch_id(source: &SwatchSource) -> SwatchId {
    let mut hash = Sha256::new();
    let mut put = |bytes: &[u8]| {
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    };
    put(source.entry.as_bytes());
    for (path, bytes) in &source.files {
        put(path.as_bytes());
        put(bytes);
    }
    SwatchId(hash.finalize().into())
}

/// The probe Checks over every Render: findings per viewport and the
/// overall Outcome, `Passed` only when no viewport has a finding.
#[must_use]
pub fn probe_gate(renders: &[Render]) -> (Outcome, Vec<(Viewport, Vec<Finding>)>) {
    let per_viewport: Vec<(Viewport, Vec<Finding>)> = renders
        .iter()
        .map(|r| (r.viewport, probe::evaluate(&r.probe)))
        .collect();
    let all: Vec<Finding> = per_viewport.iter().flat_map(|(_, f)| f.clone()).collect();
    (probe::outcome(&all), per_viewport)
}

/// Both presentations of one comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// `a` shown first.
    pub a_first: Answer,
    /// `b` shown first.
    pub b_first: Answer,
}

/// Asks the Judge about `a` and `b` in both orders.
///
/// # Errors
///
/// Returns the first [`Failure`] of either presentation.
pub fn compare(judge: &dyn Judge, a: &[Render], b: &[Render]) -> Result<Comparison, Failure> {
    Ok(Comparison {
        a_first: judge.judge(a, b)?,
        b_first: judge.judge(b, a)?,
    })
}
