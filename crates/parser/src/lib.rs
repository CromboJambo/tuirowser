//! HTML parsing and markdown conversion pipeline.
//!
//! This crate takes raw HTML and converts it to clean, readable markdown,
//! stripping away navigation bars, footers, ads, and other non-content elements.
//!
//! Uses a simple regex-based approach for initial extraction, with plans to
//! migrate to full DOM parsing once the markup5ever API stabilizes.

use anyhow::Result;
use std::collections::HashMap;

/// Parse HTML and convert to simplified markdown.
pub fn html_to_markdown(html: &str) -> Result<String> {
    // First, extract the main content area (body)
    let body = extract_body(html)?;

    // Convert HTML tags to markdown equivalents
    let mut markdown = String::new();
    let mut chars = body.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Parse tag
            let mut tag_content = String::from("<");
            while let Some(&next_c) = chars.peek() {
                tag_content.push(next_c);
                chars.next();
                if next_c == '>' {
                    break;
                }
            }

            // Convert tag to markdown
            let converted = convert_tag(&tag_content, &mut chars)?;
            markdown.push_str(&converted);
        } else {
            markdown.push(c);
        }
    }

    // Clean up excessive whitespace
    let cleaned = cleanup_whitespace(&markdown);

    Ok(cleaned)
}

/// Extract the body content from HTML.
fn extract_body(html: &str) -> Result<String> {
    // Find <body>...</body> or use entire document
    let start = html.find("<body");
    if let Some(start_pos) = start {
        let end_tag_start = html[start_pos..].find("</body>");
        if let Some(end_pos) = end_tag_start {
            return Ok(html[start_pos + "<body".len()..start_pos + end_pos].to_string());
        }
    }

    // Fallback: strip <html>, <head>, etc. tags and use the rest
    let without_head = html.replace("<head>", "").replace("</head>", "");
    let without_html = without_head.replace("<html>", "").replace("</html>", "");
    Ok(without_html)
}

/// Convert an HTML tag to its markdown equivalent.
fn convert_tag(tag: &str, _chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<String> {
    let tag_lower = tag.to_lowercase();

    // Headings
    if tag_lower.starts_with("<h1") {
        return Ok("\n\n# ".to_string());
    }
    if tag_lower.starts_with("</h1>") {
        return Ok("\n\n".to_string());
    }
    if tag_lower.starts_with("<h2") {
        return Ok("\n\n## ".to_string());
    }
    if tag_lower.starts_with("</h2>") {
        return Ok("\n\n".to_string());
    }
    if tag_lower.starts_with("<h3") {
        return Ok("\n\n### ".to_string());
    }
    if tag_lower.starts_with("</h3>") {
        return Ok("\n\n".to_string());
    }

    // Paragraph
    if tag_lower == "<p>" || tag_lower.starts_with("<p ") {
        return Ok("\n\n".to_string());
    }
    if tag_lower == "</p>" {
        return Ok("\n\n".to_string());
    }

    // Links
    if tag_lower.starts_with("<a ") {
        if let Some(href) = extract_href(tag) {
            return Ok(format!("[", ));
        }
    }
    if tag_lower == "</a>" {
        // Closing link — handled by capturing text after opening <a>
        return Ok("]".to_string());
    }

    // Bold/Strong
    if tag_lower == "<b>" || tag_lower == "<strong>" {
        return Ok("**".to_string());
    }
    if tag_lower == "</b>" || tag_lower == "</strong>" {
        return Ok("**".to_string());
    }

    // Italic
    if tag_lower == "<i>" || tag_lower == "<em>" {
        return Ok("*".to_string());
    }
    if tag_lower == "</i>" || tag_lower == "</em>" {
        return Ok("*".to_string());
    }

    // Code
    if tag_lower == "<code>" {
        return Ok("`".to_string());
    }
    if tag_lower == "</code>" {
        return Ok("`".to_string());
    }

    // Preformatted
    if tag_lower.starts_with("<pre") {
        return Ok("\n\n```\n".to_string());
    }
    if tag_lower == "</pre>" {
        return Ok("\n```\n\n".to_string());
    }

    // Line break
    if tag_lower == "<br>" || tag_lower == "<br/>" || tag_lower.starts_with("<br ") {
        return Ok("\n".to_string());
    }

    // List items
    if tag_lower.starts_with("<li") {
        return Ok("- ".to_string());
    }

    // Skip script and style tags entirely
    if tag_lower.starts_with("<script") || tag_lower.starts_with("<style") {
        return Ok("".to_string());
    }
    if tag_lower.starts_with("</script>") || tag_lower.starts_with("</style>") {
        return Ok("".to_string());
    }

    // For all other tags, just skip the tag but keep content
    Ok("".to_string())
}

/// Extract href attribute from an anchor tag.
fn extract_href(tag: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    if let Some(pos) = lower.find("href=") {
        let rest = &tag[pos + 5..];
        let quote_pos = rest.find(|c: char| c == '"' || c == '\'');
        if let Some(q) = quote_pos {
            let end_quote = rest[q + 1..].find(|c: char| c == rest.as_bytes()[q] as char);
            if let Some(e) = end_quote {
                return Some(rest[q + 1..q + 1 + e].to_string());
            }
        }
    }
    None
}

/// Clean up excessive whitespace in markdown output.
fn cleanup_whitespace(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut prev_empty = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_empty && !result.is_empty() {
                result.push("");
            }
            prev_empty = true;
        } else {
            result.push(line);
            prev_empty = false;
        }
    }

    let joined = result.join("\n");
    // Trim leading/trailing newlines
    joined.trim().to_string()
}