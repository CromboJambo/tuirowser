//! Search functionality using crabjar-web-search (multi-backend).
//!
//! Provides web search capabilities via crabjar's multi-backend search engine
//! (SearXNG primary, Brave fallback, DuckDuckGo final fallback) and formats
//! results as markdown for display in the TUI browser.

use anyhow::Result;
use std::env;

/// Perform a web search using crabjar-web-search and return formatted markdown results.
///
/// Uses crabjar's multi-backend search (SearXNG > Brave > DuckDuckGo).
/// The `searxng_url` parameter sets the SEARXNG_URL env var for this call if provided.
///
/// # Arguments
/// * `query` - The search query string
/// * `searxng_url` - URL of the SearXNG instance (optional; sets SEARXNG_URL for this call)
/// * `limit` - Maximum number of results to return
///
/// # Returns
/// Markdown-formatted search results with numbered links
pub async fn web_search(
    query: &str,
    searxng_url: Option<&str>,
    limit: usize,
) -> Result<String> {
    // If a specific SearXNG URL is provided, set it for this call
    if let Some(url) = searxng_url {
        unsafe { env::set_var("SEARXNG_URL", url); }
    }

    let response = crabjar_web_search::search(query, Some(limit))
        .await
        .map_err(|e| anyhow::anyhow!("Web search failed: {}", e))?;

    // Format results as markdown
    let mut output = format!("# Search Results for: {}\n\n", query);

    if response.results.is_empty() {
        output.push_str("No results found.\n");
    } else {
        for (i, result) in response.results.iter().enumerate() {
            let num = i + 1;
            output.push_str(&format!("{}. [{}]({})\n", num, result.title, result.url));

            if let Some(desc) = &result.description {
                let preview = truncate(desc, 200);
                output.push_str(&format!("   {}\n", preview));
            } else if let Some(snippet) = &result.snippet {
                let preview = truncate(snippet, 200);
                output.push_str(&format!("   {}\n", preview));
            }

            output.push('\n');
        }
    }

    Ok(output)
}

/// Perform a web search and return structured results (for programmatic use).
///
/// # Arguments
/// * `query` - The search query string
/// * `searxng_url` - URL of the SearXNG instance (optional; sets SEARXNG_URL for this call)
/// * `limit` - Maximum number of results to return
///
/// # Returns
/// Structured search response with URLs, titles, and descriptions
pub async fn web_search_structured(
    query: &str,
    searxng_url: Option<&str>,
    limit: usize,
) -> Result<crabjar_web_search::SearchResponse> {
    if let Some(url) = searxng_url {
        unsafe { env::set_var("SEARXNG_URL", url); }
    }

    crabjar_web_search::search(query, Some(limit))
        .await
        .map_err(|e| anyhow::anyhow!("Web search failed: {}", e))
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        // Truncate at char boundary to avoid splitting UTF-8
        let mut end = max_chars;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}
