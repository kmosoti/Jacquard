//! Admission use cases (ADR-0005): verify that the set's known answers are
//! known, then run the Judge over every pair in both orders and decide.

use jacquard_core::admission::{self, ADR_0005, Entry, Presentation, Report};
use jacquard_core::probe::{self, Rule};
use jacquard_ports::{Answer, Failure, Judge, Render};

/// One pair of the admission set, rendered.
#[derive(Clone, Debug)]
pub struct Pair {
    /// The pair's id.
    pub id: String,
    /// The rule the defect must trip.
    pub rule: Rule,
    /// The base's Renders.
    pub base: Vec<Render>,
    /// The variant's Renders.
    pub variant: Vec<Render>,
}

fn trips(renders: &[Render], rule: Rule) -> bool {
    renders
        .iter()
        .flat_map(|r| probe::evaluate(&r.probe))
        .any(|f| f.rule == rule)
}

/// Checks that a pair's answer is known: the rule passes on the base and
/// fails on the variant.
///
/// # Errors
///
/// Returns why the pair is not a known answer.
pub fn verify(pair: &Pair) -> Result<(), String> {
    let rule = pair.rule.name();
    match (
        trips(&pair.base, pair.rule),
        trips(&pair.variant, pair.rule),
    ) {
        (false, true) => Ok(()),
        (true, _) => Err(format!("{}: the base already fails {rule}", pair.id)),
        (false, false) => Err(format!("{}: the variant does not fail {rule}", pair.id)),
    }
}

/// One Judgment of the admission run, or why there is none.
#[derive(Clone, Debug)]
pub struct Recorded {
    /// The pair's id.
    pub pair: String,
    /// The order it was shown in.
    pub presentation: Presentation,
    /// The Judge's answer, or the failure.
    pub answer: Result<Answer, Failure>,
}

/// The whole run: every recorded Judgment and the decision.
#[derive(Clone, Debug)]
pub struct Run {
    /// Every presentation, in the order asked.
    pub recorded: Vec<Recorded>,
    /// The kernel's report over the answered Judgments.
    pub report: Report,
}

/// Asks the Judge about every pair in both orders and decides admission
/// under ADR-0005. `progress` is called after each Judgment. A failed
/// Judgment leaves its pair incomplete, so the run is inconclusive.
pub fn run(judge: &dyn Judge, pairs: &[Pair], mut progress: impl FnMut(&Recorded)) -> Run {
    let mut recorded = Vec::new();
    for pair in pairs {
        for presentation in [Presentation::BaseFirst, Presentation::VariantFirst] {
            let (first, second) = match presentation {
                Presentation::BaseFirst => (&pair.base, &pair.variant),
                Presentation::VariantFirst => (&pair.variant, &pair.base),
            };
            let entry = Recorded {
                pair: pair.id.clone(),
                presentation,
                answer: judge.judge(first, second),
            };
            progress(&entry);
            recorded.push(entry);
        }
    }
    let set: Vec<&str> = pairs.iter().map(|p| p.id.as_str()).collect();
    let entries: Vec<Entry<'_>> = recorded
        .iter()
        .filter_map(|r| {
            let answer = r.answer.as_ref().ok()?;
            Some(Entry {
                pair: &r.pair,
                presentation: r.presentation,
                verdict: answer.verdict,
                pinned_model: answer.model == judge.pinned_model(),
            })
        })
        .collect();
    let report = admission::decide(&set, &entries, ADR_0005);
    Run { recorded, report }
}
