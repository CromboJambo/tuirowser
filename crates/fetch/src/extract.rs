//! Web content extraction using crabjar-web-extract.
//!
//! Provides HTML and PDF content extraction to clean markdown/text,
//! powered by crabjar's web-extract crate.

use anyhow::Result;

/// Extract readable content from a web URL.
///
/// Handles both HTML pages and PDF documents automatically.
/// Returns the extracted content as clean text/markdown.
///
/// # Arguments
/// * `url` - The URL to extract content from
/// * `char_limit` - Maximum characters to return (default: 15000)
///
/// # Returns
/// The extracted content as a string, or an error if extraction fails
pub async fn extract_content(url: &str, char_limit: Option<usize>) -> Result<String> {
    let response = crabjar_web_extract::extract(&[url.to_string()], char_limit)
        .await
        .map_err(|e| anyhow::anyhow!("Web extraction failed: {}", e))?;

    if response.results.is_empty() {
        return Err(anyhow::anyhow!(
            "No results returned for URL: {}",
            url
        ));
    }

    let result = &response.results[0];
    if let Some(error) = &result.error {
        return Err(anyhow::anyhow!("Extraction error for {}: {}", url, error));
    }

    Ok(result.content.clone())
}

/// Extract readable content from multiple web URLs.
///
/// Handles both HTML pages and PDF documents automatically.
///
/// # Arguments
/// * `urls` - List of URLs to extract content from (max 5)
/// * `char_limit` - Maximum characters per page (default: 15000)
///
/// # Returns
/// A vector of extraction results with URL, title, and content for each
pub async fn extract_content_multi(
    urls: &[String],
    char_limit: Option<usize>,
) -> Result<Vec<ExtractedContent>> {
    let response = crabjar_web_extract::extract(urls, char_limit)
        .await
        .map_err(|e| anyhow::anyhow!("Web extraction failed: {}", e))?;

    let results = response
        .results
        .into_iter()
        .map(|r| ExtractedContent {
            url: r.url,
            title: r.title,
            content: r.content,
            error: r.error,
        })
        .collect();

    Ok(results)
}

/// Result of extracting content from a single URL.
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// The URL that was extracted.
    pub url: String,
    /// The page title (if available).
    pub title: String,
    /// The extracted content as clean text/markdown.
    pub content: String,
    /// Error message if extraction failed for this URL.
    pub error: Option<String>,
}
