//! HTTP client for fetching web pages with cookie support.
//!
//! This crate provides a high-level API for fetching URLs, managing cookies,
//! and handling common HTTP concerns like user-agent headers and redirects.

use anyhow::Result;
use reqwest::Client;

/// Configuration for the HTTP client.
#[derive(Debug, Clone)]
pub struct FetchConfig {
    /// User-Agent string to send with requests.
    pub user_agent: String,
    /// Maximum number of redirects to follow.
    pub max_redirects: usize,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            user_agent: "termview/0.1.0 (TUI markdown browser)".to_string(),
            max_redirects: 10,
            timeout_secs: 30,
        }
    }
}

/// Result of a page fetch operation.
#[derive(Debug)]
pub struct PageContent {
    /// The final URL after redirects.
    pub url: String,
    /// The raw HTML content of the page.
    pub html: String,
    /// The HTTP status code.
    pub status: u16,
}

/// Fetch a web page and return its content.
pub async fn fetch_page(url: &str, config: &FetchConfig) -> Result<PageContent> {
    let client = Client::builder()
        .user_agent(&config.user_agent)
        .redirect(reqwest::redirect::Policy::limited(config.max_redirects))
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()?;

    let response = client.get(url).send().await?;
    let status = response.status().as_u16();
    let final_url = response.url().to_string();
    let html = response.text().await?;

    Ok(PageContent {
        url: final_url,
        html,
        status,
    })
}