//! Tab management, history, and bookmark persistence.
//!
//! This crate handles browser state: open tabs, navigation history, bookmarks,
//! and session restoration across restarts.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single browser tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    /// Current URL of the tab.
    pub url: String,
    /// Title of the page (if known).
    pub title: Option<String>,
    /// Navigation history for this tab.
    pub history: Vec<String>,
    /// Current position in history.
    pub history_index: usize,
}

impl Tab {
    /// Create a new tab with the given URL.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            title: None,
            history: vec![url.to_string()],
            history_index: 0,
        }
    }

    /// Navigate to a new URL in this tab.
    pub fn navigate(&mut self, url: &str) {
        // Truncate forward history
        if self.history_index < self.history.len() - 1 {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(url.to_string());
        self.history_index = self.history.len() - 1;
        self.url = url.to_string();
    }
}

/// Browser session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// All open tabs.
    pub tabs: Vec<Tab>,
    /// Index of the currently active tab.
    pub active_tab: usize,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            tabs: vec![Tab::new("about:blank")],
            active_tab: 0,
        }
    }
}

/// Get the path to the session file.
pub fn session_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map(PathBuf::from).or_else(|_| {
        std::env::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))
    })?;
    Ok(home.join(".config").join("termview").join("session.json"))
}

/// Save the current session to disk.
pub fn save_session(session: &Session) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Load a previously saved session from disk.
pub fn load_session() -> Result<Session> {
    let path = session_path()?;
    if !path.exists() {
        return Ok(Session::default());
    }
    let json = std::fs::read_to_string(&path)?;
    let session: Session = serde_json::from_str(&json)?;
    Ok(session)
}