//! Deterministic RSS 2.0 and Atom 1.0 syndication feed generators.

use std::fmt::Write;

use crate::markdown::escape_html_into;
use crate::model::{Article, ArticleKind, SiteFeedConfig};

/// Renders an RSS 2.0 XML document for published entries (defaults to `Blog` entries if `kind` is `Some`).
pub fn render_rss(
    articles: &[&Article],
    config: &SiteFeedConfig,
    kind_filter: Option<ArticleKind>,
) -> String {
    let base = config.site_url.trim_end_matches('/');
    let mut xml = String::with_capacity(4096);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<rss version=\"2.0\">\n<channel>\n");
    write_tag(&mut xml, "title", &config.site_title);
    write_tag(&mut xml, "link", base);
    write_tag(&mut xml, "description", &config.description);
    write_tag(&mut xml, "language", &config.language);

    for article in articles {
        if article.meta.draft || kind_filter.is_some_and(|k| article.meta.kind != k) {
            continue;
        }
        let item_url = format!("{base}{}", article.route);
        xml.push_str("  <item>\n");
        write_tag_indent(&mut xml, "    ", "title", &article.meta.title);
        write_tag_indent(&mut xml, "    ", "link", &item_url);
        write_tag_indent(&mut xml, "    ", "guid", &item_url);
        if !article.meta.published_at.is_empty() {
            let rfc822 = iso_date_to_rfc822(&article.meta.published_at);
            write_tag_indent(&mut xml, "    ", "pubDate", &rfc822);
        }
        let desc = if article.meta.summary.is_empty() {
            article.meta.title.as_str()
        } else {
            article.meta.summary.as_str()
        };
        write_tag_indent(&mut xml, "    ", "description", desc);
        xml.push_str("  </item>\n");
    }

    xml.push_str("</channel>\n</rss>\n");
    xml
}

/// Renders an Atom 1.0 XML document for published entries.
pub fn render_atom(
    articles: &[&Article],
    config: &SiteFeedConfig,
    kind_filter: Option<ArticleKind>,
) -> String {
    let base = config.site_url.trim_end_matches('/');
    let feed_id = format!("{base}/feed.xml");
    let updated = articles
        .iter()
        .filter(|a| !a.meta.draft && kind_filter.is_none_or(|k| a.meta.kind == k))
        .filter_map(|a| {
            a.meta
                .updated_at
                .as_deref()
                .or(Some(a.meta.published_at.as_str()))
        })
        .filter(|d| !d.is_empty())
        .max()
        .unwrap_or("2026-01-01");

    let mut xml = String::with_capacity(4096);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
    write_tag(&mut xml, "title", &config.site_title);
    write_tag(&mut xml, "subtitle", &config.description);
    write_tag(&mut xml, "id", &feed_id);
    let updated_rfc = iso_date_to_rfc3339(updated);
    write_tag(&mut xml, "updated", &updated_rfc);
    xml.push_str("  <link href=\"");
    escape_html_into(&mut xml, base);
    xml.push_str("\" />\n  <link href=\"");
    escape_html_into(&mut xml, &feed_id);
    xml.push_str("\" rel=\"self\" />\n  <author><name>");
    escape_html_into(&mut xml, &config.author_name);
    xml.push_str("</name></author>\n");

    for article in articles {
        if article.meta.draft || kind_filter.is_some_and(|k| article.meta.kind != k) {
            continue;
        }
        let item_url = format!("{base}{}", article.route);
        let entry_date = article
            .meta
            .updated_at
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(if article.meta.published_at.is_empty() {
                "2026-01-01"
            } else {
                &article.meta.published_at
            });
        xml.push_str("  <entry>\n");
        write_tag_indent(&mut xml, "    ", "title", &article.meta.title);
        write_tag_indent(&mut xml, "    ", "id", &item_url);
        xml.push_str("    <link href=\"");
        escape_html_into(&mut xml, &item_url);
        xml.push_str("\" />\n");
        write_tag_indent(
            &mut xml,
            "    ",
            "updated",
            &iso_date_to_rfc3339(entry_date),
        );
        let summary = if article.meta.summary.is_empty() {
            article.meta.title.as_str()
        } else {
            article.meta.summary.as_str()
        };
        write_tag_indent(&mut xml, "    ", "summary", summary);
        xml.push_str("  </entry>\n");
    }

    xml.push_str("</feed>\n");
    xml
}

fn write_tag(buf: &mut String, tag: &str, value: &str) {
    write_tag_indent(buf, "  ", tag, value);
}

fn write_tag_indent(buf: &mut String, indent: &str, tag: &str, value: &str) {
    let _ = write!(buf, "{indent}<{tag}>");
    escape_html_into(buf, value);
    let _ = writeln!(buf, "</{tag}>");
}

fn iso_date_to_rfc3339(date: &str) -> String {
    let trimmed = date.trim();
    if trimmed.contains('T') {
        trimmed.to_string()
    } else if trimmed.len() == 10 {
        format!("{trimmed}T00:00:00Z")
    } else {
        "2026-01-01T00:00:00Z".to_string()
    }
}

fn iso_date_to_rfc822(date: &str) -> String {
    let parts: Vec<&str> = date.trim().split('-').collect();
    if parts.len() == 3 {
        let year = parts[0];
        let month = match parts[1] {
            "01" => "Jan",
            "02" => "Feb",
            "03" => "Mar",
            "04" => "Apr",
            "05" => "May",
            "06" => "Jun",
            "07" => "Jul",
            "08" => "Aug",
            "09" => "Sep",
            "10" => "Oct",
            "11" => "Nov",
            "12" => "Dec",
            _ => "Jan",
        };
        let day = parts[2].get(..2).unwrap_or("01");
        format!("{day} {month} {year} 00:00:00 +0000")
    } else {
        "01 Jan 2026 00:00:00 +0000".to_string()
    }
}
