//! Pure-Rust OKLCH-to-WCAG-2.2 relative luminance and contrast verifier.
//!
//! Parses CSS custom property declarations from `tokens.css` and computes
//! exact WCAG 2.2 contrast ratios `(L1 + 0.05) / (L2 + 0.05)` for foreground
//! and background token pairs in both light (`Atelier-Draft`) and dark
//! (`Oxide-Monolith`) palettes.

use std::collections::BTreeMap;

/// Linear OKLCH color triple plus alpha channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OklchColor {
    /// Perceptual lightness (`0.0..=1.0`).
    pub l: f64,
    /// Chroma magnitude (`>= 0.0`).
    pub c: f64,
    /// Hue angle in degrees (`0.0..360.0`).
    pub h_deg: f64,
    /// Alpha opacity (`0.0..=1.0`).
    pub alpha: f64,
}

/// One measured contrast pair audit result.
#[derive(Debug, Clone, PartialEq)]
pub struct ContrastCheckResult {
    /// Semantic label describing the foreground/background pair.
    pub label: &'static str,
    /// Minimum required WCAG 2.2 AA ratio (`4.5` normal text, `3.0` UI border/focus).
    pub required_ratio: f64,
    /// Measured contrast ratio (`1.0..=21.0`).
    pub actual_ratio: f64,
    /// True if `actual_ratio >= required_ratio`.
    pub passed: bool,
}

/// Parses `--token: oklch(...);` definitions from CSS source text.
pub fn extract_oklch_tokens(css: &str) -> BTreeMap<String, OklchColor> {
    let mut map = BTreeMap::new();
    for line in css.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("--") else {
            continue;
        };
        let Some((name, val)) = rest.split_once(':') else {
            continue;
        };
        let key = format!("--{}", name.trim());
        if let Some(color) = parse_oklch_expr(val.trim().trim_end_matches(';')) {
            map.insert(key, color);
        }
    }
    map
}

/// Parses a single `oklch(L C H [/ alpha])` CSS expression.
pub fn parse_oklch_expr(expr: &str) -> Option<OklchColor> {
    let inner = expr.strip_prefix("oklch(")?.strip_suffix(')')?.trim();
    let (main_part, alpha) = if let Some((left, right)) = inner.split_once('/') {
        let a = parse_percent_or_unit(right.trim())?;
        (left.trim(), a)
    } else {
        (inner, 1.0)
    };
    let mut nums = main_part.split_whitespace();
    let l = parse_percent_or_unit(nums.next()?)?;
    let c = nums.next()?.parse::<f64>().ok()?;
    let h_raw = nums.next()?;
    let h_deg = h_raw
        .strip_suffix("deg")
        .unwrap_or(h_raw)
        .parse::<f64>()
        .ok()?;
    Some(OklchColor { l, c, h_deg, alpha })
}

fn parse_percent_or_unit(token: &str) -> Option<f64> {
    if let Some(pct) = token.strip_suffix('%') {
        pct.trim().parse::<f64>().ok().map(|v| v / 100.0)
    } else {
        token.parse::<f64>().ok()
    }
}

/// Converts [`OklchColor`] to clamped linear sRGB `(r, g, b)` using the Oklab standard matrix.
pub fn oklch_to_linear_srgb(color: OklchColor) -> (f64, f64, f64) {
    let h_rad = color.h_deg.to_radians();
    let a = color.c * h_rad.cos();
    let b = color.c * h_rad.sin();

    let l_ = color.l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = color.l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = color.l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s;
    let g = -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s;
    let bl = -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s;

    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), bl.clamp(0.0, 1.0))
}

/// Computes WCAG 2.2 relative luminance `Y` from an [`OklchColor`].
pub fn relative_luminance(color: OklchColor) -> f64 {
    let (r, g, b) = oklch_to_linear_srgb(color);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Computes WCAG 2.2 contrast ratio `(L_lighter + 0.05) / (L_darker + 0.05)` between two OKLCH colors.
pub fn contrast_ratio(fg: OklchColor, bg: OklchColor) -> f64 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

/// Evaluates all critical Hallmark Light + Dark contrast pairs in `tokens_css` against WCAG 2.2 AA.
pub fn verify_tokens_wcag_aa(tokens_css: &str) -> Result<Vec<ContrastCheckResult>, String> {
    let tokens = extract_oklch_tokens(tokens_css);
    let pairs: [(&'static str, &str, &str, f64); 18] = [
        ("light ink on canvas", "--l-ink", "--l-canvas", 4.5),
        ("light ink on surface", "--l-ink", "--l-surface", 4.5),
        ("light ink on rail", "--l-ink", "--l-rail", 4.5),
        (
            "light muted on surface",
            "--l-ink-muted",
            "--l-surface",
            4.5,
        ),
        ("light muted on rail", "--l-ink-muted", "--l-rail", 4.5),
        (
            "light subtle on surface",
            "--l-ink-subtle",
            "--l-surface",
            4.5,
        ),
        (
            "light accent-ink on accent",
            "--l-accent-ink",
            "--l-accent",
            4.5,
        ),
        ("light ok pill", "--l-status-ok", "--l-status-ok-bg", 4.5),
        (
            "light warn pill",
            "--l-status-warn",
            "--l-status-warn-bg",
            4.5,
        ),
        ("light err pill", "--l-status-err", "--l-status-err-bg", 4.5),
        (
            "light info pill",
            "--l-status-info",
            "--l-status-info-bg",
            4.5,
        ),
        (
            "light focus ring on surface",
            "--l-focus-ring",
            "--l-surface",
            3.0,
        ),
        ("dark ink on canvas", "--d-ink", "--d-canvas", 4.5),
        ("dark ink on surface", "--d-ink", "--d-surface", 4.5),
        ("dark muted on rail", "--d-ink-muted", "--d-rail", 4.5),
        (
            "dark subtle on surface",
            "--d-ink-subtle",
            "--d-surface",
            4.5,
        ),
        (
            "dark accent-ink on accent",
            "--d-accent-ink",
            "--d-accent",
            4.5,
        ),
        ("dark ok pill", "--d-status-ok", "--d-status-ok-bg", 4.5),
    ];

    let mut out = Vec::with_capacity(pairs.len());
    for (label, fg_key, bg_key, required_ratio) in pairs {
        let fg = tokens
            .get(fg_key)
            .copied()
            .ok_or_else(|| format!("missing OKLCH token {fg_key}"))?;
        let bg = tokens
            .get(bg_key)
            .copied()
            .ok_or_else(|| format!("missing OKLCH token {bg_key}"))?;
        let actual_ratio = contrast_ratio(fg, bg);
        out.push(ContrastCheckResult {
            label,
            required_ratio,
            actual_ratio,
            passed: actual_ratio >= required_ratio,
        });
    }
    Ok(out)
}
