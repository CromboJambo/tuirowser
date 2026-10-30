//! Bookmark and history management with TOML persistence.

use anyhow::{Context, Result};
use directories::UserDirs;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// A bookmark entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bookmark {
    pub url: String,
    pub title: Option<String>,
}

/// History entry with timestamp
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: Option<String>,
    pub timestamp: u64,
}

/// Manages bookmarks and browsing history with file persistence.
pub struct BookmarksManager {
    bookmarks_path: PathBuf,
    history_path: PathBuf,
    bookmarks: HashMap<String, Bookmark>,
    history: Vec<HistoryEntry>,
}

impl BookmarksManager {
    /// Create a new bookmarks manager, loading existing data from disk.
    pub fn new() -> Result<Self> {
        // Use HOME directory for config storage
        let home = std::env::var("HOME").context("Could not determine HOME directory")?;
        let config_dir = PathBuf::from(home).join(".config").join("tuirowser");

        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        let bookmarks_path = config_dir.join("bookmarks.toml");
        let history_path = config_dir.join("history.toml");

        let mut manager = Self {
            bookmarks: HashMap::new(),
            history: Vec::new(),
            bookmarks_path,
            history_path,
        };

        manager.load_bookmarks()?;
        manager.load_history()?;

        Ok(manager)
    }

    /// Add a bookmark for the current URL.
    pub fn add_bookmark(&mut self, url: &str, title: Option<&str>) -> Result<()> {
        let bookmark = Bookmark {
            url: url.to_string(),
            title: title.map(|t| t.to_string()),
        };

        self.bookmarks.insert(url.to_string(), bookmark);
        self.save_bookmarks()?;

        Ok(())
    }

    /// List all bookmarks.
    pub fn list_bookmarks(&self) -> Vec<&Bookmark> {
        let mut bookmarks: Vec<&Bookmark> = self.bookmarks.values().collect();
        bookmarks.sort_by(|a, b| a.url.cmp(&b.url));
        bookmarks
    }

    /// Add a URL to browsing history.
    pub fn add_to_history(&mut self, url: &str, title: Option<&str>) -> Result<()> {
        // Remove existing entry for this URL if it exists
        self.history.retain(|e| e.url != url);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = HistoryEntry {
            url: url.to_string(),
            title: title.map(|t| t.to_string()),
            timestamp,
        };

        self.history.push(entry);

        // Keep only last 100 entries
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        self.save_history()?;

        Ok(())
    }

    /// Get recent history entries.
    pub fn get_recent_history(&self, limit: usize) -> Vec<&HistoryEntry> {
        let mut history: Vec<&HistoryEntry> = self.history.iter().collect();
        // Sort by timestamp (newest first)
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        history.into_iter().take(limit).collect()
    }

    /// Search history for URLs matching a query.
    pub fn search_history(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.history
            .iter()
            .filter(|e| e.url.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Load bookmarks from disk.
    fn load_bookmarks(&mut self) -> Result<()> {
        if !self.bookmarks_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.bookmarks_path)
            .context("Failed to read bookmarks file")?;

        let parsed: HashMap<String, Bookmark> = toml::from_str(&content)
            .context("Failed to parse bookmarks TOML")?;

        self.bookmarks = parsed;
        Ok(())
    }

    /// Save bookmarks to disk.
    fn save_bookmarks(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self.bookmarks)
            .context("Failed to serialize bookmarks")?;

        fs::write(&self.bookmarks_path, content)
            .context("Failed to write bookmarks file")?;

        Ok(())
    }

    /// Load history from disk.
    fn load_history(&mut self) -> Result<()> {
        if !self.history_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.history_path)
            .context("Failed to read history file")?;

        let parsed: Vec<HistoryEntry> = toml::from_str(&content)
            .context("Failed to parse history TOML")?;

        self.history = parsed;
        Ok(())
    }

    /// Save history to disk.
    fn save_history(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self.history)
            .context("Failed to serialize history")?;

        fs::write(&self.history_path, content)
            .context("Failed to write history file")?;

        Ok(())
    }
}