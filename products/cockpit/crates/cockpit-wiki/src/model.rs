//! Core strongly typed article, heading, feed, and error models for `cockpit-wiki`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Semantic section category of an authored Markdown document.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize, Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ArticleKind {
    /// Technical documentation or architecture wiki page.
    #[default]
    Wiki,
    /// Chronological essay or engineering log entry.
    Blog,
    /// Structured verifiable case record or engineering audit.
    Record,
    /// Reference manual or specification page.
    Reference,
}

impl ArticleKind {
    /// Canonical lowercase route prefix for this article kind.
    pub fn route_prefix(self) -> &'static str {
        match self {
            ArticleKind::Wiki => "/wiki",
            ArticleKind::Blog => "/blog",
            ArticleKind::Record => "/records",
            ArticleKind::Reference => "/docs",
        }
    }
}

/// Parsed document front-matter metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ArticleMeta {
    /// URL slug relative to `kind.route_prefix()`.
    pub slug: String,
    /// Primary article heading title.
    pub title: String,
    /// Concise summary used in index cards and RSS/Atom feeds.
    pub summary: String,
    /// Logical collection category.
    pub kind: ArticleKind,
    /// ISO-8601 publication date (`YYYY-MM-DD`).
    pub published_at: String,
    /// Optional ISO-8601 last-modified date (`YYYY-MM-DD`).
    pub updated_at: Option<String>,
    /// Author name or handle.
    pub author: String,
    /// Searchable topic tags.
    pub tags: Vec<String>,
    /// Optional evidence classification badge (`Architecture`, `Released Alpha`, `Verified`).
    pub evidence_class: Option<String>,
    /// True when the entry is unpublished and excluded from public feeds/indexes.
    pub draft: bool,
}

/// Extracted document heading entry used to render a Table of Contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    /// Heading depth (`1..=6`).
    pub level: u8,
    /// Unique lowercase HTML anchor fragment identifier.
    pub id: String,
    /// Plain heading label text.
    pub text: String,
}

/// Fully parsed and safely rendered Markdown article.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Article {
    /// Front-matter metadata.
    pub meta: ArticleMeta,
    /// Normalized relative source path (safe, forward-slash separated).
    pub source_path: String,
    /// Canonical route path combining `meta.kind.route_prefix()` and `meta.slug`.
    pub route: String,
    /// Rendered, raw-HTML-free body markup.
    pub html: String,
    /// Ordered headings extracted during rendering.
    pub toc: Vec<Heading>,
    /// Internal links extracted from Markdown (`/...` or relative `.md` targets).
    pub internal_links: Vec<String>,
    /// External HTTP(S) links extracted from Markdown.
    pub external_links: Vec<String>,
    /// True when at least one fenced `mermaid` diagram block is present.
    pub has_mermaid: bool,
    /// Approximate word count excluding Markdown punctuation.
    pub word_count: usize,
    /// Stripped raw Markdown body after front matter.
    pub raw_markdown: String,
}

/// Unresolved internal link found across a [`crate::WikiCatalog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    /// Route of the article containing the broken link.
    pub source_route: String,
    /// Normalized link target path that did not match any article or extra route.
    pub target: String,
}

/// Site-level metadata used to emit valid RSS 2.0 and Atom 1.0 syndication feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteFeedConfig {
    /// Feed and site title.
    pub site_title: String,
    /// Base canonical HTTPS origin without trailing slash (for example `https://example.org`).
    pub site_url: String,
    /// Concise feed subtitle/description.
    pub description: String,
    /// Default author identity for Atom `<author><name>`.
    pub author_name: String,
    /// BCP-47 language tag (for example `en-us`).
    pub language: String,
}

/// Errors emitted while validating paths, parsing front matter, or loading articles.
#[derive(Debug, Error)]
pub enum WikiError {
    /// Path attempted directory traversal, absolute path access, or invalid characters.
    #[error("unsafe wiki path rejected: {path} ({reason})")]
    UnsafePath {
        /// Offending path string.
        path: String,
        /// Rejection reason.
        reason: &'static str,
    },
    /// Filesystem I/O failure during catalog loading.
    #[error("wiki I/O error at {path}: {source}")]
    Io {
        /// Filesystem path involved.
        path: String,
        /// Underlying OS error.
        #[source]
        source: std::io::Error,
    },
    /// Malformed or incomplete front matter header.
    #[error("front matter parse error in {path}: {detail}")]
    FrontMatter {
        /// Logical path or identifier.
        path: String,
        /// Human-readable parse detail.
        detail: String,
    },
    /// Duplicate `(ArticleKind, slug)` route collision in catalog.
    #[error("duplicate article route `{route}` in `{path}`")]
    DuplicateRoute {
        /// Conflicting canonical route.
        route: String,
        /// Second article path causing the conflict.
        path: String,
    },
}
