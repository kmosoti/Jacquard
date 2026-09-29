//! The audit viewports: the sizes every Product is rendered and checked at.

use core::fmt;

/// A viewport in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Viewport {
    /// Width in CSS pixels.
    pub width: u32,
    /// Height in CSS pixels.
    pub height: u32,
}

impl Viewport {
    /// Parses `WIDTHxHEIGHT`, for example `390x844`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (w, h) = text.split_once('x')?;
        let width = w.parse().ok().filter(|v| *v > 0)?;
        let height = h.parse().ok().filter(|v| *v > 0)?;
        Some(Self { width, height })
    }
}

impl fmt::Display for Viewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

/// Large desktop, laptop, small laptop or tablet, phone.
pub const AUDIT: [Viewport; 4] = [
    Viewport {
        width: 2560,
        height: 1440,
    },
    Viewport {
        width: 1440,
        height: 900,
    },
    Viewport {
        width: 1024,
        height: 768,
    },
    Viewport {
        width: 390,
        height: 844,
    },
];
