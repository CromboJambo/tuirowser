use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// TUIrowser application configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// HTTP fetch settings.
    pub fetch: FetchConfig,
    /// User interface settings.
    pub ui: UiConfig,
}

/// HTTP client configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FetchConfig {
    /// User-Agent string to send with requests.
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    /// Maximum number of redirects to follow.
    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

/// User interface configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Color theme ("dark" or "light").
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_user_agent() -> String {
    "Mozilla/5.0 (compatible; TUIrowser/1.0)".to_string()
}

fn default_max_redirects() -> usize {
    10
}

fn default_timeout_secs() -> u64 {
    30
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            fetch: FetchConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for FetchConfig {
    fn default() -> Self {
        FetchConfig {
            user_agent: default_user_agent(),
            max_redirects: default_max_redirects(),
            timeout_secs: default_timeout_secs(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            theme: default_theme(),
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults.
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Get the path to the configuration file.
    fn config_path() -> PathBuf {
        match std::env::home_dir() {
            Some(home) => home.join(".tuirowser.toml"),
            None => PathBuf::from(".tuirowser.toml"),
        }
    }
}
