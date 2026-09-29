//! A Judgment is a model Judge's pairwise verdict on two Renders presented
//! in a stated order. The Judge sees rendered evidence only: never the
//! generator's log, plan, or claims of success.
//!
//! A comparison of two Swatches is two Judgments, one per presentation
//! order, because order alone moves verdicts. A Judgment never gates; it
//! feeds a ranking. A tie counts as one half to each side.

use crate::swatch::SwatchId;

/// Which of the two presented Renders the Judge preferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Verdict {
    /// The Render shown first.
    First,
    /// The Render shown second.
    Second,
    /// Neither; scored one half each.
    Tie,
}

/// One presentation to the Judge and its verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Judgment {
    /// The Swatch whose Render was shown first.
    pub first: SwatchId,
    /// The Swatch whose Render was shown second.
    pub second: SwatchId,
    /// The Judge's verdict for this order.
    pub verdict: Verdict,
}
