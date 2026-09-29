//! A Ranking is a Bradley–Terry estimate over Judgments and Votes, with a
//! confidence interval per Swatch and a floor on comparisons below which a
//! Swatch is unranked. Rankings are relative; no absolute score is ever a
//! claim.
//!
//! The estimator lands with the semantic-kernels milestone, with an
//! independently written reference and metamorphic relations stated first:
//! permuting the order of comparisons does not change the ranking; adding a
//! tie does not reorder two Swatches.
