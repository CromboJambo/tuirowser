//! Basic inline CSS style parsing for terminal formatting.

/// Parse an inline style attribute and return markdown formatting hints.
pub fn parse_inline_style(style: &str) -> StyleHints {
    let mut hints = StyleHints::default();

    // Parse font-weight
    if style.contains("font-weight:bold") || style.contains("font-weight: 700") || style.contains("font-weight:800") || style.contains("font-weight:900") {
        hints.bold = true;
    }

    // Parse font-style
    if style.contains("font-style:italic") {
        hints.italic = true;
    }

    // Parse text-decoration
    if style.contains("text-decoration:underline") {
        hints.underline = true;
    }

    // Parse color (basic named colors)
    let color = extract_color(style);
    if let Some(c) = color {
        hints.color = Some(c);
    }

    hints
}

/// Extract a color value from a style string.
fn extract_color(style: &str) -> Option<String> {
    // Look for color property
    if let Some(pos) = style.find("color:") {
        let rest = &style[pos + 6..];
        let trimmed = rest.trim_start();
        
        // Named colors
        if trimmed.starts_with("red") { return Some("red".to_string()); }
        if trimmed.starts_with("blue") { return Some("blue".to_string()); }
        if trimmed.starts_with("green") { return Some("green".to_string()); }
        if trimmed.starts_with("yellow") { return Some("yellow".to_string()); }
        if trimmed.starts_with("orange") { return Some("orange".to_string()); }
        if trimmed.starts_with("purple") { return Some("purple".to_string()); }
        if trimmed.starts_with("cyan") { return Some("cyan".to_string()); }
        if trimmed.starts_with("magenta") { return Some("magenta".to_string()); }
        if trimmed.starts_with("white") { return Some("white".to_string()); }
        if trimmed.starts_with("black") { return Some("black".to_string()); }
        
        // Hex colors - convert to closest terminal color
        if let Some(hex_pos) = trimmed.find('#') {
            let hex = &trimmed[hex_pos + 1..];
            if hex.len() >= 6 {
                return parse_hex_color(&hex[..6]);
            }
        }
    }

    None
}

/// Parse a hex color and map to closest terminal color name.
fn parse_hex_color(hex: &str) -> Option<String> {
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    // Simple distance-based color matching to terminal colors
    let colors = [
        ("black", 0, 0, 0),
        ("red", 255, 0, 0),
        ("green", 0, 255, 0),
        ("yellow", 255, 255, 0),
        ("blue", 0, 0, 255),
        ("magenta", 255, 0, 255),
        ("cyan", 0, 255, 255),
        ("white", 255, 255, 255),
    ];

    let mut best_name = "white";
    let mut best_dist = u32::MAX;

    for (name, cr, cg, cb) in &colors {
        let dist = color_distance(r, g, b, *cr, *cg, *cb);
        if dist < best_dist {
            best_dist = dist;
            best_name = name;
        }
    }

    Some(best_name.to_string())
}

/// Calculate squared color distance.
fn color_distance(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> u32 {
    let dr = (r1 as i32 - r2 as i32) as u32;
    let dg = (g1 as i32 - g2 as i32) as u32;
    let db = (b1 as i32 - b2 as i32) as u32;
    dr * dr + dg * dg + db * db
}

/// Style hints extracted from inline CSS.
#[derive(Debug, Default)]
pub struct StyleHints {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bold() {
        let hints = parse_inline_style("font-weight:bold");
        assert!(hints.bold);
    }

    #[test]
    fn test_parse_italic() {
        let hints = parse_inline_style("font-style:italic");
        assert!(hints.italic);
    }

    #[test]
    fn test_parse_color() {
        let hints = parse_inline_style("color:red");
        assert_eq!(hints.color, Some("red".to_string()));
    }

    #[test]
    fn test_parse_hex_color() {
        let hints = parse_inline_style("color:#FF0000");
        assert_eq!(hints.color, Some("red".to_string()));
    }
}