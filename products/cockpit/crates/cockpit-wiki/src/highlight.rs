//! Zero-JS server-side code syntax tokenizer (Rust, Nix, TOML, Shell) and
//! native `iso-diagram` Mermaid flowchart-to-SVG renderer.

use std::collections::HashMap;

use iso_diagram::model::{Asset, Diagram, EdgeStyle, NodeId, ThemePreset, Tint, ZoneStyle};
use iso_diagram::svg::render_svg;

use crate::sanitize::escape_html_into;

const RUST_KW: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];
const NIX_KW: &[&str] = &[
    "assert", "builtins", "else", "false", "if", "import", "in", "inherit", "let", "null", "or",
    "rec", "then", "true", "with",
];
const SHELL_KW: &[&str] = &[
    "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in",
    "local", "return", "set", "then", "until", "while",
];

/// Highlights a fenced code block into HTML-escaped `<span class="tok-*">` markup.
#[must_use]
pub fn highlight_code(lang: &str, code: &str) -> String {
    let lower = lang.trim().to_ascii_lowercase();
    let mut out = String::with_capacity(code.len() + code.len() / 2);
    match lower.as_str() {
        "rust" | "rs" => tokenize_code(code, RUST_KW, "//", false, &mut out),
        "nix" => tokenize_code(code, NIX_KW, "#", false, &mut out),
        "sh" | "bash" | "shell" | "zsh" => tokenize_code(code, SHELL_KW, "#", false, &mut out),
        "toml" => tokenize_toml(code, &mut out),
        _ => escape_html_into(&mut out, code),
    }
    out
}

fn push_span(out: &mut String, cls: &str, text: &str) {
    out.push_str("<span class=\"tok-");
    out.push_str(cls);
    out.push_str("\">");
    escape_html_into(out, text);
    out.push_str("</span>");
}

fn tokenize_code(code: &str, kws: &[&str], comment_prefix: &str, is_toml: bool, out: &mut String) {
    let bytes = code.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if code[i..].starts_with(comment_prefix) {
            let end = code[i..].find('\n').map_or(bytes.len(), |n| i + n);
            push_span(out, "com", &code[i..end]);
            i = end;
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            let q = bytes[i];
            let mut j = i + 1;
            while j < bytes.len() {
                if bytes[j] == b'\\' && j + 1 < bytes.len() {
                    j += 2;
                    continue;
                }
                if bytes[j] == q {
                    j += 1;
                    break;
                }
                j += 1;
            }
            push_span(out, "str", &code[i..j]);
            i = j;
        } else if bytes[i].is_ascii_digit() {
            let mut j = i + 1;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'.')
            {
                j += 1;
            }
            push_span(out, "num", &code[i..j]);
            i = j;
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let mut j = i + 1;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric()
                    || bytes[j] == b'_'
                    || (is_toml && bytes[j] == b'-'))
            {
                j += 1;
            }
            let word = &code[i..j];
            if kws.contains(&word) {
                push_span(out, "kw", word);
            } else if word.chars().next().is_some_and(char::is_uppercase) {
                push_span(out, "type", word);
            } else {
                escape_html_into(out, word);
            }
            i = j;
        } else {
            let ch = code[i..].chars().next().expect("valid utf8");
            let end = i + ch.len_utf8();
            escape_html_into(out, &code[i..end]);
            i = end;
        }
    }
}

fn tokenize_toml(code: &str, out: &mut String) {
    for (idx, line) in code.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            push_span(out, "com", line);
        } else if trimmed.starts_with('[') && trimmed.trim_end().ends_with(']') {
            push_span(out, "type", line);
        } else if let Some(eq) = line.find('=') {
            push_span(out, "key", &line[..eq]);
            out.push('=');
            tokenize_code(&line[eq + 1..], &["true", "false"], "#", true, out);
        } else {
            tokenize_code(line, &["true", "false"], "#", true, out);
        }
    }
}

/// Renders a fenced ```` ```mermaid ```` block into a semantic `<figure>` containing
/// a native `iso-diagram` SVG when it is a flowchart/graph, or a semantic zero-JS
/// `<figure><pre class="mermaid">` fallback otherwise.
#[must_use]
pub fn render_mermaid_block(src: &str) -> String {
    if let Some(diag) = parse_mermaid_flowchart(src) {
        let svg = render_svg(&diag, 760.0, 360.0);
        let mut out = String::with_capacity(svg.len() + 192);
        out.push_str(
            "<figure class=\"wiki-diagram\" data-lang=\"mermaid\" data-renderer=\"iso-diagram\">\n",
        );
        out.push_str(&svg);
        out.push_str(
            "<figcaption class=\"sr-only\">Mermaid flowchart diagram</figcaption>\n</figure>\n",
        );
        return out;
    }
    let mut out = String::with_capacity(src.len() + 128);
    out.push_str(
        "<figure class=\"wiki-diagram\" data-lang=\"mermaid\" data-renderer=\"fallback\"><pre class=\"mermaid\" data-lang=\"mermaid\">",
    );
    escape_html_into(&mut out, src);
    out.push_str("</pre></figure>\n");
    out
}

fn parse_mermaid_flowchart(src: &str) -> Option<Diagram> {
    let mut lines = src
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("%%"));
    let header = lines.next()?;
    let lower = header.to_ascii_lowercase();
    if !(lower.starts_with("flowchart") || lower.starts_with("graph")) {
        return None;
    }

    let mut builder = Diagram::builder("Mermaid Flowchart");
    builder.theme(ThemePreset::WarmPaper);
    let mut nodes: HashMap<String, NodeId> = HashMap::new();
    let mut active_zone = None;
    let mut edge_count = 0usize;

    for line in lines {
        let low = line.to_ascii_lowercase();
        if let Some(rest) = low.strip_prefix("subgraph ") {
            let title = line[9..].trim().trim_matches('"');
            let slug = iso_diagram::model::sanitize_slug(rest);
            active_zone = Some(builder.add_zone(slug, title, ZoneStyle::Dashed, Tint::Slate, None));
            continue;
        }
        if low == "end" {
            active_zone = None;
            continue;
        }
        if let Some((lhs, style, label, rhs)) = split_mermaid_edge(line) {
            let from = intern_node(&mut builder, &mut nodes, lhs, active_zone);
            let to = intern_node(&mut builder, &mut nodes, rhs, active_zone);
            let mut er = builder.edge(from, to).style(style).tint(Tint::Cobalt);
            if let Some(lbl) = label {
                er = er.label(lbl);
            }
            let _ = er.id();
            edge_count += 1;
        }
    }
    if edge_count == 0 {
        return None;
    }
    Some(builder.build())
}

fn split_mermaid_edge(line: &str) -> Option<(&str, EdgeStyle, Option<String>, &str)> {
    for (op, style) in [
        ("-.->", EdgeStyle::Dashed),
        ("==>", EdgeStyle::Solid),
        ("-->", EdgeStyle::Solid),
        ("---", EdgeStyle::Solid),
    ] {
        if let Some((lhs, rhs_raw)) = line.split_once(op) {
            let rhs_trim = rhs_raw.trim();
            let (label, rhs) = if let Some(after) = rhs_trim.strip_prefix('|') {
                if let Some((lbl, rest)) = after.split_once('|') {
                    (Some(lbl.trim().to_owned()), rest.trim())
                } else {
                    (None, rhs_trim)
                }
            } else {
                (None, rhs_trim)
            };
            if !lhs.trim().is_empty() && !rhs.is_empty() {
                return Some((lhs.trim(), style, label, rhs));
            }
        }
    }
    None
}

fn intern_node(
    builder: &mut iso_diagram::builder::DiagramBuilder,
    nodes: &mut HashMap<String, NodeId>,
    raw: &str,
    zone: Option<iso_diagram::model::ZoneId>,
) -> NodeId {
    let (id_part, label, asset) = parse_node_token(raw);
    if let Some(&existing) = nodes.get(id_part) {
        return existing;
    }
    let slug = iso_diagram::model::sanitize_slug(id_part);
    let nid = builder.insert_node(slug, asset, label, zone).id();
    nodes.insert(id_part.to_owned(), nid);
    nid
}

fn parse_node_token(raw: &str) -> (&str, String, Asset) {
    let trimmed = raw.trim().trim_end_matches(';');
    for (open, close, asset) in [
        (
            "[",
            "]",
            Asset::Rack {
                blades: 2,
                pedestal: false,
            },
        ),
        ("(", ")", Asset::Pod),
        ("{", "}", Asset::Balancer),
    ] {
        if let Some((id, rest)) = trimmed.split_once(open) {
            let lbl = rest
                .trim_end_matches(close)
                .trim()
                .trim_matches('"')
                .to_owned();
            let id_clean = id.trim();
            return (
                if id_clean.is_empty() {
                    trimmed
                } else {
                    id_clean
                },
                if lbl.is_empty() {
                    trimmed.to_owned()
                } else {
                    lbl
                },
                asset,
            );
        }
    }
    (trimmed, trimmed.to_owned(), Asset::Pod)
}
