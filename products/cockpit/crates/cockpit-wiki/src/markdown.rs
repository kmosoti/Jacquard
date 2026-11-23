//! Syntax-safe Markdown renderer with native `iso-diagram` Mermaid flowchart
//! rendering, server-side syntax highlighting, strict URL scheme allowlisting,
//! deterministic heading anchors, ToC extraction, and link collection.

use std::collections::HashMap;
use std::fmt::Write;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::highlight::{highlight_code, render_mermaid_block};
use crate::model::Heading;
pub(crate) use crate::sanitize::escape_html_into;
use crate::sanitize::{ImageCapture, dedup_id, sanitize_link_url, slugify};

/// Output of rendering Markdown through [`render_markdown`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderedMarkdown {
    /// Sanitized HTML fragment string (raw HTML blocks stripped, strict URL allowlist).
    pub html: String,
    /// Ordered headings with deduplicated anchor IDs.
    pub toc: Vec<Heading>,
    /// Internal route/path links referenced in the document.
    pub internal_links: Vec<String>,
    /// External HTTP(S) links referenced in the document.
    pub external_links: Vec<String>,
    /// True if one or more ```` ```mermaid ```` blocks were rendered.
    pub has_mermaid: bool,
    /// Approximate body word count.
    pub word_count: usize,
}

struct HeadingCapture {
    level: HeadingLevel,
    explicit_id: Option<String>,
    text: String,
    html_buf: String,
}

#[derive(Clone, Copy)]
enum BlockMode {
    None,
    Code,
    Mermaid,
}

/// Renders Markdown into sanitized HTML, extracting ToC headings, Mermaid presence, and links.
pub fn render_markdown(markdown: &str) -> RenderedMarkdown {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let mut out = RenderedMarkdown::default();
    let mut used_ids: HashMap<String, usize> = HashMap::new();
    let mut heading: Option<HeadingCapture> = None;
    let mut image: Option<ImageCapture> = None;
    let mut mode = BlockMode::None;
    let mut code_buf = String::new();
    let mut code_lang = String::new();
    let mut table_head = false;
    let mut in_tbody = false;
    let mut alignments: Vec<Alignment> = Vec::new();
    let mut col_idx = 0usize;

    for event in Parser::new_ext(markdown, opts) {
        if let Some(img) = image.as_mut() {
            if img.push_event(&event, &mut out.word_count)
                && let Some(finished) = image.take()
            {
                let target = current_buf(&mut out.html, &mut heading);
                finished.emit_into(target);
                if let Some(cap) = &mut heading {
                    cap.text.push_str(&finished.alt);
                }
            }
            continue;
        }

        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, id, .. } => {
                    heading = Some(HeadingCapture {
                        level,
                        explicit_id: id.map(|s| s.to_string()),
                        text: String::new(),
                        html_buf: String::new(),
                    });
                }
                Tag::Paragraph => current_buf(&mut out.html, &mut heading).push_str("<p>"),
                Tag::BlockQuote(_) => {
                    current_buf(&mut out.html, &mut heading).push_str("<blockquote>");
                }
                Tag::CodeBlock(kind) => {
                    code_buf.clear();
                    code_lang.clear();
                    mode = match kind {
                        CodeBlockKind::Fenced(info) => {
                            let lang = info.split_whitespace().next().unwrap_or("");
                            if lang.eq_ignore_ascii_case("mermaid") {
                                out.has_mermaid = true;
                                BlockMode::Mermaid
                            } else {
                                code_lang.push_str(lang);
                                BlockMode::Code
                            }
                        }
                        CodeBlockKind::Indented => BlockMode::Code,
                    };
                }
                Tag::List(Some(1)) => out.html.push_str("<ol>"),
                Tag::List(Some(start)) => {
                    let _ = write!(out.html, "<ol start=\"{start}\">");
                }
                Tag::List(None) => out.html.push_str("<ul>"),
                Tag::Item => out.html.push_str("<li>"),
                Tag::Emphasis => current_buf(&mut out.html, &mut heading).push_str("<em>"),
                Tag::Strong => current_buf(&mut out.html, &mut heading).push_str("<strong>"),
                Tag::Strikethrough => current_buf(&mut out.html, &mut heading).push_str("<del>"),
                Tag::Link {
                    dest_url, title, ..
                } => {
                    let href = sanitize_link_url(&dest_url, &mut out);
                    let target = current_buf(&mut out.html, &mut heading);
                    target.push_str("<a href=\"");
                    escape_html_into(target, &href);
                    target.push('"');
                    if !title.is_empty() {
                        target.push_str(" title=\"");
                        escape_html_into(target, &title);
                        target.push('"');
                    }
                    if href.starts_with("https://") || href.starts_with("http://") {
                        target.push_str(" rel=\"noopener noreferrer\"");
                    }
                    target.push('>');
                }
                Tag::Image {
                    dest_url, title, ..
                } => {
                    image = Some(ImageCapture::new(&dest_url, &title));
                }
                Tag::Table(align) => {
                    alignments = align;
                    in_tbody = false;
                    out.html.push_str("<table>");
                }
                Tag::TableHead => {
                    table_head = true;
                    col_idx = 0;
                    out.html.push_str("<thead><tr>");
                }
                Tag::TableRow => {
                    col_idx = 0;
                    out.html.push_str("<tr>");
                }
                Tag::TableCell => {
                    let tag_name = if table_head { "th" } else { "td" };
                    let align = alignments.get(col_idx).copied().unwrap_or(Alignment::None);
                    col_idx += 1;
                    let attr = match align {
                        Alignment::Left => " style=\"text-align:left\"",
                        Alignment::Center => " style=\"text-align:center\"",
                        Alignment::Right => " style=\"text-align:right\"",
                        Alignment::None => "",
                    };
                    let _ = write!(out.html, "<{tag_name}{attr}>");
                }
                Tag::FootnoteDefinition(name) => {
                    out.html.push_str("<aside class=\"footnote\" id=\"fn-");
                    escape_html_into(&mut out.html, &slugify(&name));
                    out.html.push_str("\">");
                }
                Tag::DefinitionList => out.html.push_str("<dl>"),
                Tag::DefinitionListTitle => out.html.push_str("<dt>"),
                Tag::DefinitionListDefinition => out.html.push_str("<dd>"),
                Tag::HtmlBlock | Tag::MetadataBlock(_) => {}
            },
            Event::End(end) => match end {
                TagEnd::Heading(_) => {
                    if let Some(cap) = heading.take() {
                        let lvl = heading_num(cap.level);
                        let raw_id = cap.explicit_id.unwrap_or_else(|| slugify(&cap.text));
                        let id = dedup_id(&raw_id, &mut used_ids);
                        let _ = write!(out.html, "<h{lvl} id=\"");
                        escape_html_into(&mut out.html, &id);
                        out.html.push_str("\">");
                        out.html.push_str(&cap.html_buf);
                        let _ = writeln!(out.html, "</h{lvl}>");
                        out.toc.push(Heading {
                            level: lvl,
                            id,
                            text: cap.text.trim().to_string(),
                        });
                    }
                }
                TagEnd::Paragraph => current_buf(&mut out.html, &mut heading).push_str("</p>\n"),
                TagEnd::BlockQuote(_) => {
                    current_buf(&mut out.html, &mut heading).push_str("</blockquote>\n");
                }
                TagEnd::CodeBlock => {
                    match mode {
                        BlockMode::Mermaid => out.html.push_str(&render_mermaid_block(&code_buf)),
                        BlockMode::Code => {
                            if code_lang.is_empty() {
                                out.html.push_str("<pre><code>");
                            } else {
                                out.html.push_str("<pre><code class=\"language-");
                                escape_html_into(&mut out.html, &slugify(&code_lang));
                                out.html.push_str("\">");
                            }
                            out.html.push_str(&highlight_code(&code_lang, &code_buf));
                            out.html.push_str("</code></pre>\n");
                        }
                        BlockMode::None => {}
                    }
                    mode = BlockMode::None;
                }
                TagEnd::List(true) => out.html.push_str("</ol>\n"),
                TagEnd::List(false) => out.html.push_str("</ul>\n"),
                TagEnd::Item => out.html.push_str("</li>\n"),
                TagEnd::Emphasis => current_buf(&mut out.html, &mut heading).push_str("</em>"),
                TagEnd::Strong => current_buf(&mut out.html, &mut heading).push_str("</strong>"),
                TagEnd::Strikethrough => {
                    current_buf(&mut out.html, &mut heading).push_str("</del>");
                }
                TagEnd::Link => current_buf(&mut out.html, &mut heading).push_str("</a>"),
                TagEnd::Image => {}
                TagEnd::Table => {
                    if in_tbody {
                        out.html.push_str("</tbody></table>\n");
                        in_tbody = false;
                    } else {
                        out.html.push_str("</table>\n");
                    }
                }
                TagEnd::TableHead => {
                    table_head = false;
                    in_tbody = true;
                    out.html.push_str("</tr></thead><tbody>");
                }
                TagEnd::TableRow => out.html.push_str("</tr>"),
                TagEnd::TableCell => {
                    out.html
                        .push_str(if table_head { "</th>" } else { "</td>" });
                }
                TagEnd::FootnoteDefinition => out.html.push_str("</aside>\n"),
                TagEnd::DefinitionList => out.html.push_str("</dl>\n"),
                TagEnd::DefinitionListTitle => out.html.push_str("</dt>\n"),
                TagEnd::DefinitionListDefinition => out.html.push_str("</dd>\n"),
                TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => {}
            },
            Event::Text(text) => {
                out.word_count += text.split_whitespace().count();
                match mode {
                    BlockMode::None => {
                        if let Some(cap) = &mut heading {
                            cap.text.push_str(&text);
                            escape_html_into(&mut cap.html_buf, &text);
                        } else {
                            escape_html_into(&mut out.html, &text);
                        }
                    }
                    BlockMode::Code | BlockMode::Mermaid => code_buf.push_str(&text),
                }
            }
            Event::Code(inline) => {
                out.word_count += 1;
                if let Some(cap) = &mut heading {
                    cap.text.push_str(&inline);
                    cap.html_buf.push_str("<code>");
                    escape_html_into(&mut cap.html_buf, &inline);
                    cap.html_buf.push_str("</code>");
                } else {
                    out.html.push_str("<code>");
                    escape_html_into(&mut out.html, &inline);
                    out.html.push_str("</code>");
                }
            }
            Event::SoftBreak => current_buf(&mut out.html, &mut heading).push('\n'),
            Event::HardBreak => current_buf(&mut out.html, &mut heading).push_str("<br />\n"),
            Event::Rule => out.html.push_str("<hr />\n"),
            Event::TaskListMarker(checked) => {
                let state = if checked { " checked" } else { "" };
                let _ = write!(
                    out.html,
                    "<input type=\"checkbox\" disabled{state} aria-label=\"task item\" /> "
                );
            }
            Event::FootnoteReference(name) => {
                let id = slugify(&name);
                let _ = write!(out.html, "<sup><a href=\"#fn-{id}\">[");
                escape_html_into(&mut out.html, &name);
                out.html.push_str("]</a></sup>");
            }
            // Deliberately strip raw HTML blocks & inline HTML to uphold strict XSS safety.
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::InlineMath(tex) | Event::DisplayMath(tex) => {
                let target = current_buf(&mut out.html, &mut heading);
                target.push_str("<code class=\"math\">");
                escape_html_into(target, &tex);
                target.push_str("</code>");
            }
        }
    }
    if let Some(finished) = image.take() {
        let target = current_buf(&mut out.html, &mut heading);
        finished.emit_into(target);
    }
    out
}

fn current_buf<'a>(
    main: &'a mut String,
    heading: &'a mut Option<HeadingCapture>,
) -> &'a mut String {
    match heading {
        Some(cap) => &mut cap.html_buf,
        None => main,
    }
}

fn heading_num(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
