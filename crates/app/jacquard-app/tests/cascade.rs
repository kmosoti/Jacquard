//! The use cases against fake adapters. The fakes are the oracle's shape,
//! not its content: a Judge that reads the probe (a perfect judge), one that
//! always picks the first slot, and one answered by another model.

use std::collections::BTreeMap;

use jacquard_app::admission::{self, Pair};
use jacquard_app::{compare, probe_gate, swatch_id};
use jacquard_core::admission::{Decision, Shortfall};
use jacquard_core::check::Outcome;
use jacquard_core::judgment::Verdict;
use jacquard_core::probe::{Report, Rule};
use jacquard_core::viewport::Viewport;
use jacquard_ports::{Answer, Failure, Judge, Render, SwatchSource};

const PINNED: &str = "claude-sonnet-5-5";

fn render(clipped: bool) -> Render {
    let viewport = Viewport {
        width: 390,
        height: 844,
    };
    Render {
        viewport,
        png: Vec::new(),
        probe: Report {
            viewport: Some(viewport),
            document_width: 390,
            clipped: if clipped {
                vec!["h1".into()]
            } else {
                Vec::new()
            },
            ..Report::default()
        },
    }
}

fn answer(verdict: Verdict, model: &str) -> Answer {
    Answer {
        verdict,
        dimensions: BTreeMap::new(),
        reason: String::new(),
        model: model.into(),
    }
}

struct Perfect;
impl Judge for Perfect {
    fn pinned_model(&self) -> &str {
        PINNED
    }
    fn rubric_hash(&self) -> String {
        "perfect".into()
    }
    fn judge(&self, first: &[Render], second: &[Render]) -> Result<Answer, Failure> {
        let bad = |r: &[Render]| r.iter().any(|x| !x.probe.clipped.is_empty());
        let verdict = match (bad(first), bad(second)) {
            (false, true) => Verdict::First,
            (true, false) => Verdict::Second,
            _ => Verdict::Tie,
        };
        Ok(answer(verdict, PINNED))
    }
}

struct AlwaysFirst(&'static str);
impl Judge for AlwaysFirst {
    fn pinned_model(&self) -> &str {
        PINNED
    }
    fn rubric_hash(&self) -> String {
        "first".into()
    }
    fn judge(&self, _: &[Render], _: &[Render]) -> Result<Answer, Failure> {
        Ok(answer(Verdict::First, self.0))
    }
}

struct Broken;
impl Judge for Broken {
    fn pinned_model(&self) -> &str {
        PINNED
    }
    fn rubric_hash(&self) -> String {
        "broken".into()
    }
    fn judge(&self, _: &[Render], _: &[Render]) -> Result<Answer, Failure> {
        Err(Failure::Failed("no verdict".into()))
    }
}

fn pairs(n: usize) -> Vec<Pair> {
    (0..n)
        .map(|i| Pair {
            id: format!("pair-{i:02}"),
            rule: Rule::ClippedText,
            base: vec![render(false)],
            variant: vec![render(true)],
        })
        .collect()
}

#[test]
fn a_perfect_judge_is_admitted() {
    let run = admission::run(&Perfect, &pairs(20), |_| {});
    assert_eq!(run.recorded.len(), 40);
    assert_eq!((run.report.correct, run.report.flipped), (20, 0));
    assert_eq!(run.report.decision, Decision::Admitted);
}

#[test]
fn a_judge_that_follows_the_order_is_rejected() {
    let run = admission::run(&AlwaysFirst(PINNED), &pairs(20), |_| {});
    assert_eq!(run.report.flipped, 20);
    assert_eq!(
        run.report.decision,
        Decision::Rejected(vec![Shortfall::Accuracy, Shortfall::FlipRate])
    );
}

#[test]
fn an_answer_from_another_model_is_rejected() {
    let run = admission::run(&AlwaysFirst("claude-sonnet-5"), &pairs(20), |_| {});
    let Decision::Rejected(short) = run.report.decision else {
        panic!("not rejected")
    };
    assert!(short.contains(&Shortfall::WrongModel), "{short:?}");
}

#[test]
fn failed_judgments_make_the_run_inconclusive() {
    let run = admission::run(&Broken, &pairs(20), |_| {});
    assert_eq!(run.report.decision, Decision::Inconclusive);
}

#[test]
fn a_pair_is_known_only_when_the_rule_separates_its_sides() {
    let good = &pairs(1)[0];
    assert_eq!(admission::verify(good), Ok(()));
    let both_bad = Pair {
        base: vec![render(true)],
        ..good.clone()
    };
    assert!(
        admission::verify(&both_bad)
            .unwrap_err()
            .contains("base already fails")
    );
    let neither = Pair {
        variant: vec![render(false)],
        ..good.clone()
    };
    assert!(
        admission::verify(&neither)
            .unwrap_err()
            .contains("variant does not fail")
    );
    let other_rule = Pair {
        rule: Rule::LowContrast,
        ..good.clone()
    };
    assert!(
        admission::verify(&other_rule).is_err(),
        "the defect must trip its own rule"
    );
}

#[test]
fn comparisons_ask_in_both_orders() {
    let c = compare(&Perfect, &[render(false)], &[render(true)]).expect("compared");
    assert_eq!(
        (c.a_first.verdict, c.b_first.verdict),
        (Verdict::First, Verdict::Second)
    );
}

#[test]
fn the_probe_gate_fails_if_any_viewport_fails() {
    assert_eq!(
        probe_gate(&[render(false), render(false)]).0,
        Outcome::Passed
    );
    assert_eq!(
        probe_gate(&[render(false), render(true)]).0,
        Outcome::Failed
    );
}

#[test]
fn swatch_identity_is_content() {
    let source = |entry: &str, files: &[(&str, &[u8])]| SwatchSource {
        entry: entry.into(),
        files: files
            .iter()
            .map(|(p, b)| ((*p).to_string(), b.to_vec()))
            .collect(),
    };
    let a = source(
        "index.html",
        &[("index.html", b"<p>hi</p>"), ("a.css", b"p{}")],
    );
    let same = source(
        "index.html",
        &[("a.css", b"p{}"), ("index.html", b"<p>hi</p>")],
    );
    assert_eq!(
        swatch_id(&a),
        swatch_id(&same),
        "file order does not matter"
    );
    let edited = source(
        "index.html",
        &[("index.html", b"<p>hi!</p>"), ("a.css", b"p{}")],
    );
    assert_ne!(swatch_id(&a), swatch_id(&edited));
    let shifted = source(
        "index.html",
        &[("index.html", b"<p>hi</p>a.css"), ("", b"p{}")],
    );
    assert_ne!(
        swatch_id(&a),
        swatch_id(&shifted),
        "length prefixes keep boundaries"
    );
}
