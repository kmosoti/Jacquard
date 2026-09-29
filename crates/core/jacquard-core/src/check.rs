//! A Check is a deterministic examination of a Swatch or its Renders by a
//! registered command: rendering, formatting, compilation, accessibility,
//! performance budgets, layout probes, token lint. Its [`Outcome`] is the
//! only kind of verdict that gates acceptance.
//!
//! A Check counts when a Receipt says it ran. `Passed` is recordable only
//! for a registered Check that was executed; a Check that did not run is
//! `NotRun`, a timeout is `Inconclusive`, and a missing tool is
//! `EnvironmentUnavailable`. None of those is a pass.

/// What one run of a Check reports. Only [`Outcome::Passed`] supports the
/// word "passed"; every other variant is reported as itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The registered command ran and exited as expected.
    Passed,
    /// The registered command ran and did not exit as expected.
    Failed,
    /// The Check was not executed.
    NotRun,
    /// The Check was stopped before it finished.
    Interrupted,
    /// The Check ran but its result cannot be read as pass or fail.
    Inconclusive,
    /// A tool or resource the Check requires was missing.
    EnvironmentUnavailable,
}

impl Outcome {
    /// Whether this outcome supports the claim "passed".
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Outcome::Passed)
    }
}
