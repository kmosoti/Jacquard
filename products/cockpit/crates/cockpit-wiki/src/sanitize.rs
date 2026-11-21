//! Strict URL scheme allowlist, deterministic heading slugification, and HTML escaping.

use std::collections::HashMap;

use crate::markdown::RenderedMarkdown;

pub(crate) fn sanitize_link_url(raw: &str, out: &mut RenderedMarkdown) -> String {
    let trimmed = raw.trim();
    if !is_allowed_url(trimmed, true) {
        return "#blocked-unsafe-scheme".to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https:") || lower.starts_with("http:") {
        out.external_links.push(trimmed.to_string());
    } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !lower.starts_with("mailto:") {
        out.internal_links.push(trimmed.to_string());
    }
    trimmed.to_string()
}

pub(crate) fn sanitize_media_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if is_allowed_url(trimmed, false) {
        trimmed.to_string()
    } else {
        String::new()
    }
}

/// Strict URL allowlist: `https:`, `http:`, optional `mailto:`, `#` fragment,
/// `/` site-relative path (excluding `//`), or relative path without a scheme colon.
pub(crate) fn is_allowed_url(url: &str, allow_mailto: bool) -> bool {
    if url.is_empty() || url.chars().any(char::is_control) {
        return false;
    }
    if url.starts_with('#') {
        return true;
    }
    if url.starts_with('/') {
        return !url.starts_with("//");
    }
    let compact: String = url
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if compact.starts_with("https:") || compact.starts_with("http:") {
        return true;
    }
    if allow_mailto && compact.starts_with("mailto:") {
        return true;
    }
    let head = compact.split(['/', '?', '#']).next().unwrap_or("");
    !head.contains(':')
}

pub(crate) fn slugify(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "section".to_string()
    } else {
        out
    }
}

pub(crate) fn dedup_id(base: &str, used: &mut HashMap<String, usize>) -> String {
    let count = used.entry(base.to_string()).or_insert(0);
    let result = if *count == 0 {
        base.to_string()
    } else {
        format!("{base}-{}", *count)
    };
    *count += 1;
    result
}

pub(crate) fn escape_html_into(buf: &mut String, input: &str) {
    for ch in input.chars() {
        match ch {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            '"' => buf.push_str("&quot;"),
            '\'' => buf.push_str("&#39;"),
            other => buf.push(other),
        }
    }
}
