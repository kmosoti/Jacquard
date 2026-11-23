//! Integration tests for `cockpit-wiki` Markdown/Mermaid rendering, feeds, and path safety.

use cockpit_wiki::{
    ArticleKind, SiteFeedConfig, WikiCatalog, WikiError, render_atom, render_markdown, render_rss,
    validate_safe_rel_path,
};

#[test]
fn embedded_wiki_renders_mermaid_toc_and_blocks_xss() {
    let catalog = WikiCatalog::from_embedded(&[
        (
            "wiki/architecture.md",
            "+++\ntitle = \"Cockpit Architecture\"\nslug = \"architecture\"\nkind = \"wiki\"\npublished_at = \"2026-11-21\"\nsummary = \"Pure-Rust islands architecture\"\ntags = [\"rust\", \"leptos\"]\n+++\n\n# Overview\n\n<script>alert(1)</script>\n\n[Bad](javascript:alert(1))\n\n[Notes](/blog/release-notes)\n\n```mermaid\nflowchart LR\n  A[Core] --> B[SVG]\n```\n",
        ),
        (
            "blog/release-notes.md",
            "---\ntitle: \"Alpha Release Notes\"\nslug: \"release-notes\"\nkind: \"blog\"\npublished_at: \"2026-11-20\"\nsummary: \"Stage A engineering notes\"\n---\n\n## Highlights\n\nClean zero-JS SSR baseline.\n",
        ),
    ])
    .expect("valid catalog");

    let arch = catalog
        .find_by_route("/wiki/architecture")
        .expect("wiki article exists");
    assert!(arch.has_mermaid);
    assert!(arch.html.contains("<figure class=\"wiki-diagram\""));
    assert!(arch.html.contains("<svg "));
    assert!(arch.html.contains("Core"));
    assert!(arch.html.contains("SVG"));
    assert!(!arch.html.contains("<script>"));
    assert!(!arch.html.contains("javascript:"));
    assert_eq!(arch.toc.len(), 1);
    assert_eq!(arch.toc[0].id, "overview");
    assert_eq!(arch.internal_links, vec!["/blog/release-notes"]);
    assert!(catalog.verify_internal_links(&["/"]).is_empty());

    let blog = catalog
        .find_by_slug(ArticleKind::Blog, "release-notes")
        .expect("blog entry exists");
    assert_eq!(blog.meta.title, "Alpha Release Notes");
}

#[test]
fn path_traversal_and_unsafe_paths_are_rejected() {
    for bad in [
        "../secret.md",
        "wiki/../../etc/passwd.md",
        "/etc/passwd.md",
        "wiki\\win.md",
        "a\0b.md",
        "",
        "wiki/./page.md",
    ] {
        let err = validate_safe_rel_path(bad).expect_err("must reject unsafe path");
        assert!(matches!(err, WikiError::UnsafePath { .. }));
    }
}

#[test]
fn rss_and_atom_feeds_escape_xml_and_filter_drafts() {
    let catalog = WikiCatalog::from_embedded(&[
        (
            "blog/alpha.md",
            "+++\ntitle = \"Release & Notes <Alpha>\"\nslug = \"alpha\"\nkind = \"blog\"\npublished_at = \"2026-11-21\"\nsummary = \"Details & benchmarks\"\n+++\n\nBody.\n",
        ),
        (
            "blog/wip.md",
            "+++\ntitle = \"Hidden Draft\"\nslug = \"wip\"\nkind = \"blog\"\npublished_at = \"2026-11-21\"\ndraft = true\n+++\n\nHidden.\n",
        ),
    ])
    .expect("catalog");
    let cfg = SiteFeedConfig {
        site_title: "Example Engineering".into(),
        site_url: "https://example.org".into(),
        description: "Engineering notes".into(),
        author_name: "Maintainer".into(),
        language: "en-us".into(),
    };

    let list = catalog.list(Some(ArticleKind::Blog));
    let rss = render_rss(&list, &cfg, Some(ArticleKind::Blog));
    assert!(rss.contains("Release &amp; Notes &lt;Alpha&gt;"));
    assert!(!rss.contains("Hidden Draft"));
    assert!(rss.contains("<pubDate>21 Nov 2026 00:00:00 +0000</pubDate>"));

    let atom = render_atom(&list, &cfg, Some(ArticleKind::Blog));
    assert!(atom.contains("<updated>2026-11-21T00:00:00Z</updated>"));
    assert!(!atom.contains("Hidden Draft"));
}

#[test]
fn markdown_detects_broken_internal_links_and_blocks_data_urls() {
    let catalog = WikiCatalog::from_embedded(&[(
        "wiki/index.md",
        "# Home\n\n[Missing](/wiki/nonexistent)\n\n![Img](data:text/html,<svg/>)\n",
    )])
    .expect("catalog");
    let broken = catalog.verify_internal_links(&[]);
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].target, "/wiki/nonexistent");

    let rendered = render_markdown("[Click](  JavaScript:alert(1))");
    assert!(rendered.html.contains("#blocked-unsafe-scheme"));
}

#[test]
fn multi_table_closes_tbody_inside_each_table_not_at_eof() {
    let md = "| A | B |\n|---|---|\n| 1 | 2 |\n\nParagraph between tables.\n\n| C | D |\n|---|---|\n| 3 | 4 |\n\nTrailing note.\n";
    let rendered = render_markdown(md);
    assert_eq!(rendered.html.matches("<tbody>").count(), 2);
    assert_eq!(rendered.html.matches("</tbody></table>").count(), 2);
    assert!(
        !rendered.html.trim_end().ends_with("</tbody>"),
        "tbody must close inside each table, not at EOF: {}",
        rendered.html
    );
}

#[test]
fn url_scheme_allowlist_permits_safe_schemes_and_blocks_all_others() {
    let md = "[HTTPS](https://example.org) [Mail](mailto:hi@example.org) [Frag](#sec) [Root](/wiki/a) \
              [File](file:///etc/passwd) [Blob](blob:https://example.org/x) [ProtoRel](//evil.example/x)";
    let rendered = render_markdown(md);
    assert!(rendered.html.contains("href=\"https://example.org\""));
    assert!(rendered.html.contains("href=\"mailto:hi@example.org\""));
    assert!(rendered.html.contains("href=\"#sec\""));
    assert!(rendered.html.contains("href=\"/wiki/a\""));
    assert!(!rendered.html.contains("file://"));
    assert!(!rendered.html.contains("blob:"));
    assert!(!rendered.html.contains("//evil.example"));
    assert_eq!(rendered.html.matches("#blocked-unsafe-scheme").count(), 3);
}

#[test]
fn server_side_syntax_highlighting_and_mermaid_fallback_require_zero_js() {
    let md = "```rust\npub fn main() { let x = 42; // hi\n}\n```\n\n```toml\n[bundle]\nwasm_max_bytes = 256000\n```\n\n```mermaid\nsequenceDiagram\n  A->>B: ping\n```\n";
    let rendered = render_markdown(md);
    assert!(rendered.html.contains("<span class=\"tok-kw\">fn</span>"));
    assert!(rendered.html.contains("<span class=\"tok-num\">42</span>"));
    assert!(
        rendered
            .html
            .contains("<span class=\"tok-com\">// hi</span>")
    );
    assert!(
        rendered
            .html
            .contains("<span class=\"tok-type\">[bundle]</span>")
    );
    assert!(rendered.html.contains("data-renderer=\"fallback\""));
    assert!(rendered.html.contains("<figure class=\"wiki-diagram\""));
}

#[test]
fn bm25_search_index_ranks_relevant_articles_deterministically() {
    let catalog = WikiCatalog::from_embedded(&[
        (
            "wiki/sugiyama.md",
            "+++\ntitle = \"Sugiyama Layered Layout\"\nslug = \"sugiyama\"\nkind = \"wiki\"\npublished_at = \"2026-11-22\"\nsummary = \"Crossing minimization with virtual nodes\"\ntags = [\"layout\", \"graph\"]\n+++\n\nSugiyama sweeps minimize edge crossings in 2.5D isometric diagrams.\n",
        ),
        (
            "wiki/bm25.md",
            "+++\ntitle = \"BM25 Full-Text Search\"\nslug = \"bm25\"\nkind = \"wiki\"\npublished_at = \"2026-11-22\"\nsummary = \"Pure-Rust inverted index\"\ntags = [\"search\", \"rust\"]\n+++\n\nOkapi BM25 scores term frequency and inverse document frequency without JS.\n",
        ),
        (
            "wiki/draft.md",
            "+++\ntitle = \"Draft BM25 Notes\"\nslug = \"draft\"\nkind = \"wiki\"\npublished_at = \"2026-11-22\"\ndraft = true\n+++\n\nBM25 BM25 BM25.\n",
        ),
    ])
    .expect("catalog");
    let index = catalog.build_search_index();
    assert_eq!(index.doc_count(), 2);
    let hits = index.search("okapi bm25 inverted", 5);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].route, "/wiki/bm25");
    assert!(hits[0].score > 0.0);
}

#[test]
fn nested_image_link_and_unsafe_urls_cannot_inject_attributes() {
    let payload = "![outer [inner](<onerror=alert(document.domain) x>)](missing.png)";
    let rendered = render_markdown(payload);
    assert!(
        rendered.html.contains(r#"alt="outer inner""#),
        "expected flattened plain-text alt, got: {}",
        rendered.html
    );
    assert!(!rendered.html.contains("<a "));
    assert!(!rendered.html.contains("onerror="));

    let rich_alt = "![a *b* `c` [link](https://example.org) ![nest](x.png)](/img.png)";
    let rendered_rich = render_markdown(rich_alt);
    assert!(
        rendered_rich.html.contains(r#"alt="a b c link nest""#),
        "unexpected alt output: {}",
        rendered_rich.html
    );

    for bad in [
        r#"[q](<foo"onerror="alert(1)>)"#,
        "[sp](<foo bar>)",
        "[ang](<onerror=alert(1) x>)",
        "[bs](<foo\\bar>)",
        "[ws](<https://example.org/a b>)",
    ] {
        let out = render_markdown(bad);
        assert!(
            out.html.contains(r##"href="#blocked-unsafe-scheme""##),
            "unsafe URL {bad} was not blocked: {}",
            out.html
        );
    }
}

#[test]
fn mermaid_budgets_reject_oversized_graphs_into_fallback() {
    use std::fmt::Write as _;

    // > 16 KiB bytes
    let mut huge_bytes = String::from("```mermaid\nflowchart LR\n");
    while huge_bytes.len() <= 17 * 1024 {
        huge_bytes.push_str("  A[NodeA] --> B[NodeB]\n");
    }
    huge_bytes.push_str("```\n");
    let r_bytes = render_markdown(&huge_bytes);
    assert!(r_bytes.html.contains(r#"data-renderer="fallback""#));
    assert!(!r_bytes.html.contains("<svg "));

    // > 64 nodes (65 nodes across 64 edges)
    let mut too_many_nodes = String::from("```mermaid\nflowchart LR\n");
    for i in 0..64 {
        let _ = writeln!(too_many_nodes, "  N{i}[N{i}] --> N{}[N{}]", i + 1, i + 1);
    }
    too_many_nodes.push_str("```\n");
    let r_nodes = render_markdown(&too_many_nodes);
    assert!(r_nodes.html.contains(r#"data-renderer="fallback""#));
    assert!(!r_nodes.html.contains("<svg "));

    // > 128 edges (129 edges among 10 nodes)
    let mut too_many_edges = String::from("```mermaid\nflowchart LR\n");
    for i in 0..129 {
        let _ = writeln!(too_many_edges, "  A{} --> A{}", i % 10, (i + 1) % 10);
    }
    too_many_edges.push_str("```\n");
    let r_edges = render_markdown(&too_many_edges);
    assert!(r_edges.html.contains(r#"data-renderer="fallback""#));
    assert!(!r_edges.html.contains("<svg "));

    // > 128 char label
    let long_label = "x".repeat(129);
    let long_lbl_md = format!("```mermaid\nflowchart LR\n  A[{long_label}] --> B[ok]\n```\n");
    let r_lbl = render_markdown(&long_lbl_md);
    assert!(r_lbl.html.contains(r#"data-renderer="fallback""#));
    assert!(!r_lbl.html.contains("<svg "));
}
