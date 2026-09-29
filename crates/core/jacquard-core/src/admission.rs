//! Judge admission (ADR-0005), as exact integer arithmetic over recorded
//! Judgments. A pair is judged twice, base first and variant first.
//!
//! | base first | variant first | class |
//! | --- | --- | --- |
//! | First | Second | correct: the base both times |
//! | First | First | flipped: the first slot both times |
//! | Second | Second | flipped: the second slot both times |
//! | Second | First | wrong: the variant both times |
//! | any tie | | wrong: a tie on a known answer |

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::judgment::Verdict;

/// Which side of a pair was shown first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Presentation {
    /// The base first, the variant second.
    BaseFirst,
    /// The variant first, the base second.
    VariantFirst,
}

/// How a pair's two Judgments classify.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    /// The base was preferred in both orders.
    Correct,
    /// The same presentation slot was preferred in both orders.
    Flipped,
    /// Anything else, ties included.
    Wrong,
}

/// Classifies a pair from its two verdicts.
#[must_use]
pub const fn classify(base_first: Verdict, variant_first: Verdict) -> Class {
    match (base_first, variant_first) {
        (Verdict::First, Verdict::Second) => Class::Correct,
        (Verdict::First, Verdict::First) | (Verdict::Second, Verdict::Second) => Class::Flipped,
        _ => Class::Wrong,
    }
}

/// One recorded admission Judgment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    /// The pair's id in the set.
    pub pair: &'a str,
    /// The order it was shown in.
    pub presentation: Presentation,
    /// The Judge's verdict.
    pub verdict: Verdict,
    /// Whether the pinned model answered.
    pub pinned_model: bool,
}

/// The thresholds of ADR-0005, as ratios `numerator / denominator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds {
    /// The smallest admissible set.
    pub minimum_pairs: u32,
    /// Correct pairs over all pairs must be at least this.
    pub accuracy: (u32, u32),
    /// Flipped pairs over all pairs must be at most this.
    pub flip_rate: (u32, u32),
}

/// ADR-0005: at least 20 pairs, accuracy at least 0.90, flips at most 0.10.
pub const ADR_0005: Thresholds = Thresholds {
    minimum_pairs: 20,
    accuracy: (90, 100),
    flip_rate: (10, 100),
};

/// Why a configuration was not admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Shortfall {
    /// The set is smaller than the minimum.
    TooFewPairs,
    /// Accuracy is below the threshold.
    Accuracy,
    /// The flip rate is above the threshold.
    FlipRate,
    /// A Judgment was answered by another model.
    WrongModel,
}

/// The verdict of an admission run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Every threshold met over a complete run.
    Admitted,
    /// A threshold missed.
    Rejected(Vec<Shortfall>),
    /// A pair lacks one of its two Judgments, or has a duplicate.
    Inconclusive,
}

/// The counts and the decision of one admission run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Pairs in the set.
    pub pairs: u32,
    /// Pairs judged correctly in both orders.
    pub correct: u32,
    /// Pairs whose verdict followed the order.
    pub flipped: u32,
    /// Pairs judged wrongly or tied.
    pub wrong: u32,
    /// The decision.
    pub decision: Decision,
}

fn at_least(count: u32, total: u32, (num, den): (u32, u32)) -> bool {
    u64::from(count) * u64::from(den) >= u64::from(num) * u64::from(total)
}

/// Decides admission for a set of `set` pair ids from the recorded entries.
/// The order of `entries` does not matter.
#[must_use]
pub fn decide(set: &[&str], entries: &[Entry<'_>], thresholds: Thresholds) -> Report {
    let mut by_pair: BTreeMap<&str, (Option<Verdict>, Option<Verdict>, u32)> =
        set.iter().map(|id| (*id, (None, None, 0))).collect();
    let mut complete = true;
    let mut wrong_model = false;
    for entry in entries {
        wrong_model |= !entry.pinned_model;
        let Some(slot) = by_pair.get_mut(entry.pair) else {
            complete = false;
            continue;
        };
        let side = match entry.presentation {
            Presentation::BaseFirst => &mut slot.0,
            Presentation::VariantFirst => &mut slot.1,
        };
        if side.replace(entry.verdict).is_some() {
            complete = false;
        }
        slot.2 += 1;
    }
    let (mut correct, mut flipped, mut wrong) = (0, 0, 0);
    for (base_first, variant_first, _) in by_pair.values() {
        match (base_first, variant_first) {
            (Some(a), Some(b)) => match classify(*a, *b) {
                Class::Correct => correct += 1,
                Class::Flipped => flipped += 1,
                Class::Wrong => wrong += 1,
            },
            _ => complete = false,
        }
    }
    let pairs = u32::try_from(by_pair.len()).unwrap_or(u32::MAX);
    let decision = if complete {
        let mut short = Vec::new();
        if pairs < thresholds.minimum_pairs {
            short.push(Shortfall::TooFewPairs);
        }
        if !at_least(correct, pairs, thresholds.accuracy) {
            short.push(Shortfall::Accuracy);
        }
        // flipped / pairs <= num / den, i.e. not (flipped * den > num * pairs).
        if u64::from(flipped) * u64::from(thresholds.flip_rate.1)
            > u64::from(thresholds.flip_rate.0) * u64::from(pairs)
        {
            short.push(Shortfall::FlipRate);
        }
        if wrong_model {
            short.push(Shortfall::WrongModel);
        }
        if short.is_empty() {
            Decision::Admitted
        } else {
            Decision::Rejected(short)
        }
    } else {
        Decision::Inconclusive
    };
    Report {
        pairs,
        correct,
        flipped,
        wrong,
        decision,
    }
}
