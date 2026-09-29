//! A finding is one rule violation with a stable code.

use std::fmt;

/// One violation reported by a gate. The code is stable and documented; the
/// detail is for people.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// The stable failure code, for example `forbidden-edge`.
    pub code: &'static str,
    /// What was found and where.
    pub detail: String,
}

impl Finding {
    /// Makes a finding.
    #[must_use]
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.detail)
    }
}

/// Prints the findings under the gate's word and returns the process exit
/// code: 0 when there are none, 1 otherwise.
#[must_use]
pub fn report(word: &str, findings: &[Finding]) -> i32 {
    if findings.is_empty() {
        println!("ok");
        return 0;
    }
    for finding in findings {
        println!("{word} {finding}");
    }
    1
}
