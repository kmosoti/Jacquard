//! A Vote is the owner's pairwise preference between two Renders. Votes are
//! the ground truth a Judge is calibrated against and the data a
//! personalised preference model learns from. They are recorded blind, in a
//! stated order, like Judgments.

use crate::judgment::Verdict;
use crate::swatch::SwatchId;

/// One blind comparison by the owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Vote {
    /// The Swatch whose Render was shown first.
    pub first: SwatchId,
    /// The Swatch whose Render was shown second.
    pub second: SwatchId,
    /// The owner's preference for this order.
    pub verdict: Verdict,
}
