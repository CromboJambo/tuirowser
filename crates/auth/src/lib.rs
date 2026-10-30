//! Ephemeral SSH-style authentication for interactive browser sessions.
//!
//! This module provides a localhost HTTP server that generates short-lived,
//! single-use authentication tokens — like an SSH key left under the door
//! that only works once and times out responsibly.
//!
//! ## Architecture
//!
//! 1. TUIrowser detects form/password fields in HTML
//! 2. Requests a token from the auth server (`/token/new`)
//! 3. Opens user's real browser via `xdg-open` with the token URL
//! 4. User completes interactive auth in their browser
//! 5. Token is validated and cleaned up after use
//!
//! ## Security Model
//!
//! - Tokens are cryptographically random (64-byte hex)
//! - Default TTL: 60 seconds
//! - Single-use by default (can be configured for multi-use)
//! - In-memory storage only — no persistent secrets
//! - Server runs on localhost only (127.0.0.1)

mod token;
pub mod server;

pub use token::{TokenStore, TokenRequest};