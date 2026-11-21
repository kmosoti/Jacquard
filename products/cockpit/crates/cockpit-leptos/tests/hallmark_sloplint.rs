//! Pure-Rust Hallmark `Floor` + `Reflex` + `Ledger` + OKLCH WCAG 2.2 AA automated test suite.

use std::path::PathBuf;

use cockpit_leptos::wcag::verify_tokens_wcag_aa;
use cockpit_leptos::{COMPONENTS_CSS, LAYOUT_CSS, TOKENS_CSS, render_ssr_shell_html};

fn style_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("style")
        .join(name)
}

fn read_style(name: &str) -> String {
    let path = style_path(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn contains_raw_color_literal(css: &str) -> Option<String> {
    let clean = strip_css_comments(css);
    for (idx, line) in clean.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        for forbidden in ["oklch(", "oklab(", "rgb(", "rgba(", "hsl(", "hsla("] {
            if lower.contains(forbidden) {
                return Some(format!("line {}: found `{forbidden}` in `{line}`", idx + 1));
            }
        }
        let bytes = line.as_bytes();
        for i in 0..bytes.len() {
            if bytes[i] == b'#' {
                let hex_len = bytes[i + 1..]
                    .iter()
                    .take_while(|b| b.is_ascii_hexdigit())
                    .count();
                if matches!(hex_len, 3 | 4 | 6 | 8) {
                    return Some(format!("line {}: found hex literal in `{line}`", idx + 1));
                }
            }
        }
    }
    None
}

#[test]
fn hallmark_floor_token_purity_and_cascade_layers() {
    let tokens = read_style("tokens.css");
    let layout = read_style("layout.css");
    let components = read_style("components.css");

    assert_eq!(tokens, TOKENS_CSS);
    assert_eq!(layout, LAYOUT_CSS);
    assert_eq!(components, COMPONENTS_CSS);

    assert!(
        tokens.contains("@layer reset, tokens, base, layout, components, utilities;"),
        "tokens.css must declare cascade layer order upfront"
    );
    assert!(
        tokens.contains("color-scheme: light dark;"),
        "tokens.css must declare root color-scheme"
    );
    assert!(
        tokens.contains("@media (prefers-color-scheme: dark)"),
        "tokens.css must provide prefers-color-scheme dark fallback"
    );
    assert!(
        tokens.contains("@supports (color: light-dark(white, black))"),
        "tokens.css must enhance with light-dark()"
    );
    assert!(
        tokens.contains("@media (forced-colors: active)"),
        "tokens.css must support Forced Colors Mode"
    );
    for sys_kw in ["Canvas", "CanvasText", "Highlight", "ButtonBorder"] {
        assert!(
            tokens.contains(sys_kw),
            "tokens.css forced-colors block must reference {sys_kw}"
        );
    }
    for semantic in [
        "--status-ok",
        "--status-warn",
        "--status-err",
        "--status-info",
    ] {
        assert!(
            tokens.contains(semantic),
            "missing semantic token {semantic}"
        );
    }

    for (name, css) in [
        ("tokens.css", &tokens),
        ("layout.css", &layout),
        ("components.css", &components),
    ] {
        let clean = strip_css_comments(css);
        assert!(
            !clean.contains("!important"),
            "{name} must not use !important"
        );
        assert!(
            !clean.contains("color-mix(in oklch"),
            "{name} violates Chromium #565837690 guard (color-mix in oklch)"
        );
        assert!(
            !clean.contains("transition: all"),
            "{name} must not use transition: all"
        );
        assert!(
            !clean.contains("linear-gradient(") && !clean.contains("radial-gradient("),
            "{name} must not introduce decorative gradients"
        );
        for (idx, line) in clean.lines().enumerate() {
            let trimmed = line.trim();
            assert!(
                !trimmed.starts_with("* {") && trimmed != "*,",
                "{name}:{} must not use global `*` reset",
                idx + 1
            );
            if trimmed.starts_with("font-size:") {
                assert!(
                    !trimmed.contains("px"),
                    "{name}:{} must not use px font-size: `{trimmed}`",
                    idx + 1
                );
            }
        }
    }

    if let Some(hit) = contains_raw_color_literal(&layout) {
        panic!("layout.css contains raw color literal outside tokens.css: {hit}");
    }
    if let Some(hit) = contains_raw_color_literal(&components) {
        panic!("components.css contains raw color literal outside tokens.css: {hit}");
    }
}

#[test]
fn hallmark_wcag_aa_oklch_contrast_floor_passes_and_catches_negative_control() {
    let results = verify_tokens_wcag_aa(TOKENS_CSS).expect("parse OKLCH tokens");
    assert_eq!(results.len(), 18);
    for r in &results {
        assert!(
            r.passed,
            "WCAG 2.2 AA contrast failure for `{}`: {:.2}:1 < {:.1}:1",
            r.label, r.actual_ratio, r.required_ratio
        );
    }

    // Negative control: intentionally illegible light-grey on light-paper surface must fail!
    let broken_css = TOKENS_CSS.replace(
        "--l-ink-muted: oklch(0.415 0.014 252);",
        "--l-ink-muted: oklch(0.85 0.01 252);",
    );
    let neg_results = verify_tokens_wcag_aa(&broken_css).expect("parse broken tokens");
    assert!(
        neg_results
            .iter()
            .any(|r| r.label == "light muted on surface" && !r.passed),
        "WCAG verifier negative control failed to catch low-contrast pair"
    );
}

#[test]
fn hallmark_reflex_modern_layout_overlays_and_accessibility_css() {
    let tokens = read_style("tokens.css");
    let layout = read_style("layout.css");
    let components = read_style("components.css");

    assert!(
        tokens.contains(":focus-visible"),
        "missing :focus-visible outline"
    );
    assert!(
        layout.contains("grid-template-areas:"),
        "missing grid-template-areas cockpit"
    );
    assert!(layout.contains("100dvb"), "missing 100dvb viewport sizing");
    assert!(
        layout.contains("scrollbar-gutter: stable"),
        "missing scrollbar-gutter: stable"
    );
    assert!(
        layout.contains("overscroll-behavior: contain"),
        "missing overscroll-behavior: contain"
    );
    assert!(
        layout.contains("grid-template-rows: subgrid"),
        "missing subgrid alignment"
    );
    assert!(
        layout.contains("container: stage / inline-size"),
        "missing stage container query"
    );
    assert!(
        layout.contains("container: inspector / inline-size"),
        "missing inspector container query"
    );

    assert!(
        components.contains("font-variant-numeric: tabular-nums"),
        "missing tabular-nums"
    );
    assert!(
        components.contains("min-block-size: max(24px"),
        "missing 24px min-block-size target"
    );
    assert!(
        components.contains("contain: layout style paint"),
        "missing SVG viewport containment"
    );
    assert!(
        components.contains("content-visibility: auto"),
        "missing content-visibility: auto"
    );
    assert!(
        components.contains("contain-intrinsic-block-size: auto 2.2lh"),
        "missing contain-intrinsic-block-size on rows"
    );
    assert!(
        components.contains("@scope (.app-surface)"),
        "missing @scope (.app-surface)"
    );
    assert!(
        components.contains("@scope (.wiki-prose)"),
        "missing @scope (.wiki-prose)"
    );
    assert!(
        components.contains(":is(:popover-open, .\\:popover-open)"),
        "missing forgiving popover-open selector"
    );
    assert!(
        components.contains("@supports (position-area: block-end)"),
        "missing @supports guard around position-area"
    );
    assert!(
        components
            .contains("position-try-fallbacks: flip-block, flip-inline, flip-block flip-inline"),
        "missing viewport-aware position-try-fallbacks"
    );
    assert!(
        components.contains("anchor-name: --active-rail-item"),
        "missing anchor-name active rail indicator"
    );
    assert!(
        components.contains("@media (prefers-reduced-motion: no-preference)"),
        "missing prefers-reduced-motion: no-preference guard"
    );
    assert!(
        components.contains("@media (prefers-reduced-motion: reduce)"),
        "missing prefers-reduced-motion: reduce guard"
    );
    assert!(
        components.contains("@media (forced-colors: active)")
            && components.contains("color: HighlightText"),
        "components.css must pair HighlightText on active Forced-Colors items"
    );

    let clean = strip_css_comments(&components);
    let mut supports_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut supports_start_depth: Vec<usize> = Vec::new();
    for (idx, line) in clean.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("@supports") {
            supports_depth += 1;
            supports_start_depth.push(brace_depth);
        }
        if trimmed == "inset: auto;" {
            assert!(
                supports_depth > 0,
                "components.css:{} bare `inset: auto;` outside @supports breaks fallback popovers",
                idx + 1
            );
        }
        for ch in trimmed.chars() {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
                if supports_start_depth.last() == Some(&brace_depth) {
                    supports_start_depth.pop();
                    supports_depth = supports_depth.saturating_sub(1);
                }
            }
        }
    }
}

#[test]
fn hallmark_ledger_ssr_shell_semantics_and_zero_slop() {
    let html = render_ssr_shell_html();
    assert!(!html.is_empty(), "SSR shell HTML must be non-empty");

    for required in [
        "leptos-island",
        "<header",
        "<nav",
        "<main",
        "<aside",
        "<h1",
        "<dialog",
        "closedby=\"any\"",
        "popover=\"auto\"",
        "<details",
        "assets-drawer",
        "<svg",
        "role=\"img\"",
        "role=\"group\"",
        "aria-roledescription=",
        "role=\"toolbar\"",
        "role=\"listbox\"",
        "iso-callout-sub",
        "<title>",
        "data-state=",
        "skip-link",
        "Jacquard Cockpit",
        "StageOrbitIsland",
        "InspectorStreamIsland",
        "CommandPaletteIsland",
        "__hydrate",
    ] {
        assert!(
            html.contains(required),
            "SSR HTML missing required landmark/primitive `{required}`"
        );
    }

    assert_eq!(
        html.matches("<main").count(),
        1,
        "SSR shell must contain exactly one <main> landmark"
    );

    let forbidden_host_leaks = ["/home/", "/Users/", "bwrap://", "localhost:", "127.0.0.1"];
    for forbidden in forbidden_host_leaks {
        assert!(
            !html.contains(forbidden),
            "SSR HTML must not embed local host path or non-generic fixture `{forbidden}`"
        );
    }

    for emoji in ["🟢", "🔴", "🟡", "⚠", "✅", "❌", "🚀", "🔥", "✨", "💎"] {
        assert!(
            !html.contains(emoji),
            "SSR HTML must not use emoji status icon `{emoji}`; use paired geometric SVG + text"
        );
    }

    assert!(
        !html.contains("style=\"color:") && !html.contains("style=\"background:"),
        "SSR HTML must not embed static inline color/background styles"
    );
}
