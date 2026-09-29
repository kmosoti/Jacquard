//! Ranking (ADR-0006): Bradley–Terry strengths with a phantom prior,
//! estimated by Hunter's minorization–maximization iteration, with 95%
//! percentile-bootstrap intervals from a seeded generator and a floor below
//! which a Swatch is estimated but unranked. Relative, never absolute.
//!
//! Comparisons are aggregated into integer half-unit counts, and resampled
//! from a canonical order, so the result never depends on the order the
//! records arrive in.

use alloc::vec;
use alloc::vec::Vec;

use crate::judgment::Verdict;

/// One comparison between items `first` and `second`, shown in that order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Comparison {
    /// The index of the item shown first.
    pub first: usize,
    /// The index of the item shown second.
    pub second: usize,
    /// The verdict.
    pub verdict: Verdict,
}

/// A Swatch's estimate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    /// Strength relative to the phantom, which is 1.
    pub strength: f64,
    /// The 2.5th percentile of the bootstrap strengths.
    pub low: f64,
    /// The 97.5th percentile of the bootstrap strengths.
    pub high: f64,
    /// Real comparisons this item took part in.
    pub comparisons: u32,
    /// The rank, 1 for the strongest, or `None` below the floor.
    pub rank: Option<u32>,
}

/// ADR-0006's floor: fewer comparisons than this is unranked.
pub const FLOOR: u32 = 15;

/// ADR-0006's bootstrap resample count.
pub const RESAMPLES: usize = 200;

const TOLERANCE: f64 = 1e-12;
const MAX_ITERATIONS: usize = 10_000;

/// Win counts in half-units: `wins[i][j]` is twice the score of `i` against
/// `j`, a win counting 2 and a tie 1.
fn aggregate(items: usize, comparisons: &[Comparison]) -> Vec<Vec<u32>> {
    let mut wins = vec![vec![0u32; items]; items];
    for c in comparisons {
        if c.first >= items || c.second >= items || c.first == c.second {
            continue;
        }
        match c.verdict {
            Verdict::First => wins[c.first][c.second] += 2,
            Verdict::Second => wins[c.second][c.first] += 2,
            Verdict::Tie => {
                wins[c.first][c.second] += 1;
                wins[c.second][c.first] += 1;
            }
        }
    }
    wins
}

fn relative_change(old: f64, new: f64) -> f64 {
    let d = if new > old { new - old } else { old - new };
    d / old
}

/// Maximum-likelihood strengths for half-unit win counts, with one virtual
/// win and one virtual loss per item against a phantom of strength 1.
#[must_use]
pub fn strengths(wins: &[Vec<u32>]) -> Vec<f64> {
    let n = wins.len();
    // Half-units throughout: the prior is 2 each way against the phantom.
    let total: Vec<f64> = (0..n)
        .map(|i| f64::from(wins[i].iter().sum::<u32>() + 2))
        .collect();
    let games: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| f64::from(wins[i][j] + wins[j][i])).collect())
        .collect();
    let mut p = vec![1.0f64; n];
    for _ in 0..MAX_ITERATIONS {
        let mut next = vec![0.0f64; n];
        let mut largest = 0.0f64;
        for i in 0..n {
            // Four half-unit games against the phantom (two wins, two losses).
            let mut denominator = 4.0 / (p[i] + 1.0);
            for j in 0..n {
                if j != i && games[i][j] > 0.0 {
                    denominator += games[i][j] / (p[i] + p[j]);
                }
            }
            next[i] = total[i] / denominator;
            let change = relative_change(p[i], next[i]);
            if change > largest {
                largest = change;
            }
        }
        p = next;
        if largest < TOLERANCE {
            break;
        }
    }
    p
}

/// `SplitMix64`: a deterministic generator, seeded by the caller.
#[derive(Clone, Debug)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A generator from a seed.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value below `bound`, which must be positive.
    pub fn below(&mut self, bound: usize) -> usize {
        // Modulo bias is below 2^-40 for any bound this kernel sees.
        usize::try_from(self.next_u64() % (bound as u64)).unwrap_or(0)
    }
}

fn percentile(sorted: &[f64], per_mille: usize) -> f64 {
    let at = (sorted.len().saturating_sub(1) * per_mille) / 1000;
    sorted[at]
}

/// Ranks `items` Swatches from their comparisons. Deterministic in the
/// comparisons as a multiset and in `seed`.
#[must_use]
pub fn rank(items: usize, comparisons: &[Comparison], seed: u64) -> Vec<Estimate> {
    let mut canonical: Vec<Comparison> = comparisons
        .iter()
        .copied()
        .filter(|c| c.first < items && c.second < items && c.first != c.second)
        .collect();
    canonical.sort_unstable();
    let point = strengths(&aggregate(items, &canonical));
    let mut counts = vec![0u32; items];
    for c in &canonical {
        counts[c.first] += 1;
        counts[c.second] += 1;
    }
    let mut samples: Vec<Vec<f64>> = vec![Vec::with_capacity(RESAMPLES); items];
    if !canonical.is_empty() {
        let mut rng = SplitMix64::new(seed);
        let mut resample = Vec::with_capacity(canonical.len());
        for _ in 0..RESAMPLES {
            resample.clear();
            for _ in 0..canonical.len() {
                resample.push(canonical[rng.below(canonical.len())]);
            }
            for (i, s) in strengths(&aggregate(items, &resample))
                .into_iter()
                .enumerate()
            {
                samples[i].push(s);
            }
        }
    }
    let mut estimates: Vec<Estimate> = (0..items)
        .map(|i| {
            let mut s = core::mem::take(&mut samples[i]);
            s.sort_unstable_by(f64::total_cmp);
            let (low, high) = if s.is_empty() {
                (point[i], point[i])
            } else {
                (percentile(&s, 25), percentile(&s, 975))
            };
            Estimate {
                strength: point[i],
                low,
                high,
                comparisons: counts[i],
                rank: None,
            }
        })
        .collect();
    let mut order: Vec<usize> = (0..items).filter(|&i| counts[i] >= FLOOR).collect();
    order.sort_by(|&a, &b| point[b].total_cmp(&point[a]).then(a.cmp(&b)));
    for (position, i) in order.into_iter().enumerate() {
        estimates[i].rank = Some(u32::try_from(position + 1).unwrap_or(u32::MAX));
    }
    estimates
}
