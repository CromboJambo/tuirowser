//! Configuration and theming for tuirowser.
//!
//! Config is loaded from `~/.config/tuirowser/config.toml` (or `$XDG_CONFIG_HOME/tuirowser/config.toml`).
//! If the file doesn't exist, sensible defaults are used.

use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::Deserialize;
use std::path::PathBuf;

/// Main configuration structure.
#[derive(Debug, Clone, Default, serde::Serialize, Deserialize)]
pub struct Config {
    /// Color theming options.
    #[serde(default)]
    pub colors: ColorConfig,

    /// Herdr integration options.
    #[serde(default)]
    pub herdr: HerdrConfig,

    /// UI layout options.
    #[serde(default)]
    pub ui: UiConfig,
}

/// Color theming configuration.
#[derive(Debug, Clone, Default, serde::Serialize, Deserialize)]
pub struct ColorConfig {
    /// Text color for level-1 headings.
    #[serde(default = "default_h1_color")]
    pub heading_1: String,

    /// Text color for level-2 headings.
    #[serde(default = "default_h2_color")]
    pub heading_2: String,

    /// Text color for level-3+ headings.
    #[serde(default = "default_h3_color")]
    pub heading_3: String,

    /// Background color for code blocks.
    #[serde(default = "default_code_bg")]
    pub code_background: String,

    /// Text color for code blocks.
    #[serde(default = "default_code_fg")]
    pub code_foreground: String,

    /// Text color for blockquotes.
    #[serde(default = "default_quote_color")]
    pub quote: String,

    /// Text color for URLs and links.
    #[serde(default = "default_link_color")]
    pub link: String,
}

impl ColorConfig {
    /// Parse a color string into a ratatui Color.
    pub fn parse(&self, hex_or_name: &str) -> Color {
        parse_color(hex_or_name)
    }
}

/// Herdr integration configuration.
#[derive(Debug, Clone, Default, serde::Serialize, Deserialize)]
pub struct HerdrConfig {
    /// How to place the viewer when launched from herdr.
    #[serde(default = "default_placement")]
    pub placement: String, // "overlay" | "split" | "popup"

    /// Whether to show the herdr badge in the title bar.
    #[serde(default = "default_show_badge")]
    pub show_badge: bool,
}

/// UI layout configuration.
#[derive(Debug, Clone, Default, serde::Serialize, Deserialize)]
pub struct UiConfig {
    /// Maximum document width before wrapping (0 = auto).
    #[serde(default)]
    pub doc_width: usize,

    /// Number of lines to scroll per page down/up.
    #[serde(default = "default_scroll_lines")]
    pub scroll_lines: usize,
}

// Default value functions for serde
fn default_h1_color() -> String { "light_magenta".to_string() }
fn default_h2_color() -> String { "light_cyan".to_string() }
fn default_h3_color() -> String { "light_green".to_string() }
fn default_code_bg() -> String { "#1e1e1e".to_string() }
fn default_code_fg() -> String { "light_cyan".to_string() }
fn default_quote_color() -> String { "light_yellow".to_string() }
fn default_link_color() -> String { "cyan".to_string() }
fn default_placement() -> String { "overlay".to_string() }
fn default_show_badge() -> bool { true }
fn default_scroll_lines() -> usize { 3 }

/// Parse a color specification string into a ratatui Color.
/// Supports: named colors ("red", "light_cyan"), hex ("#ff0000", "ff0000"), and RGB tuples via env.
pub fn parse_color(spec: &str) -> Color {
    let spec = spec.trim();

    // Try hex color
    if spec.starts_with('#') || (spec.len() == 6 && spec.chars().all(|c| c.is_ascii_hexdigit())) {
        let hex = spec.strip_prefix('#').unwrap_or(spec);
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
    }

    // Named colors
    match spec.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "white" => Color::White,
        "light_black" | "dark_gray" | "dark_grey" => Color::DarkGray,
        "light_red" | "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "light_gray" | "light_grey" => Color::Gray, // ratatui doesn't have light gray distinct from gray
        _ => Color::Reset,
    }
}

impl Config {
    /// Load configuration from the standard location.
    /// Returns default config if the file doesn't exist or fails to parse.
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            match toml::from_str::<Config>(&content) {
                Ok(config) => {
                    tracing::info!("Loaded config from {}", path.display());
                    return config;
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config {}: {}", path.display(), e);
                }
            }
        }
        Config::default()
    }

    /// Get the config file path.
    pub fn config_path() -> PathBuf {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("tuirowser").join("config.toml");
        }
        directories::ProjectDirs::from("com", "crombo", "tuirowser")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("~/.config/tuirowser"))
            .join("config.toml")
    }

    /// Create the config directory if it doesn't exist.
    pub fn ensure_config_dir() -> Result<PathBuf> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
        }
        Ok(path)
    }

    /// Write the current config to disk (for `tuirowser config init`).
    pub fn save(&self) -> Result<PathBuf> {
        let path = Self::ensure_config_dir()?;
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(path)
    }
}
