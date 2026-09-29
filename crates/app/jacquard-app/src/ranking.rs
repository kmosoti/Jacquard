//! Ranking Swatches from recorded comparisons (ADR-0006). The ledger stores
//! comparisons by Swatch id; the core ranks by index; this maps between them.

use std::collections::BTreeMap;

use jacquard_core::judgment::Verdict;
use jacquard_core::rank::{self, Comparison, Estimate};

/// One recorded comparison between two Swatches, by id, in presentation order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    /// The id of the Swatch shown first.
    pub first: String,
    /// The id of the Swatch shown second.
    pub second: String,
    /// The verdict.
    pub verdict: Verdict,
}

/// A ranked Swatch.
#[derive(Clone, Debug, PartialEq)]
pub struct Ranked {
    /// The Swatch id.
    pub id: String,
    /// Its estimate.
    pub estimate: Estimate,
}

/// Ranks every Swatch named in the comparisons, strongest first, unranked
/// Swatches last in id order.
#[must_use]
pub fn rank(recorded: &[Recorded], seed: u64) -> Vec<Ranked> {
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for r in recorded {
        for id in [r.first.as_str(), r.second.as_str()] {
            let next = index.len();
            index.entry(id).or_insert(next);
        }
    }
    // Indices follow sorted id order, so the result does not depend on which
    // Swatch was named first in the records.
    for (position, value) in index.values_mut().enumerate() {
        *value = position;
    }
    let comparisons: Vec<Comparison> = recorded
        .iter()
        .map(|r| Comparison {
            first: index[r.first.as_str()],
            second: index[r.second.as_str()],
            verdict: r.verdict,
        })
        .collect();
    let estimates = rank::rank(index.len(), &comparisons, seed);
    let mut out: Vec<Ranked> = index
        .keys()
        .zip(estimates)
        .map(|(id, estimate)| Ranked {
            id: (*id).to_string(),
            estimate,
        })
        .collect();
    out.sort_by(|a, b| match (a.estimate.rank, b.estimate.rank) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });
    out
}

/// The display rating, `400 log10 p`: formatting, outside the core.
#[must_use]
pub fn rating(strength: f64) -> f64 {
    400.0 * strength.log10()
}
