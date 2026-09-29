//! Layout probes. The renderer injects a script that measures the page after
//! layout; this module decides. The script reports facts, never verdicts, so
//! every rule here is tested without a browser.
//!
//! Rules, each a Check:
//!
//! - `horizontal-overflow`: the document is wider than the viewport.
//! - `clipped-text`: an element's own box hides part of its text.
//! - `overlapping-text`: the boxes of two unrelated text runs intersect.
//! - `low-contrast`: text contrast below 4.5:1, or 3:1 for large text (WCAG 2.2, 1.4.3).
//! - `small-target`: an interactive target smaller than 24 by 24 CSS pixels
//!   that is not inline in a sentence (WCAG 2.2, 2.5.8).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::check::Outcome;
use crate::viewport::Viewport;

/// A probe rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rule {
    /// The document is wider than the viewport.
    HorizontalOverflow,
    /// An element's box hides part of its text.
    ClippedText,
    /// Two unrelated text runs overlap.
    OverlappingText,
    /// Text contrast is below the WCAG minimum.
    LowContrast,
    /// An interactive target is too small.
    SmallTarget,
}

impl Rule {
    /// Every rule.
    pub const ALL: [Rule; 5] = [
        Rule::HorizontalOverflow,
        Rule::ClippedText,
        Rule::OverlappingText,
        Rule::LowContrast,
        Rule::SmallTarget,
    ];

    /// The rule's stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Rule::HorizontalOverflow => "horizontal-overflow",
            Rule::ClippedText => "clipped-text",
            Rule::OverlappingText => "overlapping-text",
            Rule::LowContrast => "low-contrast",
            Rule::SmallTarget => "small-target",
        }
    }

    /// The rule with this stable name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// Text whose contrast the page measured.
#[derive(Clone, Debug, PartialEq)]
pub struct ContrastSample {
    /// A short description of the element.
    pub element: String,
    /// Contrast ratio against the nearest opaque background, 1 to 21.
    pub ratio: f64,
    /// Whether the text is large: at least 24px, or at least 18.66px and bold.
    pub large: bool,
}

/// An interactive target the page measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    /// A short description of the element.
    pub element: String,
    /// Width in CSS pixels.
    pub width: f64,
    /// Height in CSS pixels.
    pub height: f64,
    /// Whether the target sits inline in a sentence, which WCAG exempts.
    pub inline: bool,
}

/// What the page measured of itself at one viewport.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Report {
    /// The viewport the page was rendered at.
    pub viewport: Option<Viewport>,
    /// `document.documentElement.scrollWidth`.
    pub document_width: u32,
    /// Elements whose own box hides part of their text.
    pub clipped: Vec<String>,
    /// Pairs of unrelated text elements whose boxes intersect.
    pub overlaps: Vec<(String, String)>,
    /// Contrast of every visible text element.
    pub contrast: Vec<ContrastSample>,
    /// Every interactive target.
    pub targets: Vec<Target>,
}

/// One rule violation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// The rule.
    pub rule: Rule,
    /// What and where.
    pub detail: String,
}

/// The WCAG 2.2 minimum ratio for text of this size.
#[must_use]
pub const fn minimum_ratio(large: bool) -> f64 {
    if large { 3.0 } else { 4.5 }
}

/// The smallest target side WCAG 2.2 2.5.8 accepts, in CSS pixels.
pub const MINIMUM_TARGET: f64 = 24.0;

/// Evaluates every rule over one report. The findings are sorted.
#[must_use]
pub fn evaluate(report: &Report) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(viewport) = report.viewport.filter(|v| report.document_width > v.width) {
        findings.push(Finding {
            rule: Rule::HorizontalOverflow,
            detail: format!("document is {}px wide at {viewport}", report.document_width),
        });
    }
    for element in &report.clipped {
        findings.push(Finding {
            rule: Rule::ClippedText,
            detail: format!("{element} hides part of its text"),
        });
    }
    for (a, b) in &report.overlaps {
        findings.push(Finding {
            rule: Rule::OverlappingText,
            detail: format!("{a} overlaps {b}"),
        });
    }
    for sample in &report.contrast {
        let minimum = minimum_ratio(sample.large);
        if sample.ratio < minimum {
            findings.push(Finding {
                rule: Rule::LowContrast,
                detail: format!(
                    "{} has contrast {:.2}:1, below {minimum}:1",
                    sample.element, sample.ratio
                ),
            });
        }
    }
    for target in &report.targets {
        if !target.inline && (target.width < MINIMUM_TARGET || target.height < MINIMUM_TARGET) {
            findings.push(Finding {
                rule: Rule::SmallTarget,
                detail: format!(
                    "{} is {:.0}x{:.0}px, below {MINIMUM_TARGET}x{MINIMUM_TARGET}",
                    target.element, target.width, target.height
                ),
            });
        }
    }
    findings.sort();
    findings
}

/// The Outcome of the probe Checks over a report: `Passed` with no findings,
/// `Failed` otherwise.
#[must_use]
pub fn outcome(findings: &[Finding]) -> Outcome {
    if findings.is_empty() {
        Outcome::Passed
    } else {
        Outcome::Failed
    }
}
