//! The probe rules at their WCAG 2.2 boundaries: 4.5:1 and 3:1 contrast
//! (1.4.3), 24 by 24 CSS pixel targets with the inline exemption (2.5.8).

use jacquard_core::check::Outcome;
use jacquard_core::probe::{ContrastSample, Report, Rule, Target, evaluate, outcome};
use jacquard_core::viewport::{AUDIT, Viewport};

fn rules(report: &Report) -> Vec<Rule> {
    evaluate(report).into_iter().map(|f| f.rule).collect()
}

fn phone() -> Option<Viewport> {
    Viewport::parse("390x844")
}

#[test]
fn an_empty_page_passes() {
    let report = Report {
        viewport: phone(),
        document_width: 390,
        ..Report::default()
    };
    assert_eq!(outcome(&evaluate(&report)), Outcome::Passed);
}

#[test]
fn overflow_starts_one_pixel_past_the_viewport() {
    let at = Report {
        viewport: phone(),
        document_width: 390,
        ..Report::default()
    };
    let past = Report {
        viewport: phone(),
        document_width: 391,
        ..Report::default()
    };
    assert_eq!(rules(&at), []);
    assert_eq!(rules(&past), [Rule::HorizontalOverflow]);
    assert_eq!(outcome(&evaluate(&past)), Outcome::Failed);
}

fn contrast(ratio: f64, large: bool) -> Report {
    Report {
        viewport: phone(),
        document_width: 390,
        contrast: vec![ContrastSample {
            element: "p".into(),
            ratio,
            large,
        }],
        ..Report::default()
    }
}

#[test]
fn contrast_thresholds_are_wcag_1_4_3() {
    assert_eq!(rules(&contrast(4.5, false)), []);
    assert_eq!(rules(&contrast(4.49, false)), [Rule::LowContrast]);
    assert_eq!(rules(&contrast(3.0, true)), []);
    assert_eq!(rules(&contrast(2.99, true)), [Rule::LowContrast]);
    assert_eq!(rules(&contrast(21.0, false)), []);
}

fn target(width: f64, height: f64, inline: bool) -> Report {
    Report {
        viewport: phone(),
        document_width: 390,
        targets: vec![Target {
            element: "button".into(),
            width,
            height,
            inline,
        }],
        ..Report::default()
    }
}

#[test]
fn target_size_is_wcag_2_5_8() {
    assert_eq!(rules(&target(24.0, 24.0, false)), []);
    assert_eq!(rules(&target(23.9, 44.0, false)), [Rule::SmallTarget]);
    assert_eq!(rules(&target(44.0, 23.9, false)), [Rule::SmallTarget]);
    assert_eq!(
        rules(&target(12.0, 12.0, true)),
        [],
        "inline targets are exempt"
    );
}

#[test]
fn clipped_and_overlapping_text_each_fail() {
    let report = Report {
        viewport: phone(),
        document_width: 390,
        clipped: vec!["h1".into()],
        overlaps: vec![("h1".into(), "p".into())],
        ..Report::default()
    };
    assert_eq!(rules(&report), [Rule::ClippedText, Rule::OverlappingText]);
}

#[test]
fn rule_names_round_trip() {
    for rule in Rule::ALL {
        assert_eq!(Rule::from_name(rule.name()), Some(rule));
    }
    assert_eq!(Rule::from_name("looks-fine"), None);
}

#[test]
fn viewports_parse_and_print() {
    for viewport in AUDIT {
        assert_eq!(Viewport::parse(&viewport.to_string()), Some(viewport));
    }
    for bad in ["390", "x844", "0x844", "390x0", "390x-1", "wide"] {
        assert_eq!(Viewport::parse(bad), None, "{bad}");
    }
}
