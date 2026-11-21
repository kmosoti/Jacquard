//! Compile-time and path-traversal-safe filesystem wiki/blog loader & link verifier.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use crate::frontmatter::parse_document;
use crate::markdown::render_markdown;
use crate::model::{Article, ArticleKind, BrokenLink, WikiError};

/// Validates that `rel` is a safe, strictly normalized workspace-relative path.
///
/// Rejects NUL bytes, backslashes, leading slashes, Windows drive prefixes,
/// `.` / `..` segments, and control characters.
pub fn validate_safe_rel_path(rel: &str) -> Result<(), WikiError> {
    if rel.is_empty() {
        return Err(WikiError::UnsafePath {
            path: rel.to_string(),
            reason: "empty path",
        });
    }
    if rel.contains('\0') || rel.chars().any(char::is_control) {
        return Err(WikiError::UnsafePath {
            path: rel.to_string(),
            reason: "control characters or NUL byte",
        });
    }
    if rel.starts_with('/') || rel.contains('\\') || rel.contains(':') {
        return Err(WikiError::UnsafePath {
            path: rel.to_string(),
            reason: "absolute path, backslash, or drive prefix",
        });
    }
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(WikiError::UnsafePath {
                path: rel.to_string(),
                reason: "dot, parent-dir, or empty path segment",
            });
        }
    }
    Ok(())
}

/// Ordered collection of parsed Wiki, Blog, Record, and Reference articles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WikiCatalog {
    articles: Vec<Article>,
}

impl WikiCatalog {
    /// Builds a catalog from embedded `(rel_path, raw_markdown)` pairs (e.g. `include_str!`).
    pub fn from_embedded(entries: &[(&str, &str)]) -> Result<Self, WikiError> {
        let mut articles = Vec::with_capacity(entries.len());
        let mut seen_routes: HashSet<String> = HashSet::new();
        for &(rel_path, raw) in entries {
            validate_safe_rel_path(rel_path)?;
            let article = compile_article(rel_path, raw)?;
            if !seen_routes.insert(article.route.clone()) {
                return Err(WikiError::DuplicateRoute {
                    route: article.route,
                    path: rel_path.to_string(),
                });
            }
            articles.push(article);
        }
        sort_articles(&mut articles);
        Ok(Self { articles })
    }

    /// Recursively loads `.md` documents from `root`, blocking symlink & path-traversal escapes.
    pub fn from_dir(root: &Path) -> Result<Self, WikiError> {
        let canon_root = fs::canonicalize(root).map_err(|source| WikiError::Io {
            path: root.display().to_string(),
            source,
        })?;
        let mut raw_paths: Vec<(String, String)> = Vec::new();
        collect_dir_recursive(&canon_root, &canon_root, "", &mut raw_paths)?;
        raw_paths.sort_by(|a, b| a.0.cmp(&b.0));

        let mut articles = Vec::with_capacity(raw_paths.len());
        let mut seen_routes: HashSet<String> = HashSet::new();
        for (rel, content) in raw_paths {
            let article = compile_article(&rel, &content)?;
            if !seen_routes.insert(article.route.clone()) {
                return Err(WikiError::DuplicateRoute {
                    route: article.route,
                    path: rel,
                });
            }
            articles.push(article);
        }
        sort_articles(&mut articles);
        Ok(Self { articles })
    }

    /// Returns all non-draft articles matching `kind` (or all published articles if `None`).
    pub fn list(&self, kind: Option<ArticleKind>) -> Vec<&Article> {
        self.articles
            .iter()
            .filter(|a| !a.meta.draft && kind.is_none_or(|k| a.meta.kind == k))
            .collect()
    }

    /// Returns all articles including drafts.
    pub fn all_including_drafts(&self) -> &[Article] {
        &self.articles
    }

    /// Looks up a published article by canonical route (e.g. `/blog/release-notes`).
    pub fn find_by_route(&self, route: &str) -> Option<&Article> {
        let clean = route.trim_end_matches('/');
        let key = if clean.is_empty() { "/" } else { clean };
        self.articles
            .iter()
            .find(|a| !a.meta.draft && a.route == key)
    }

    /// Looks up a published article by `(ArticleKind, slug)`.
    pub fn find_by_slug(&self, kind: ArticleKind, slug: &str) -> Option<&Article> {
        self.articles
            .iter()
            .find(|a| !a.meta.draft && a.meta.kind == kind && a.meta.slug == slug)
    }

    /// Builds a pure-Rust BM25 inverted search index over published catalog articles.
    #[must_use]
    pub fn build_search_index(&self) -> crate::search::SearchIndex {
        crate::search::SearchIndex::build(&self.articles)
    }

    /// Checks every Markdown internal link against catalog routes and caller `extra_routes`.
    pub fn verify_internal_links(&self, extra_routes: &[&str]) -> Vec<BrokenLink> {
        let mut known: BTreeSet<String> = BTreeSet::new();
        for r in extra_routes {
            let norm = r.split('#').next().unwrap_or(r).trim_end_matches('/');
            known.insert(if norm.is_empty() { "/" } else { norm }.to_string());
        }
        for a in &self.articles {
            known.insert(a.route.clone());
            known.insert(a.meta.kind.route_prefix().to_string());
        }

        let mut broken = Vec::new();
        for a in &self.articles {
            for link in &a.internal_links {
                let target_no_frag = link
                    .split('#')
                    .next()
                    .unwrap_or(link)
                    .split('?')
                    .next()
                    .unwrap_or(link)
                    .trim_end_matches('/');
                let norm = if target_no_frag.is_empty() {
                    "/"
                } else {
                    target_no_frag
                };
                if !known.contains(norm) {
                    broken.push(BrokenLink {
                        source_route: a.route.clone(),
                        target: link.clone(),
                    });
                }
            }
        }
        broken
    }
}

fn compile_article(rel_path: &str, raw: &str) -> Result<Article, WikiError> {
    let (meta, body) = parse_document(rel_path, raw)?;
    let rendered = render_markdown(body);
    let prefix = meta.kind.route_prefix().trim_end_matches('/');
    let route = format!("{prefix}/{}", meta.slug);
    Ok(Article {
        meta,
        source_path: rel_path.to_string(),
        route,
        html: rendered.html,
        toc: rendered.toc,
        internal_links: rendered.internal_links,
        external_links: rendered.external_links,
        has_mermaid: rendered.has_mermaid,
        word_count: rendered.word_count,
        raw_markdown: body.to_string(),
    })
}

fn sort_articles(articles: &mut [Article]) {
    articles.sort_by(|a, b| {
        b.meta
            .published_at
            .cmp(&a.meta.published_at)
            .then_with(|| a.meta.slug.cmp(&b.meta.slug))
    });
}

fn collect_dir_recursive(
    canon_root: &Path,
    dir: &Path,
    rel_prefix: &str,
    out: &mut Vec<(String, String)>,
) -> Result<(), WikiError> {
    let entries = fs::read_dir(dir).map_err(|source| WikiError::Io {
        path: dir.display().to_string(),
        source,
    })?;

    for entry_res in entries {
        let entry = entry_res.map_err(|source| WikiError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|source| WikiError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let canon = fs::canonicalize(&path).map_err(|source| WikiError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if !canon.starts_with(canon_root) {
            return Err(WikiError::UnsafePath {
                path: path.display().to_string(),
                reason: "resolved path escapes catalog root",
            });
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let rel = if rel_prefix.is_empty() {
            name.clone()
        } else {
            format!("{rel_prefix}/{name}")
        };
        if meta.is_dir() {
            collect_dir_recursive(canon_root, &canon, &rel, out)?;
        } else if meta.is_file() && (name.ends_with(".md") || name.ends_with(".markdown")) {
            validate_safe_rel_path(&rel)?;
            let content = fs::read_to_string(&canon).map_err(|source| WikiError::Io {
                path: canon.display().to_string(),
                source,
            })?;
            out.push((rel, content));
        }
    }
    Ok(())
}
