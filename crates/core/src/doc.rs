//! Block-based document model.
//!
//! A `Document` is a sequence of `Block`s, each representing a semantic unit
//! of the markdown source (heading, paragraph, code block, list, table, etc.).
//! Blocks are parsed from markdown using comrak's AST and carry pre-rendered
//! styled text for efficient display.

use anyhow::Result;
use comrak::nodes::{AstNode, ListType, NodeValue};
use comrak::{Arena, Options};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

/// Styling variant for code blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeStyle {
    /// Plain fenced or indented code block.
    Fenced,
    /// Inline code span within a paragraph.
    Inline,
}

/// Semantic kind of a document block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// A heading with its level (1-6).
    Heading(u8),
    /// A regular paragraph of text.
    Paragraph,
    /// A code block with optional language tag.
    CodeBlock(CodeStyle),
    /// A list item. `marker` is the bullet/number character.
    List { marker: char },
    /// A table row (header or data).
    TableRow,
    /// A horizontal rule.
    Rule,
    /// A blockquote line.
    Quote,
    /// An empty line / spacing.
    Blank,
}

/// A single semantic unit of a document.
#[derive(Debug, Clone)]
pub struct Block {
    /// Stable identifier for this block (its index in the source parse order).
    pub id: usize,
    /// The kind of content this block holds.
    pub kind: BlockKind,
    /// Pre-rendered styled text for display.
    pub text: Text<'static>,
    /// Source line numbers this block spans (1-indexed, inclusive start, exclusive end).
    pub source_lines: (usize, usize),
    /// For headings: the heading text without the `#` markers.
    pub heading_text: Option<String>,
    /// For links within paragraphs: extracted URL targets.
    pub links: Vec<(String, String)>, // (display text, url)
}

impl Block {
    /// Number of rendered lines this block occupies (before wrapping).
    pub fn line_count(&self) -> usize {
        self.text.lines.len()
    }

    /// Whether this block is a heading.
    pub fn is_heading(&self) -> bool {
        matches!(self.kind, BlockKind::Heading(_))
    }
}

/// A parsed markdown document as a sequence of blocks.
#[derive(Debug, Clone)]
pub struct Document {
    /// The original markdown source (for reference and "view source" mode).
    pub source: String,
    /// Parsed blocks in document order.
    pub blocks: Vec<Block>,
    /// Title extracted from the first H1, if any.
    pub title: Option<String>,
}

impl Document {
    /// Parse a markdown string into a block-based document.
    pub fn parse(source: &str) -> Result<Self> {
        let mut options = Options::default();
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.autolink = true;

        let arena = Arena::new();
        let root = comrak::parse_document(&arena, source, &options);

        let mut blocks: Vec<Block> = Vec::new();
        let mut title = None;
        let mut block_id = 0;

        // Walk the top-level children of the document root
        walk_top_level_blocks(root, &mut blocks, &mut title, &mut block_id);

        Ok(Self {
            source: source.to_string(),
            blocks,
            title,
        })
    }

    /// Get a block by ID.
    pub fn block(&self, id: usize) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }

    /// Total number of rendered lines (before wrapping).
    pub fn total_lines(&self) -> usize {
        self.blocks.iter().map(|b| b.line_count()).sum()
    }

    /// Find all headings with their block IDs and levels.
    pub fn headings(&self) -> Vec<(usize, u8, String)> {
        self.blocks
            .iter()
            .filter_map(|b| {
                if let BlockKind::Heading(level) = b.kind {
                    Some((b.id, level, b.heading_text.clone()?))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find the next heading at or below a given block ID.
    pub fn next_heading(&self, after_id: usize) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id > after_id && b.is_heading())
    }

    /// Find the previous heading before a given block ID.
    pub fn prev_heading(&self, before_id: usize) -> Option<&Block> {
        self.blocks.iter().rev().find(|b| b.id < before_id && b.is_heading())
    }
}

/// Walk top-level AST nodes and convert them to blocks, descending into containers.
fn walk_top_level_blocks(
    root: &AstNode<'_>,
    blocks: &mut Vec<Block>,
    title: &mut Option<String>,
    block_id: &mut usize,
) {
    let mut node = root.first_child();
    while let Some(current) = node {
        // Descend into container nodes (List, BlockQuote) to find renderable children
        match &current.data().value {
            NodeValue::List(_) | NodeValue::BlockQuote => {
                walk_children(current, blocks, title, block_id);
            }
            _ => {
                if let Some(block) = ast_node_to_block(current, block_id) {
                    if title.is_none() && block.is_heading() {
                        *title = block.heading_text.clone();
                    }
                    blocks.push(block);
                }
            }
        }
        node = current.next_sibling();
    }
}

/// Walk children of a container node and convert renderable ones to blocks.
fn walk_children(
    parent: &AstNode<'_>,
    blocks: &mut Vec<Block>,
    title: &mut Option<String>,
    block_id: &mut usize,
) {
    let mut node = parent.first_child();
    while let Some(current) = node {
        match &current.data().value {
            NodeValue::List(_) | NodeValue::BlockQuote => {
                walk_children(current, blocks, title, block_id);
            }
            _ => {
                if let Some(block) = ast_node_to_block(current, block_id) {
                    if title.is_none() && block.is_heading() {
                        *title = block.heading_text.clone();
                    }
                    blocks.push(block);
                }
            }
        }
        node = current.next_sibling();
    }
}

/// Convert a comrak AST node into a Block, if it represents renderable content.
fn ast_node_to_block(node: &AstNode<'_>, block_id: &mut usize) -> Option<Block> {
    let data = node.data();
    let sourcepos = data.sourcepos;
    let start_line = (sourcepos.start.line - 1).max(0); // convert to 0-indexed
    let end_line = (sourcepos.end.line - 1).max(start_line + 1);

    match &data.value {
        NodeValue::Heading(heading) => {
            let level = heading.level;
            let text = collect_text(node);
            let heading_text = text.trim().to_string();
            let style = heading_style(level);

            let mut spans = vec![Span::styled(
                format!("{} ", "#".repeat(level as usize)),
                Style::default().fg(Color::DarkGray),
            )];
            spans.push(Span::styled(heading_text.clone(), style));

            Some(Block {
                id: take_id(block_id),
                kind: BlockKind::Heading(level),
                text: Text::from(Line::from(spans)),
                source_lines: (start_line, end_line),
                heading_text: Some(heading_text),
                links: vec![],
            })
        }

        NodeValue::Paragraph => {
            let (text, links) = collect_paragraph(node);
            Some(Block {
                id: take_id(block_id),
                kind: BlockKind::Paragraph,
                text,
                source_lines: (start_line, end_line),
                heading_text: None,
                links,
            })
        }

        NodeValue::CodeBlock(code_block) => {
            let style = CodeStyle::Fenced;
            let lang = code_block.info.clone();
            let content = code_block.literal.clone();

            let mut lines: Vec<Line> = Vec::new();
            if !lang.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("```{}", lang),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "```",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
                )));
            }

            let code_style = Style::default()
                .fg(Color::LightCyan)
                .bg(Color::Rgb(30, 30, 30));
            for line in content.lines() {
                lines.push(Line::from(Span::styled(line.to_string(), code_style)));
            }

            Some(Block {
                id: take_id(block_id),
                kind: BlockKind::CodeBlock(style),
                text: Text::from(lines),
                source_lines: (start_line, end_line),
                heading_text: None,
                links: vec![],
            })
        }

        NodeValue::Item(item) => {
            let marker = match item.list_type {
                ListType::Bullet => '•',
                ListType::Ordered => '1', // Simplified; real numbering needs context
            };
            let text = collect_text(node);
            let style = Style::default();

            Some(Block {
                id: take_id(block_id),
                kind: BlockKind::List { marker },
                text: Text::from(Line::from(vec![
                    Span::styled(format!("{} ", marker), Style::default().fg(Color::Yellow)),
                    Span::styled(text, style),
                ])),
                source_lines: (start_line, end_line),
                heading_text: None,
                links: vec![],
            })
        }

        NodeValue::ThematicBreak => Some(Block {
            id: take_id(block_id),
            kind: BlockKind::Rule,
            text: Text::from(Line::from(Span::styled(
                "─".repeat(40),
                Style::default().fg(Color::DarkGray),
            ))),
            source_lines: (start_line, end_line),
            heading_text: None,
            links: vec![],
        }),

        NodeValue::BlockQuote => {
            let text = collect_text(node);
            Some(Block {
                id: take_id(block_id),
                kind: BlockKind::Quote,
                text: Text::from(Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        text,
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])),
                source_lines: (start_line, end_line),
                heading_text: None,
                links: vec![],
            })
        }

        NodeValue::Table(_) => {
            // Tables are complex; render as plain text for now
            let text = collect_text(node);
            Some(Block {
                id: take_id(block_id),
                kind: BlockKind::TableRow,
                text: Text::from(text.lines().map(|l| Line::from(l.to_string())).collect::<Vec<_>>()),
                source_lines: (start_line, end_line),
                heading_text: None,
                links: vec![],
            })
        }

        _ => None, // Skip non-renderable nodes (lists containers, etc.)
    }
}

/// Collect plain text content from an AST node and its descendants.
fn collect_text(node: &AstNode<'_>) -> String {
    let mut result = String::new();
    collect_text_recursive(node, &mut result);
    result.trim().to_string()
}

fn collect_text_recursive(node: &AstNode<'_>, result: &mut String) {
    match &node.data().value {
        NodeValue::Text(text) => {
            result.push_str(text);
        }
        NodeValue::SoftBreak => {
            result.push(' ');
        }
        NodeValue::LineBreak => {
            result.push('\n');
        }
        _ => {}
    }
    let mut child = node.first_child();
    while let Some(c) = child {
        collect_text_recursive(c, result);
        child = c.next_sibling();
    }
}

/// Collect styled paragraph text with inline formatting and link extraction.
fn collect_paragraph(node: &AstNode<'_>) -> (Text<'static>, Vec<(String, String)>) {
    let mut spans: Vec<Span> = Vec::new();
    let mut links: Vec<(String, String)> = Vec::new();

    collect_paragraph_recursive(node, &mut spans, &mut links);

    let text = Text::from(Line::from(spans));
    (text, links)
}

fn collect_paragraph_recursive(
    node: &AstNode<'_>,
    spans: &mut Vec<Span>,
    links: &mut Vec<(String, String)>,
) {
    match &node.data().value {
        NodeValue::Text(text) => {
            spans.push(Span::styled(text.clone(), Style::default()));
        }
        NodeValue::SoftBreak => {
            spans.push(Span::raw(" "));
        }
        NodeValue::LineBreak => {
            spans.push(Span::raw(" "));
        }
        _ => {}
    }

    let mut child = node.first_child();
    while let Some(c) = child {
        match &c.data().value {
            NodeValue::Emph => {
                collect_emphasized(c, spans, Modifier::ITALIC);
            }
            NodeValue::Strong => {
                collect_emphasized(c, spans, Modifier::BOLD);
            }
            NodeValue::Strikethrough => {
                collect_emphasized(c, spans, Modifier::CROSSED_OUT);
            }
            NodeValue::Link(link) => {
                let url = link.url.clone();
                let text = collect_text(c);
                links.push((text.clone(), url.clone()));
                spans.push(Span::styled(
                    format!("[{}]({})", text, url),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            }
            NodeValue::Code(code) => {
                spans.push(Span::styled(
                    code.literal.clone(),
                    Style::default()
                        .fg(Color::LightCyan)
                        .bg(Color::Rgb(40, 40, 40)),
                ));
            }
            NodeValue::Text(text) => {
                spans.push(Span::styled(text.clone(), Style::default()));
            }
            _ => {}
        }
        child = c.next_sibling();
    }
}

fn collect_emphasized(node: &AstNode<'_>, spans: &mut Vec<Span>, modifier: Modifier) {
    let text = collect_text(node);
    if !text.is_empty() {
        spans.push(Span::styled(text, Style::default().add_modifier(modifier)));
    }
}

/// Get the style for a heading at the given level.
fn heading_style(level: u8) -> Style {
    let color = match level {
        1 => Color::LightMagenta,
        2 => Color::LightCyan,
        3 => Color::LightGreen,
        _ => Color::LightYellow,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

/// Take the next block ID and increment the counter.
fn take_id(counter: &mut usize) -> usize {
    let id = *counter;
    *counter += 1;
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_document() {
        let md = "# Title\n\nSome paragraph text.\n\n## Section\n\n- Item one\n- Item two";
        let doc = Document::parse(md).unwrap();

        assert_eq!(doc.title.as_deref(), Some("Title"));
        assert!(doc.blocks.len() >= 4);
        assert!(doc.blocks[0].is_heading());
    }

    #[test]
    fn extract_headings() {
        let md = "# One\n\n## Two\n\n### Three";
        let doc = Document::parse(md).unwrap();
        let headings = doc.headings();

        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].1, 1); // level
        assert_eq!(headings[0].2, "One");
    }

    #[test]
    fn next_prev_heading_navigation() {
        let md = "# A\n\ntext\n\n## B\n\ntext\n\n### C";
        let doc = Document::parse(md).unwrap();

        // Find block IDs for headings
        let a_id = doc.blocks.iter().find(|b| b.heading_text.as_deref() == Some("A")).unwrap().id;
        let b_id = doc.blocks.iter().find(|b| b.heading_text.as_deref() == Some("B")).unwrap().id;

        // Next heading after A should be B
        let next = doc.next_heading(a_id).unwrap();
        assert_eq!(next.id, b_id);

        // Previous heading before C should be B
        let c_id = doc.blocks.iter().find(|b| b.heading_text.as_deref() == Some("C")).unwrap().id;
        let prev = doc.prev_heading(c_id).unwrap();
        assert_eq!(prev.id, b_id);
    }

    #[test]
    fn code_block_preserved() {
        let md = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let doc = Document::parse(md).unwrap();

        let code_block = doc.blocks.iter().find(|b| matches!(b.kind, BlockKind::CodeBlock(_)));
        assert!(code_block.is_some());
        let cb = code_block.unwrap();
        assert!(cb.text.lines.len() >= 4); // fence + 3 lines + maybe closing fence
    }

    #[test]
    fn links_extracted_from_paragraphs() {
        let md = "Visit [example](https://example.com) for more.";
        let doc = Document::parse(md).unwrap();

        let para = doc.blocks.iter().find(|b| b.kind == BlockKind::Paragraph).unwrap();
        assert_eq!(para.links.len(), 1);
        assert_eq!(para.links[0].1, "https://example.com");
    }
}
