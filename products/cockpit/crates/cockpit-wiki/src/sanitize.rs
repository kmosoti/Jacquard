//! Strict URL scheme allowlist, deterministic heading slugification, and HTML escaping.

use std::collections::HashMap;

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::markdown::RenderedMarkdown;

pub(crate) struct ImageCapture {
    pub(crate) src: String,
    pub(crate) title: String,
    pub(crate) alt: String,
    pub(crate) depth: usize,
}

impl ImageCapture {
    pub(crate) fn new(dest_url: &str, title: &str) -> Self {
        Self {
            src: sanitize_media_url(dest_url),
            title: title.to_string(),
            alt: String::new(),
            depth: 1,
        }
    }

    /// Consumes an event while inside an image description, returning `true` when
    /// the outer image tag closes and is ready to emit.
    pub(crate) fn push_event(&mut self, event: &Event<'_>, word_count: &mut usize) -> bool {
        match event {
            Event::Start(Tag::Image { .. }) => {
                self.depth += 1;
            }
            Event::End(TagEnd::Image) => {
                self.depth -= 1;
                if self.depth == 0 {
                    return true;
                }
            }
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::DisplayMath(text) => {
                *word_count += text.split_whitespace().count();
                self.alt.push_str(text);
            }
            Event::SoftBreak | Event::HardBreak => self.alt.push(' '),
            Event::FootnoteReference(name) => {
                self.alt.push('[');
                self.alt.push_str(name);
                self.alt.push(']');
            }
            _ => {}
        }
        false
    }

    pub(crate) fn emit_into(&self, target: &mut String) {
        target.push_str("<img src=\"");
        escape_html_into(target, &self.src);
        target.push_str("\" loading=\"lazy\"");
        if !self.title.is_empty() {
            target.push_str(" title=\"");
            escape_html_into(target, &self.title);
            target.push('"');
        }
        target.push_str(" alt=\"");
        escape_html_into(target, &self.alt);
        target.push_str("\" />");
    }
}

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
/// Rejects whitespace, control characters, quotes, angle brackets, and backslashes.
pub(crate) fn is_allowed_url(url: &str, allow_mailto: bool) -> bool {
    if url.is_empty()
        || url.chars().any(|c| {
            c.is_control() || c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\' | '`')
        })
    {
        return false;
    }
    if url.starts_with('#') {
        return true;
    }
    if url.starts_with('/') {
        return !url.starts_with("//");
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return true;
    }
    if allow_mailto && lower.starts_with("mailto:") {
        return url.len() > "mailto:".len();
    }
    let head = lower.split(['/', '?', '#']).next().unwrap_or("");
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
