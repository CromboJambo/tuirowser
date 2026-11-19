//! Navigation helpers for moving between document sections.

use crate::doc::Document;

/// An index of headings for fast navigation.
#[derive(Debug, Clone)]
pub struct HeadingIndex {
    /// Entries: (block_id, level, text)
    entries: Vec<(usize, u8, String)>,
}

impl HeadingIndex {
    /// Build a heading index from a document.
    pub fn build(document: &Document) -> Self {
        let entries = document.headings();
        Self { entries }
    }

    /// Get all headings.
    pub fn all(&self) -> &[(usize, u8, String)] {
        &self.entries
    }

    /// Find the next heading after the given block ID.
    pub fn next_after(&self, block_id: usize) -> Option<&(usize, u8, String)> {
        self.entries.iter().find(|(id, _, _)| *id > block_id)
    }

    /// Find the previous heading before the given block ID.
    pub fn prev_before(&self, block_id: usize) -> Option<&(usize, u8, String)> {
        self.entries.iter().rev().find(|(id, _, _)| *id < block_id)
    }

    /// Find the next heading of a higher or equal level (for outline navigation).
    pub fn next_same_or_higher_level(&self, block_id: usize, level: u8) -> Option<&(usize, u8, String)> {
        self.entries
            .iter()
            .find(|(id, lvl, _)| *id > block_id && *lvl <= level)
    }

    /// Find the previous heading of a higher or equal level.
    pub fn prev_same_or_higher_level(&self, block_id: usize, level: u8) -> Option<&(usize, u8, String)> {
        self.entries
            .iter()
            .rev()
            .find(|(id, lvl, _)| *id < block_id && *lvl <= level)
    }

    /// Number of headings in the document.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the document has no headings.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Document;

    #[test]
    fn heading_index_navigation() {
        let md = "# A\n\ntext\n\n## B\n\ntext\n\n### C\n\ntext\n\n# D";
        let doc = Document::parse(md).unwrap();
        let index = HeadingIndex::build(&doc);

        assert_eq!(index.len(), 4);

        // Navigate from A to B
        let a_id = index.all()[0].0;
        let next = index.next_after(a_id).unwrap();
        assert_eq!(next.2, "B");

        // Navigate back from D to C
        let d_id = index.all()[3].0;
        let prev = index.prev_before(d_id).unwrap();
        assert_eq!(prev.2, "C");
    }

    #[test]
    fn level_aware_navigation() {
        let md = "# A\n\n## B\n\n### C\n\n## D\n\n# E";
        let doc = Document::parse(md).unwrap();
        let index = HeadingIndex::build(&doc);

        // From B (level 2), next same-or-higher should be D (level 2), not C (level 3)
        let b_id = index.all()[1].0;
        let next = index.next_same_or_higher_level(b_id, 2).unwrap();
        assert_eq!(next.2, "D");
    }
}
