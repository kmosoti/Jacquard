//! TOML (`+++`) and key-value (`---`) front-matter parser and slug normalizer.

use crate::model::{ArticleKind, ArticleMeta, WikiError};

/// Splits a raw Markdown document into optional front-matter metadata and body text.
pub fn parse_document<'a>(
    rel_path: &str,
    raw: &'a str,
) -> Result<(ArticleMeta, &'a str), WikiError> {
    let normalized = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let (meta, body) = if let Some(rest) = strip_fence(normalized, "+++") {
        let (fm, body) = split_closing_fence(rest, "+++", rel_path)?;
        let meta: ArticleMeta = toml::from_str(fm).map_err(|e| WikiError::FrontMatter {
            path: rel_path.to_string(),
            detail: e.to_string(),
        })?;
        (meta, body)
    } else if let Some(rest) = strip_fence(normalized, "---") {
        let (fm, body) = split_closing_fence(rest, "---", rel_path)?;
        let meta = parse_yaml_like_frontmatter(fm, rel_path)?;
        (meta, body)
    } else {
        (ArticleMeta::default(), normalized)
    };

    let mut meta = meta;
    if meta.slug.trim().is_empty() {
        meta.slug = slug_from_rel_path(rel_path);
    } else {
        meta.slug = normalize_slug(&meta.slug);
    }
    if meta.title.trim().is_empty() {
        meta.title = infer_title_from_body(body).unwrap_or_else(|| meta.slug.clone());
    }
    if meta.kind == ArticleKind::Wiki {
        meta.kind = infer_kind_from_path(rel_path, meta.kind);
    }
    Ok((meta, body))
}

/// Converts a validated relative Markdown path (e.g. `blog/hello-world.md`) to a URL slug.
pub fn slug_from_rel_path(rel_path: &str) -> String {
    let trimmed = rel_path.trim_matches('/');
    let without_ext = trimmed
        .strip_suffix(".md")
        .or_else(|| trimmed.strip_suffix(".markdown"))
        .unwrap_or(trimmed);
    let stem = without_ext
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("index");
    let base = if stem == "README" || stem == "index" {
        without_ext
            .rsplit('/')
            .nth(1)
            .filter(|s| !s.is_empty())
            .unwrap_or("index")
    } else {
        stem
    };
    normalize_slug(base)
}

fn normalize_slug(raw: &str) -> String {
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
        "article".to_string()
    } else {
        out
    }
}

fn infer_kind_from_path(rel_path: &str, default_kind: ArticleKind) -> ArticleKind {
    let first = rel_path
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("");
    match first {
        "blog" | "posts" => ArticleKind::Blog,
        "records" => ArticleKind::Record,
        "docs" | "reference" => ArticleKind::Reference,
        _ => default_kind,
    }
}

fn infer_title_from_body(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .filter(|s| !s.is_empty())
}

fn strip_fence<'a>(input: &'a str, fence: &str) -> Option<&'a str> {
    let after = input.strip_prefix(fence)?;
    if let Some(rest) = after.strip_prefix("\r\n") {
        Some(rest)
    } else {
        after.strip_prefix('\n')
    }
}

fn split_closing_fence<'a>(
    input: &'a str,
    fence: &str,
    path: &str,
) -> Result<(&'a str, &'a str), WikiError> {
    let mut offset = 0usize;
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == fence {
            let fm = &input[..offset];
            let body = &input[offset + line.len()..];
            return Ok((fm, body));
        }
        offset += line.len();
    }
    if input.trim_end() == fence {
        return Ok(("", ""));
    }
    Err(WikiError::FrontMatter {
        path: path.to_string(),
        detail: format!("unterminated `{fence}` front matter delimiter"),
    })
}

fn parse_yaml_like_frontmatter(fm: &str, path: &str) -> Result<ArticleMeta, WikiError> {
    let mut meta = ArticleMeta::default();
    for (idx, raw_line) in fm.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, val)) = line.split_once(':') else {
            return Err(WikiError::FrontMatter {
                path: path.to_string(),
                detail: format!("line {}: expected `key: value`", idx + 1),
            });
        };
        let key = key.trim();
        let val = unquote(val.trim());
        match key {
            "slug" => meta.slug = val.to_string(),
            "title" => meta.title = val.to_string(),
            "summary" | "description" => meta.summary = val.to_string(),
            "kind" => meta.kind = parse_kind(val, path)?,
            "published_at" | "date" => meta.published_at = val.to_string(),
            "updated_at" | "updated" => meta.updated_at = Some(val.to_string()),
            "author" => meta.author = val.to_string(),
            "evidence_class" => meta.evidence_class = Some(val.to_string()),
            "draft" => {
                meta.draft = val.eq_ignore_ascii_case("true");
            }
            "tags" => meta.tags = parse_inline_list(val),
            _ => {}
        }
    }
    Ok(meta)
}

fn parse_kind(raw: &str, path: &str) -> Result<ArticleKind, WikiError> {
    match raw.to_ascii_lowercase().as_str() {
        "wiki" => Ok(ArticleKind::Wiki),
        "blog" => Ok(ArticleKind::Blog),
        "record" => Ok(ArticleKind::Record),
        "reference" | "docs" => Ok(ArticleKind::Reference),
        other => Err(WikiError::FrontMatter {
            path: path.to_string(),
            detail: format!("unknown article kind `{other}`"),
        }),
    }
}

fn unquote(s: &str) -> &str {
    if ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
        && s.len() >= 2
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn parse_inline_list(raw: &str) -> Vec<String> {
    let inner = raw
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(raw);
    inner
        .split(',')
        .map(|item| unquote(item.trim()).trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}
