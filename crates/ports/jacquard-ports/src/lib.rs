//! The driven ports of Jacquard: traits the application layer calls and
//! adapters implement. A port exists only where an effect crosses a
//! meaningful boundary.
//!
//! Planned ports, one per effect:
//!
//! - `Loom`: weave a Swatch from a Draft and a seed. A generator; it proposes and never verifies.
//! - `Renderer`: render a Swatch into Renders at the audit viewports, or report that it does not render.
//! - `Checker`: run one registered Check and return its Outcome with the command, revision and output hash.
//! - `Judge`: return one Judgment for two Renders in the presented order, from rendered evidence only.
//! - `Ledger`: append Receipts, Judgments and Votes; read them back for ranking.
//!
//! No generic repository traits, service locators or event buses. Traits
//! arrive with the milestone that first needs them.
#![forbid(unsafe_code)]
