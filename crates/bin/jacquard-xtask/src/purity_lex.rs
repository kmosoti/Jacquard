//! Minimal Rust lexical tokenizer for `check-core-purity`.
//!
//! Strips line and nested block comments and parses raw/quoted strings and
//! character literals vs lifetimes so spoofed attributes or `std` mentions
//! inside comments or strings are ignored.

/// A single lexical token with its 1-based source line number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// Token classification.
    pub kind: TokKind,
    /// 1-based source line number.
    pub line: usize,
}

/// Token classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokKind {
    /// An identifier or keyword (with raw `r#` prefix stripped).
    Ident(String),
    /// A single punctuation character.
    Punct(char),
    /// A string literal (`"..."`, `r#"..."#`, `b"..."`, `br#"..."#`, etc.).
    Str(String),
    /// Any other literal (numbers, character literals).
    Other,
}

/// Tokenizes Rust source code, stripping line and nested block comments.
#[must_use]
pub fn tokenize(src: &str) -> Vec<Token> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'/' && bytes.get(i + 1) == Some(&b'*') {
            (i, line) = skip_block_comment(bytes, i + 2, line);
            continue;
        }
        if let Some((val, next_i, added)) = try_lex_raw_string(bytes, i) {
            out.push(Token {
                kind: TokKind::Str(val),
                line,
            });
            line += added;
            i = next_i;
            continue;
        }
        if let Some((val, next_i, added)) = try_lex_quoted_string(src, i) {
            out.push(Token {
                kind: TokKind::Str(val),
                line,
            });
            line += added;
            i = next_i;
            continue;
        }
        if b == b'r'
            && bytes.get(i + 1) == Some(&b'#')
            && bytes
                .get(i + 2)
                .is_some_and(|&c| c.is_ascii_alphabetic() || c == b'_')
        {
            let start = i + 2;
            let mut end = start + 1;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            out.push(Token {
                kind: TokKind::Ident(src[start..end].to_string()),
                line,
            });
            i = end;
            continue;
        }
        if b == b'\'' {
            if let Some(next_i) = try_lex_char_lit(src, i) {
                out.push(Token {
                    kind: TokKind::Other,
                    line,
                });
                i = next_i;
            } else {
                out.push(Token {
                    kind: TokKind::Punct('\''),
                    line,
                });
                i += 1;
            }
            continue;
        }
        if b.is_ascii_digit() {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            out.push(Token {
                kind: TokKind::Other,
                line,
            });
            continue;
        }

        let (tok, next_i) = lex_ident_or_punct(src, i, line);
        out.push(tok);
        i = next_i;
    }
    out
}

fn lex_ident_or_punct(src: &str, mut i: usize, line: usize) -> (Token, usize) {
    let Some(ch) = src[i..].chars().next() else {
        return (
            Token {
                kind: TokKind::Other,
                line,
            },
            src.len(),
        );
    };
    if ch.is_alphabetic() || ch == '_' {
        let start = i;
        i += ch.len_utf8();
        while let Some(next_ch) = src[i..].chars().next() {
            if next_ch.is_alphanumeric() || next_ch == '_' {
                i += next_ch.len_utf8();
            } else {
                break;
            }
        }
        (
            Token {
                kind: TokKind::Ident(src[start..i].to_string()),
                line,
            },
            i,
        )
    } else {
        (
            Token {
                kind: TokKind::Punct(ch),
                line,
            },
            i + ch.len_utf8(),
        )
    }
}

fn skip_block_comment(bytes: &[u8], mut i: usize, mut line: usize) -> (usize, usize) {
    let mut depth = 1usize;
    while i < bytes.len() && depth > 0 {
        if bytes[i] == b'\n' {
            line += 1;
            i += 1;
        } else if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
            depth -= 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    (i, line)
}

fn try_lex_raw_string(bytes: &[u8], i: usize) -> Option<(String, usize, usize)> {
    let pos = if bytes.get(i) == Some(&b'r') {
        i + 1
    } else if (bytes.get(i) == Some(&b'b') || bytes.get(i) == Some(&b'c'))
        && bytes.get(i + 1) == Some(&b'r')
    {
        i + 2
    } else {
        return None;
    };
    let mut hashes = 0usize;
    while bytes.get(pos + hashes) == Some(&b'#') {
        hashes += 1;
    }
    if bytes.get(pos + hashes) != Some(&b'"') {
        return None;
    }
    let content_start = pos + hashes + 1;
    let mut j = content_start;
    let mut added_lines = 0usize;
    while j < bytes.len() {
        if bytes[j] == b'\n' {
            added_lines += 1;
        }
        if bytes[j] == b'"'
            && j + 1 + hashes <= bytes.len()
            && bytes[j + 1..j + 1 + hashes].iter().all(|&c| c == b'#')
        {
            let raw = String::from_utf8_lossy(&bytes[content_start..j]).into_owned();
            return Some((raw, j + 1 + hashes, added_lines));
        }
        j += 1;
    }
    Some((
        String::from_utf8_lossy(&bytes[content_start..]).into_owned(),
        bytes.len(),
        added_lines,
    ))
}

fn try_lex_quoted_string(src: &str, i: usize) -> Option<(String, usize, usize)> {
    let bytes = src.as_bytes();
    let quote_at = if bytes.get(i) == Some(&b'"') {
        i
    } else if (bytes.get(i) == Some(&b'b') || bytes.get(i) == Some(&b'c'))
        && bytes.get(i + 1) == Some(&b'"')
    {
        i + 1
    } else {
        return None;
    };
    let mut j = quote_at + 1;
    let mut val = String::new();
    let mut added_lines = 0usize;
    while j < src.len() {
        let ch = src[j..].chars().next()?;
        if ch == '\n' {
            added_lines += 1;
        }
        if ch == '\\' {
            j += 1;
            if let Some(esc) = src[j..].chars().next() {
                if esc == '\n' {
                    added_lines += 1;
                }
                val.push(esc);
                j += esc.len_utf8();
                continue;
            }
            break;
        }
        if ch == '"' {
            return Some((val, j + 1, added_lines));
        }
        val.push(ch);
        j += ch.len_utf8();
    }
    Some((val, src.len(), added_lines))
}

fn try_lex_char_lit(src: &str, i: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    if bytes.get(i + 1) == Some(&b'\\') {
        let mut j = i + 2;
        while j < bytes.len() && bytes[j] != b'\n' {
            if bytes[j] == b'\'' {
                return Some(j + 1);
            }
            j += 1;
        }
        return None;
    }
    let ch = src.get(i + 1..)?.chars().next()?;
    let after = i + 1 + ch.len_utf8();
    (bytes.get(after) == Some(&b'\'')).then_some(after + 1)
}
