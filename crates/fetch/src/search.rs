//! Search functionality using SearXNG metasearch engine.
//!
//! Provides web search capabilities by querying a SearXNG instance
//! and formatting results as markdown for display in the TUI browser.

use anyhow::Result;
use urlencoding::encode;

/// Default public SearXNG instance.
const DEFAULT_SEARXNG_URL: &str = "http://localhost:8080";

/// Search result item from SearXNG API.
#[derive(Debug, serde::Deserialize)]
struct SearchResult {
    title: String,
    url: String,
    content: Option<String>,
}

/// Search response from SearXNG JSON API.
#[derive(Debug, serde::Deserialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

/// Perform a web search using SearXNG and return formatted markdown results.
///
/// # Arguments
/// * `query` - The search query string
/// * `searxng_url` - URL of the SearXNG instance (optional, uses default if None)
/// * `config` - Fetch configuration for HTTP client settings
///
/// # Returns
/// Markdown-formatted search results with numbered links
pub async fn web_search(
    query: &str,
    searxng_url: Option<&str>,
    config: &super::FetchConfig,
) -> Result<String> {
    let base_url = searxng_url.unwrap_or(DEFAULT_SEARXNG_URL);

    // Build the search URL with JSON format parameter
    let encoded_query = encode(query);
    let search_url = format!(
        "{}/search?q={}&format=json&language=all",
        base_url, encoded_query
    );

    // Fetch the search results
    let page = super::fetch_page(&search_url, config).await?;

    // Check if we got JSON back
    let trimmed = page.html.trim();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return Err(anyhow::anyhow!(
            "SearXNG instance at {} did not return JSON. \n\
             Make sure your SearXNG instance supports the JSON API (format=json). \n\
             Response started with: {}",
            base_url,
            &trimmed[..trimmed.chars().take(100).count()]
        ));
    }

    // Parse the JSON response
    let response: SearchResponse = serde_json::from_str(&page.html)?;

    // Format results as markdown
    let mut output = format!("# Search Results for: {}\n\n", query);

    if response.results.is_empty() {
        output.push_str("No results found.\n");
    } else {
        for (i, result) in response.results.iter().enumerate() {
            let num = i + 1;
            output.push_str(&format!("{}. [{}]({})\n", num, result.title, result.url));

            if let Some(content) = &result.content {
                // Truncate content preview to reasonable length
                let preview = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content.clone()
                };
                output.push_str(&format!("   {}\n", preview));
            }

            output.push('\n');
        }
    }

    Ok(output)
}
