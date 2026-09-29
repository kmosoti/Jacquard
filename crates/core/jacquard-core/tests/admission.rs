//! Admission against ADR-0005. The truth table and the thresholds come from
//! the ADR, not from the code: the table is exhaustive over the nine verdict
//! pairs, and the threshold cases sit on each boundary.

use jacquard_core::admission::{
    ADR_0005, Class, Decision, Entry, Presentation, Shortfall, classify, decide,
};
use jacquard_core::judgment::Verdict::{self, First, Second, Tie};

#[test]
fn the_truth_table_of_adr_0005() {
    let table = [
        ((First, Second), Class::Correct),
        ((First, First), Class::Flipped),
        ((Second, Second), Class::Flipped),
        ((Second, First), Class::Wrong),
        ((First, Tie), Class::Wrong),
        ((Second, Tie), Class::Wrong),
        ((Tie, First), Class::Wrong),
        ((Tie, Second), Class::Wrong),
        ((Tie, Tie), Class::Wrong),
    ];
    for ((a, b), want) in table {
        assert_eq!(
            classify(a, b),
            want,
            "base first {a:?}, variant first {b:?}"
        );
    }
}

fn ids(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("pair-{i:02}")).collect()
}

/// A run in which the first `correct` pairs are correct, the next `flipped`
/// are flipped, and the rest are wrong.
fn run(ids: &[String], correct: usize, flipped: usize) -> Vec<Entry<'_>> {
    let mut out = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let (a, b): (Verdict, Verdict) = if i < correct {
            (First, Second)
        } else if i < correct + flipped {
            (First, First)
        } else {
            (Second, First)
        };
        out.push(Entry {
            pair: id,
            presentation: Presentation::BaseFirst,
            verdict: a,
            pinned_model: true,
        });
        out.push(Entry {
            pair: id,
            presentation: Presentation::VariantFirst,
            verdict: b,
            pinned_model: true,
        });
    }
    out
}

fn decision(n: usize, correct: usize, flipped: usize) -> Decision {
    let ids = ids(n);
    let set: Vec<&str> = ids.iter().map(String::as_str).collect();
    decide(&set, &run(&ids, correct, flipped), ADR_0005).decision
}

#[test]
fn accuracy_boundary_is_inclusive_at_ninety_percent() {
    assert_eq!(decision(20, 18, 0), Decision::Admitted);
    assert_eq!(
        decision(20, 17, 0),
        Decision::Rejected(vec![Shortfall::Accuracy])
    );
    assert_eq!(decision(30, 27, 0), Decision::Admitted);
    assert_eq!(
        decision(30, 26, 0),
        Decision::Rejected(vec![Shortfall::Accuracy])
    );
}

#[test]
fn flip_boundary_is_inclusive_at_ten_percent() {
    assert_eq!(decision(30, 27, 3), Decision::Admitted);
    assert_eq!(
        decision(30, 26, 4),
        Decision::Rejected(vec![Shortfall::Accuracy, Shortfall::FlipRate])
    );
    let ids = ids(40);
    let set: Vec<&str> = ids.iter().map(String::as_str).collect();
    let report = decide(&set, &run(&ids, 36, 4), ADR_0005);
    assert_eq!((report.correct, report.flipped, report.wrong), (36, 4, 0));
    assert_eq!(report.decision, Decision::Admitted);
    let report = decide(&set, &run(&ids, 35, 5), ADR_0005);
    assert_eq!(
        report.decision,
        Decision::Rejected(vec![Shortfall::Accuracy, Shortfall::FlipRate])
    );
}

#[test]
fn a_set_below_twenty_pairs_is_never_admitted() {
    assert_eq!(
        decision(19, 19, 0),
        Decision::Rejected(vec![Shortfall::TooFewPairs])
    );
    assert_eq!(decision(20, 20, 0), Decision::Admitted);
}

#[test]
fn an_answer_from_another_model_rejects() {
    let ids = ids(20);
    let set: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut entries = run(&ids, 20, 0);
    entries[7].pinned_model = false;
    assert_eq!(
        decide(&set, &entries, ADR_0005).decision,
        Decision::Rejected(vec![Shortfall::WrongModel])
    );
}

#[test]
fn an_incomplete_or_duplicated_run_is_inconclusive() {
    let ids = ids(20);
    let set: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut missing = run(&ids, 20, 0);
    missing.pop();
    assert_eq!(
        decide(&set, &missing, ADR_0005).decision,
        Decision::Inconclusive
    );
    let mut duplicated = run(&ids, 20, 0);
    duplicated.push(duplicated[0]);
    assert_eq!(
        decide(&set, &duplicated, ADR_0005).decision,
        Decision::Inconclusive
    );
    let mut stranger = run(&ids, 20, 0);
    stranger.push(Entry {
        pair: "not-in-set",
        presentation: Presentation::BaseFirst,
        verdict: First,
        pinned_model: true,
    });
    assert_eq!(
        decide(&set, &stranger, ADR_0005).decision,
        Decision::Inconclusive
    );
    assert_eq!(decide(&set, &[], ADR_0005).decision, Decision::Inconclusive);
}

/// Metamorphic: the order in which Judgments are recorded does not change
/// the report. Every rotation and the reversal are checked.
#[test]
fn recording_order_does_not_matter() {
    let ids = ids(24);
    let set: Vec<&str> = ids.iter().map(String::as_str).collect();
    let entries = run(&ids, 21, 2);
    let expected = decide(&set, &entries, ADR_0005);
    for shift in 0..entries.len() {
        let mut rotated = entries.clone();
        rotated.rotate_left(shift);
        assert_eq!(
            decide(&set, &rotated, ADR_0005),
            expected,
            "rotation {shift}"
        );
    }
    let mut reversed = entries.clone();
    reversed.reverse();
    assert_eq!(decide(&set, &reversed, ADR_0005), expected);
}
