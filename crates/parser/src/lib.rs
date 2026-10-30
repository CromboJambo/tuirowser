//! HTML parsing and markdown conversion pipeline.
//!
//! This crate takes raw HTML and converts it to clean, readable markdown,
//! stripping away navigation bars, footers, ads, and other non-content elements.
//!
//! # Example
//! ```no_run
//! use tuirowser_parser::html_to_markdown;
//!
//! let html = "<h1>Hello</h1><p>World</p>";
//! let md = html_to_markdown(html).unwrap();
//! assert_eq!(md, "# Hello\n\nWorld");
//! ```

use anyhow::Result;

mod ascii_image;
mod css;

pub use ascii_image::image_to_ascii;
pub use css::parse_inline_style;

/// Parse HTML and convert to simplified markdown.
///
/// Handles headings, paragraphs, links, bold/italic, code blocks, lists,
/// blockquotes, horizontal rules, and other common HTML elements. Strips
/// script and style tags along with their content.
pub fn html_to_markdown(html: &str) -> Result<String> {
    // First, strip script and style tags with their content
    let cleaned = strip_tags_with_content(html, "script");
    let cleaned = strip_tags_with_content(&cleaned, "style");

    // Extract the body content
    let body = extract_body(&cleaned)?;

    // Convert HTML tags to markdown equivalents
    let mut markdown = String::new();
    let mut chars = body.chars().peekable();
    let mut in_ordered_list = false;
    let mut list_item_count = 0;

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
            let converted = convert_tag(&tag_content, &mut in_ordered_list, &mut list_item_count)?;
            markdown.push_str(&converted);
        } else {
            markdown.push(c);
        }
    }

    // Clean up excessive whitespace
    let cleaned = cleanup_whitespace(&markdown);

    Ok(cleaned)
}

/// Strip a tag and its content from HTML.
fn strip_tags_with_content(html: &str, tag_name: &str) -> String {
    let mut result = html.to_string();
    loop {
        let open_tag = format!("<{}", tag_name);
        if let Some(start) = result.find(&open_tag) {
            // Find the closing tag
            let close_tag = format!("</{}>", tag_name);
            if let Some(end_start) = result[start..].find(&close_tag) {
                let end = start + end_start + close_tag.len();
                result = format!("{}{}", &result[..start], &result[end..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    result
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
fn convert_tag(
    tag: &str,
    in_ordered_list: &mut bool,
    list_item_count: &mut i32,
) -> Result<String> {
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
        if let Some(_href) = extract_href(tag) {
            return Ok("[".to_string());
        }
    }
    if tag_lower == "</a>" {
        // Closing link — handled by capturing text after opening <a>
        return Ok("]".to_string());
    }

    // Span with inline styling - parse CSS and apply formatting
    if tag_lower.starts_with("<span") {
        if let Some(style) = extract_attribute(tag, "style") {
            let hints = parse_inline_style(&style);
            let mut prefix = String::new();
            let mut suffix = String::new();

            if hints.bold {
                prefix.push_str("**");
                suffix = format!("**{}", suffix);
            }
            if hints.italic {
                prefix.push('*');
                suffix = format!("*{}", suffix);
            }

            return Ok(format!("{}{}", prefix, suffix));
        }
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
        *list_item_count += 1;
        if *in_ordered_list {
            return Ok(format!("{}. ", *list_item_count));
        } else {
            return Ok("- ".to_string());
        }
    }

    // Ordered list start/end
    if tag_lower.starts_with("<ol") {
        *in_ordered_list = true;
        *list_item_count = 0;
        return Ok("\n\n".to_string());
    }
    if tag_lower == "</ol>" {
        *in_ordered_list = false;
        return Ok("\n\n".to_string());
    }

    // Unordered list start/end
    if tag_lower.starts_with("<ul") {
        *in_ordered_list = false;
        *list_item_count = 0;
        return Ok("\n\n".to_string());
    }
    if tag_lower == "</ul>" {
        return Ok("\n\n".to_string());
    }

    // Blockquote
    if tag_lower.starts_with("<blockquote") {
        return Ok("\n\n> ".to_string());
    }
    if tag_lower == "</blockquote>" {
        return Ok("\n\n".to_string());
    }

    // Horizontal rule
    if tag_lower == "<hr>" || tag_lower == "<hr/>" || tag_lower.starts_with("<hr ") {
        return Ok("\n\n---\n\n".to_string());
    }

    // Images - emit markdown image syntax with URL for post-processing
    if tag_lower.starts_with("<img") {
        if let Some(src) = extract_attribute(tag, "src") {
            if let Some(alt) = extract_attribute(tag, "alt") {
                return Ok(format!("\n\n![{}]({})\n\n", alt, src));
            } else {
                return Ok(format!("\n\n![image]({})\n\n", src));
            }
        }
    }

    // For all other tags, just skip the tag but keep content
    Ok("".to_string())
}

/// Extract href attribute from an anchor tag.
fn extract_href(tag: &str) -> Option<String> {
    extract_attribute(tag, "href")
}

/// Extract a named attribute from a tag.
fn extract_attribute(tag: &str, attr_name: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let search_term = format!("{}=", attr_name);
    if let Some(pos) = lower.find(&search_term) {
        let rest = &tag[pos + search_term.len()..];
        // Skip whitespace
        let rest = rest.trim_start();
        let quote_pos = rest.find(|c: char| c == '"' || c == '\'');
        if let Some(q) = quote_pos {
            let quote_char = rest.as_bytes()[q] as char;
            let end_quote = rest[q + 1..].find(quote_char);
            if let Some(e) = end_quote {
                return Some(rest[q + 1..q + 1 + e].to_string());
            }
        } else {
            // Unquoted attribute value
            let end = rest.find(|c: char| c == ' ' || c == '>' || c == '/');
            if let Some(e) = end {
                return Some(rest[..e].to_string());
            } else {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Clean up excessive whitespace in markdown output.
fn cleanup_whitespace(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
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

#[cfg(test)]
mod tests;
