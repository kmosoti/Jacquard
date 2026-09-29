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
