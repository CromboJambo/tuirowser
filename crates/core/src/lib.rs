//! Document model and markdown parsing for tuirowser.
//!
//! This crate provides a block-based document model that parses markdown into
//! semantic units (headings, paragraphs, code blocks, lists, tables). Each block
//! carries styled text for rendering and source position information for navigation.
//!
//! The design is inspired by plannotator-tui's document architecture:
//! - Parse markdown into typed blocks with stable IDs
//! - Each block knows its rendered height and source line range
//! - Layout pass computes screen positions within available area
//! - Navigation operates on block boundaries, not arbitrary lines

pub mod config;
pub mod doc;
pub mod layout;
pub mod nav;

pub use config::{ColorConfig, Config};
pub use doc::{Block, BlockKind, CodeStyle, Document};
pub use layout::DocLayout;
pub use nav::HeadingIndex;
