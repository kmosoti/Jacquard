#![forbid(unsafe_code)]
//! Pure-Rust compile-time and runtime Markdown + Mermaid + syntax-safe Wiki/Blog engine.
//!
//! Designed for both interactive Cockpit documentation portals and standalone
//! SSR/Islands sites: parses `+++` TOML and `---` key-value front matter, renders
//! sanitized HTML with anchor headings, ToC extraction, `<pre class="mermaid">`
//! fenced diagram passthrough, path-traversal-safe filesystem or compile-time
//! loading, internal link verification, and deterministic RSS 2.0 / Atom 1.0 feeds.

pub mod feed;
pub mod frontmatter;
pub mod highlight;
pub mod loader;
pub mod markdown;
pub mod model;
mod sanitize;
pub mod search;

pub use feed::{render_atom, render_rss};
pub use frontmatter::{parse_document, slug_from_rel_path};
pub use highlight::{highlight_code, render_mermaid_block};
pub use loader::{WikiCatalog, validate_safe_rel_path};
pub use markdown::{RenderedMarkdown, render_markdown};
pub use model::{
    Article, ArticleKind, ArticleMeta, BrokenLink, Heading, SiteFeedConfig, WikiError,
};
pub use search::{SearchHit, SearchIndex};
