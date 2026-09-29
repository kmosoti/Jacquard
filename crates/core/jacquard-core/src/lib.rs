//! The domain of Jacquard, a factory that turns design intent into
//! interfaces and accepts them on executable evidence.
//!
//! This crate decides what things mean from explicit inputs only. It has no
//! dependencies, is `no_std`, and returns effects as data. The vocabulary is
//! fixed in `docs/glossary.md`; one concept, one name:
//!
//! - A [`draft::Draft`] is compiled design intent, the artifact a Loom weaves from.
//! - A [`swatch::Swatch`] is one candidate interface, identified by the hash of its source.
//! - A [`check::Outcome`] is what a deterministic Check reports, and the only kind of verdict that gates.
//! - A [`judgment::Judgment`] is a model's pairwise verdict on two renders, in a stated order. It never gates.
//! - A [`vote::Vote`] is the owner's pairwise preference.
//! - [`rank`] turns judgments and votes into a ranking with intervals.
//! - [`probe`] decides the layout Checks from what the page measured of itself.
//! - [`admission`] decides whether a Judge configuration may be listened to.
//!
//! The repository is at the skeleton stage: modules carry their contract and
//! the types that pin the evidence rule. Algorithms land with their milestone.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod admission;
pub mod check;
pub mod draft;
pub mod judgment;
pub mod probe;
pub mod rank;
pub mod swatch;
pub mod viewport;
pub mod vote;
