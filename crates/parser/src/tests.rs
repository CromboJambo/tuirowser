use super::*;

#[test]
fn test_basic_html_to_markdown() {
    let html = "<h1>Hello World</h1><p>This is a paragraph.</p>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("# Hello World"));
    assert!(md.contains("This is a paragraph."));
}

#[test]
fn test_link_handling() {
    let html = "<a href=\"https://example.com\">Click here</a>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("[Click here]"));
}

#[test]
fn test_bold_and_italic() {
    let html = "<b>Bold text</b> and <i>italic text</i>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("**Bold text**"));
    assert!(md.contains("*italic text*"));
}

#[test]
fn test_code_blocks() {
    let html = "<code>let x = 42;</code>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("`let x = 42;`"));
}

#[test]
fn test_preformatted_text() {
    let html = "<pre>line1\nline2</pre>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("```"));
    assert!(md.contains("line1"));
    assert!(md.contains("line2"));
}

#[test]
fn test_unordered_list() {
    let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("- Item 1"));
    assert!(md.contains("- Item 2"));
}

#[test]
fn test_ordered_list() {
    let html = "<ol><li>First</li><li>Second</li></ol>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("1. First"));
    assert!(md.contains("2. Second"));
}

#[test]
fn test_image_handling() {
    let html = "<img src=\"https://example.com/image.png\" alt=\"Test image\">";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("![Test image](https://example.com/image.png)"));
}

#[test]
fn test_blockquote() {
    let html = "<blockquote>Quoted text</blockquote>";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("> Quoted text"));
}

#[test]
fn test_horizontal_rule() {
    let html = "Before<hr>After";
    let md = html_to_markdown(html).unwrap();
    assert!(md.contains("---"));
}

#[test]
fn test_strip_script_tags() {
    let html = "<script>alert('hello');</script><p>Content</p>";
    let md = html_to_markdown(html).unwrap();
    assert!(!md.contains("alert"));
    assert!(md.contains("Content"));
}

#[test]
fn test_strip_style_tags() {
    let html = "<style>.class { color: red; }</style><p>Content</p>";
    let md = html_to_markdown(html).unwrap();
    assert!(!md.contains("color"));
    assert!(md.contains("Content"));
}

#[test]
fn test_whitespace_cleanup() {
    let html = "<p>Line 1</p>\n\n<p>Line 2</p>";
    let md = html_to_markdown(html).unwrap();
    // Should not have excessive blank lines
    assert!(!md.contains("\n\n\n"));
}

#[test]
fn test_empty_html() {
    let html = "";
    let md = html_to_markdown(html).unwrap();
    assert_eq!(md, "");
}

#[test]
fn test_body_extraction() {
    let html = "<html><head><title>Test</title></head><body><p>Content</p></body></html>";
    let body = extract_body(html).unwrap();
    assert!(body.contains("<p>Content</p>"));
    assert!(!body.contains("<title>"));
}
