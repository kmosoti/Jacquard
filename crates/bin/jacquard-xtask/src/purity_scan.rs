//! Token-stream scans for `check-core-purity` (crate inner attributes,
//! `std` references, and `#[path]` / `include*!` containment).

use std::path::{Path, PathBuf};

use crate::purity_lex::{TokKind, Token};

/// Checks whether leading crate inner attributes declare `#![no_std]` (or
/// `#![cfg_attr(not(test), no_std)]`) and `#![forbid(unsafe_code)]`.
#[must_use]
pub fn has_crate_inner_attrs(tokens: &[Token]) -> (bool, bool) {
    let mut has_no_std = false;
    let mut has_forbid_unsafe = false;
    let mut i = 0usize;
    while i + 2 < tokens.len()
        && tokens[i].kind == TokKind::Punct('#')
        && tokens[i + 1].kind == TokKind::Punct('!')
        && tokens[i + 2].kind == TokKind::Punct('[')
    {
        let Some(end) = find_matching(tokens, i + 2, '[', ']') else {
            break;
        };
        let inner = &tokens[i + 3..end];
        if (inner.len() == 1 && inner[0].kind == TokKind::Ident("no_std".into()))
            || is_cfg_attr_no_std(inner)
        {
            has_no_std = true;
        }
        if is_forbid_unsafe(inner) {
            has_forbid_unsafe = true;
        }
        i = end + 1;
    }
    (has_no_std, has_forbid_unsafe)
}

fn is_cfg_attr_no_std(tokens: &[Token]) -> bool {
    let want = [
        TokKind::Ident("cfg_attr".into()),
        TokKind::Punct('('),
        TokKind::Ident("not".into()),
        TokKind::Punct('('),
        TokKind::Ident("test".into()),
        TokKind::Punct(')'),
        TokKind::Punct(','),
        TokKind::Ident("no_std".into()),
        TokKind::Punct(')'),
    ];
    tokens.len() == want.len() && tokens.iter().zip(&want).all(|(a, b)| a.kind == *b)
}

fn is_forbid_unsafe(tokens: &[Token]) -> bool {
    let want = [
        TokKind::Ident("forbid".into()),
        TokKind::Punct('('),
        TokKind::Ident("unsafe_code".into()),
        TokKind::Punct(')'),
    ];
    tokens.len() == want.len() && tokens.iter().zip(&want).all(|(a, b)| a.kind == *b)
}

fn find_matching(tokens: &[Token], open_idx: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, tok) in tokens.iter().enumerate().skip(open_idx) {
        if tok.kind == TokKind::Punct(open) {
            depth += 1;
        } else if tok.kind == TokKind::Punct(close) {
            depth -= 1;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

/// Detects `extern crate std`, `use std` / `use ::std` (including use-trees),
/// and `std::` / `::std::` paths in tokenized Rust code.
#[must_use]
pub fn find_std_use(tokens: &[Token], source: &str) -> Option<(usize, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let line_text = |ln: usize| {
        lines
            .get(ln.saturating_sub(1))
            .map_or_else(String::new, |s| s.trim().to_string())
    };

    for i in 0..tokens.len() {
        if i + 2 < tokens.len()
            && tokens[i].kind == TokKind::Ident("extern".into())
            && tokens[i + 1].kind == TokKind::Ident("crate".into())
            && tokens[i + 2].kind == TokKind::Ident("std".into())
        {
            return Some((tokens[i].line, line_text(tokens[i].line)));
        }
        if tokens[i].kind == TokKind::Ident("use".into())
            && let Some(ln) = scan_use_tree(tokens, i + 1)
        {
            return Some((ln, line_text(ln)));
        }
        if i + 2 < tokens.len()
            && tokens[i].kind == TokKind::Ident("std".into())
            && tokens[i + 1].kind == TokKind::Punct(':')
            && tokens[i + 2].kind == TokKind::Punct(':')
        {
            let prev1 = i.checked_sub(1).and_then(|k| tokens.get(k));
            let is_leading_colons = i >= 2
                && tokens[i - 1].kind == TokKind::Punct(':')
                && tokens[i - 2].kind == TokKind::Punct(':')
                && !tokens.get(i.wrapping_sub(3)).is_some_and(|t| {
                    matches!(t.kind, TokKind::Ident(_) | TokKind::Punct(')' | '>' | ']'))
                });
            let is_bare_std = !prev1.is_some_and(|t| matches!(t.kind, TokKind::Punct(':' | '.')));
            if is_bare_std || is_leading_colons {
                return Some((tokens[i].line, line_text(tokens[i].line)));
            }
        }
    }
    None
}

fn scan_use_tree(tokens: &[Token], mut i: usize) -> Option<usize> {
    let mut brace_is_root: Vec<bool> = Vec::new();
    let mut at_segment_start = true;
    while i < tokens.len() {
        let tok = &tokens[i];
        if tok.kind == TokKind::Punct(';') {
            return None;
        }
        if at_segment_start {
            if i + 1 < tokens.len()
                && tokens[i].kind == TokKind::Punct(':')
                && tokens[i + 1].kind == TokKind::Punct(':')
            {
                i += 2;
                if i < tokens.len() && tokens[i].kind == TokKind::Ident("std".into()) {
                    return Some(tokens[i].line);
                }
                at_segment_start = false;
                continue;
            }
            if tok.kind == TokKind::Ident("std".into()) {
                return Some(tok.line);
            }
            if tok.kind == TokKind::Punct('{') {
                brace_is_root.push(true);
                at_segment_start = true;
                i += 1;
                continue;
            }
            at_segment_start = false;
        } else {
            match tok.kind {
                TokKind::Punct('{') => {
                    brace_is_root.push(false);
                    at_segment_start = false;
                }
                TokKind::Punct('}') => {
                    let _ = brace_is_root.pop();
                }
                TokKind::Punct(',') => {
                    at_segment_start = brace_is_root.last().copied().unwrap_or(false);
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Scans `#[path = "..."]` and `include*!(...)` directives, returning any
/// escaping/non-literal references and any valid contained Rust include targets.
#[must_use]
pub fn scan_source_directives(
    tokens: &[Token],
    file_path: &Path,
    canon_src_dir: &Path,
) -> (Vec<(usize, String)>, Vec<PathBuf>) {
    let mut escapes = Vec::new();
    let mut included = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        if tokens[i].kind == TokKind::Punct('#') {
            let bracket_at = if tokens
                .get(i + 1)
                .is_some_and(|t| t.kind == TokKind::Punct('!'))
            {
                i + 2
            } else {
                i + 1
            };
            if tokens
                .get(bracket_at)
                .is_some_and(|t| t.kind == TokKind::Punct('['))
                && let Some(end) = find_matching(tokens, bracket_at, '[', ']')
            {
                for k in (bracket_at + 1)..end {
                    let prev_ok = k == bracket_at + 1
                        || matches!(tokens[k - 1].kind, TokKind::Punct('(' | ','));
                    if prev_ok
                        && tokens[k].kind == TokKind::Ident("path".into())
                        && tokens
                            .get(k + 1)
                            .is_some_and(|t| t.kind == TokKind::Punct('='))
                    {
                        match tokens.get(k + 2) {
                            Some(Token {
                                kind: TokKind::Str(rel),
                                ..
                            }) => match resolve_contained(file_path, rel, canon_src_dir) {
                                Some(p) => included.push(p),
                                None => escapes.push((
                                    tokens[k].line,
                                    format!("#[path = {rel:?}] escapes crate src/"),
                                )),
                            },
                            _ => escapes.push((
                                tokens[k].line,
                                "#[path = ...] uses non-literal path".into(),
                            )),
                        }
                    }
                }
                i = end + 1;
                continue;
            }
        }

        if let TokKind::Ident(name) = &tokens[i].kind
            && matches!(name.as_str(), "include" | "include_str" | "include_bytes")
            && tokens
                .get(i + 1)
                .is_some_and(|t| t.kind == TokKind::Punct('!'))
            && let Some((open, close)) = tokens.get(i + 2).and_then(|t| match t.kind {
                TokKind::Punct('(') => Some(('(', ')')),
                TokKind::Punct('[') => Some(('[', ']')),
                TokKind::Punct('{') => Some(('{', '}')),
                _ => None,
            })
            && let Some(end) = find_matching(tokens, i + 2, open, close)
        {
            let args = &tokens[i + 3..end];
            if args.len() == 1
                && let TokKind::Str(rel) = &args[0].kind
                && let Some(p) = resolve_contained(file_path, rel, canon_src_dir)
            {
                if name == "include" {
                    included.push(p);
                }
            } else {
                escapes.push((
                    tokens[i].line,
                    format!("{name}! escapes crate src/ or uses non-literal path"),
                ));
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    (escapes, included)
}

fn resolve_contained(file_path: &Path, rel: &str, canon_src_dir: &Path) -> Option<PathBuf> {
    if rel.is_empty()
        || rel.contains('\0')
        || rel.starts_with('/')
        || rel.contains('\\')
        || rel.contains(':')
    {
        return None;
    }
    let base = file_path.parent().unwrap_or(canon_src_dir);
    let target = base.join(rel);
    let meta = std::fs::symlink_metadata(&target).ok()?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return None;
    }
    let canon = std::fs::canonicalize(&target).ok()?;
    canon.starts_with(canon_src_dir).then_some(canon)
}
