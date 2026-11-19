//! Layout computation for document blocks within a scrollable area.
//!
//! Given a `Document` and an available screen rectangle, `DocLayout` computes
//! where each block should be rendered, handling:
//! - Line wrapping at the available width
//! - Scroll offset (in terms of rendered lines)
//! - Block visibility (which blocks are partially/fully in view)

use crate::doc::{Block, Document};
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

/// A single line of rendered output after wrapping.
#[derive(Debug, Clone)]
pub struct RenderedLine {
    /// The block this line belongs to.
    pub block_id: usize,
    /// Whether this is the first line of its block (for spacing decisions).
    pub is_first_of_block: bool,
    /// The wrapped text content.
    pub content: String,
}

/// Layout state for rendering a document within a viewport.
#[derive(Debug, Clone)]
pub struct DocLayout {
    /// All rendered lines after wrapping (the full scrollable content).
    pub lines: Vec<RenderedLine>,
    /// Current scroll offset in lines.
    pub scroll_offset: usize,
    /// The available rendering area.
    pub area: Rect,
    /// Map from block ID to the range of rendered line indices it occupies.
    pub block_line_ranges: Vec<(usize, usize, usize)>, // (block_id, start_line, end_line_exclusive)
}

/// Compute the display width of a string.
fn str_width(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(0)).sum()
}

impl DocLayout {
    /// Compute layout for a document within the given area.
    pub fn compute(document: &Document, area: Rect, wrap_width: usize) -> Self {
        let mut lines: Vec<RenderedLine> = Vec::new();
        let mut block_line_ranges: Vec<(usize, usize, usize)> = Vec::new();

        for block in &document.blocks {
            let start_line = lines.len();

            // Render each line of the block, wrapping if necessary
            for (line_idx, line) in block.text.lines.iter().enumerate() {
                let is_first = line_idx == 0;
                let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();

                if wrap_width > 0 && str_width(&text) > wrap_width {
                    // Wrap long lines
                    for wrapped in wrap_text(&text, wrap_width) {
                        lines.push(RenderedLine {
                            block_id: block.id,
                            is_first_of_block: is_first && lines.last().map(|l| l.block_id) != Some(block.id),
                            content: wrapped,
                        });
                    }
                } else if text.is_empty() {
                    // Preserve empty lines for spacing
                    lines.push(RenderedLine {
                        block_id: block.id,
                        is_first_of_block: is_first && lines.last().map(|l| l.block_id) != Some(block.id),
                        content: String::new(),
                    });
                } else {
                    lines.push(RenderedLine {
                        block_id: block.id,
                        is_first_of_block: is_first && lines.last().map(|l| l.block_id) != Some(block.id),
                        content: text,
                    });
                }
            }

            let end_line = lines.len();
            block_line_ranges.push((block.id, start_line, end_line));
        }

        Self {
            lines,
            scroll_offset: 0,
            area,
            block_line_ranges,
        }
    }

    /// Get the visible slice of rendered lines for the current scroll position.
    pub fn visible_lines(&self) -> Vec<&RenderedLine> {
        let height = self.area.height as usize;
        let start = self.scroll_offset.min(self.lines.len());
        let end = (start + height).min(self.lines.len());
        self.lines[start..end].iter().collect()
    }

    /// Scroll down by N lines.
    pub fn scroll_down(&mut self, n: usize) {
        let max_scroll = self.lines.len().saturating_sub(self.area.height as usize);
        self.scroll_offset = (self.scroll_offset + n).min(max_scroll);
    }

    /// Scroll up by N lines.
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Jump to the first line of a given block.
    pub fn jump_to_block(&mut self, block_id: usize) {
        if let Some((_, start, _)) = self.block_line_ranges.iter().find(|(id, _, _)| *id == block_id) {
            // Scroll so the block is at the top of the viewport
            self.scroll_offset = *start;
        }
    }

    /// Find which block contains a given rendered line index.
    pub fn block_at_line(&self, line_index: usize) -> Option<usize> {
        for (block_id, start, end) in &self.block_line_ranges {
            if line_index >= *start && line_index < *end {
                return Some(*block_id);
            }
        }
        None
    }

    /// Get the current scroll percentage (0.0 to 1.0).
    pub fn scroll_percentage(&self) -> f64 {
        if self.lines.len() <= self.area.height as usize {
            return 0.0;
        }
        let max_scroll = self.lines.len() - self.area.height as usize;
        if max_scroll == 0 {
            return 0.0;
        }
        self.scroll_offset as f64 / max_scroll as f64
    }

    /// Total number of rendered lines.
    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }
}

/// Wrap a string to the given width, breaking at word boundaries when possible.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 || str_width(text) <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        // Single long word or no spaces — hard break
        let chars: Vec<char> = text.chars().collect();
        for chunk in chars.chunks(width) {
            lines.push(chunk.iter().collect());
        }
        return lines;
    }

    let mut current_line = String::new();
    let mut current_width: usize = 0;

    for word in words {
        let word_width = str_width(word);

        if current_width == 0 {
            // First word on line
            current_line = word.to_string();
            current_width = word_width;
        } else if current_width + 1 + word_width <= width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_width;
        } else {
            // Word doesn't fit — start new line
            lines.push(current_line);
            current_line = word.to_string();
            current_width = word_width;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}


