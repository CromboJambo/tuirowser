use anyhow::Result;
use ratatui::text::{Line, Span};
use ratatui::style::{Color, Modifier, Style};

/// Parse markdown text and convert to styled ratatui lines.
pub fn markdown_to_lines(md: &str) -> Result<Vec<Line<'static>>> {
    let mut lines = Vec::new();
    
    for line in md.lines() {
        // Convert each line to owned String to break lifetime connection
        let line_str = line.to_string();
        let trimmed = line_str.trim_start().to_string();
        
        if let Some(content) = trimmed.strip_prefix("### ") {
            lines.push(Line::from(vec![
                Span::styled("### ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Magenta)),
                parse_inline_owned(content.to_string()),
            ]));
        } else if let Some(content) = trimmed.strip_prefix("## ") {
            lines.push(Line::from(vec![
                Span::styled("## ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Blue)),
                parse_inline_owned(content.to_string()),
            ]));
        } else if let Some(content) = trimmed.strip_prefix("# ") {
            lines.push(Line::from(vec![
                Span::styled("# ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)),
                parse_inline_owned(content.to_string()),
            ]));
        }
        // List items
        else if let Some(content) = trimmed.strip_prefix("- ") {
            lines.push(Line::from(vec![
                Span::styled("- ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green)),
                parse_inline_owned(content.to_string()),
            ]));
        } else if let Some(content) = trimmed.strip_prefix("* ") {
            lines.push(Line::from(vec![
                Span::styled("* ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green)),
                parse_inline_owned(content.to_string()),
            ]));
        }
        // Numbered list items
        else if trimmed.chars().next().map(|c| c.is_digit(10)).unwrap_or(false) {
            if let Some(pos) = trimmed.find(". ") {
                let prefix = trimmed[..pos + 2].to_string();
                let content = trimmed[pos + 2..].to_string();
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().add_modifier(Modifier::BOLD).fg(Color::Green)),
                    parse_inline_owned(content),
                ]));
            } else {
                lines.push(Line::from(parse_inline_owned(line_str)));
            }
        }
        // Blockquotes
        else if let Some(content) = trimmed.strip_prefix("> ") {
            lines.push(Line::from(vec![
                Span::styled("> ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                parse_inline_owned(content.to_string()),
            ]));
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" {
            lines.push(Line::from(Span::styled("---", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))));
        }
        // Empty line
        else if trimmed.is_empty() {
            lines.push(Line::from(""));
        }
        // Regular text with inline formatting
        else {
            lines.push(Line::from(parse_inline_owned(line_str)));
        }
    }
    
    Ok(lines)
}

/// Parse inline markdown elements, taking owned String to guarantee 'static lifetime.
fn parse_inline_owned(text: String) -> Span<'static> {
    // Bold: **text**
    if let Some(start) = text.find("**") {
        if let Some(end) = text[start + 2..].find("**") {
            let bold_text = text[start + 2..start + 2 + end].to_string();
            return Span::styled(
                bold_text,
                Style::default().add_modifier(Modifier::BOLD),
            );
        }
    }
    
    // Italic: *text*
    if let Some(start) = text.find('*') {
        if let Some(end) = text[start + 1..].find('*') {
            let italic_text = text[start + 1..start + 1 + end].to_string();
            return Span::styled(
                italic_text,
                Style::default().add_modifier(Modifier::ITALIC),
            );
        }
    }
    
    // Inline code: `text`
    if let Some(start) = text.find('`') {
        if let Some(end) = text[start + 1..].find('`') {
            let code_text = text[start + 1..start + 1 + end].to_string();
            return Span::styled(
                code_text,
                Style::default().fg(Color::Yellow).bg(Color::Black),
            );
        }
    }
    
    // Links: [text](url)
    if let Some(start) = text.find("[") {
        if let Some(end) = text[start..].find("]") {
            let link_text = text[start + 1..start + end].to_string();
            return Span::styled(
                link_text,
                Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
            );
        }
    }
    
    // Plain text - already owned
    Span::styled(text, Style::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_h1() {
        let result = markdown_to_lines("# Hello World").unwrap();
        assert_eq!(result.len(), 1);
        // First line should have the heading prefix
    }

    #[test]
    fn test_heading_h2() {
        let result = markdown_to_lines("## Subheading").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_list_item() {
        let result = markdown_to_lines("- Item one").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_bold_text() {
        let result = markdown_to_lines("**bold text**").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_multiple_lines() {
        let md = "# Title\n\nSome text.\n\n- Item";
        let result = markdown_to_lines(md).unwrap();
        assert_eq!(result.len(), 5); // title, empty, text, empty, item
    }
}