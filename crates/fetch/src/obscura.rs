//! Obscura browser backend client.
//!
//! Talks JSON-RPC 2.0 to an external Obscura MCP server over HTTP.
//! This is the integration point for JS-capable page fetching when
//! tuirowser's native reqwest path isn't sufficient (SPAs, dynamic content).
//!
//! ## Usage
//!
//! ```ignore
//! use tuirowser_fetch::obscura::{ObscuraClient, ObscuraConfig};
//!
//! let mut client = ObscuraClient::connect(ObscuraConfig::default()).await?;
//! client.navigate("https://example.com").await?;
//! let snapshot = client.snapshot().await?;
//! ```
//!
//! ## Server modes
//!
//! - `obscura mcp --http --port 8080` → JSON-RPC at `http://127.0.0.1:8080/mcp`
//! - `obscura serve --port 9222` → CDP WebSocket (not used by this client yet)

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

/// Default Obscura MCP HTTP endpoint.
pub const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:8080/mcp";

/// Configuration for connecting to an Obscura browser backend.
#[derive(Debug, Clone)]
pub struct ObscuraConfig {
    /// Base URL of the Obscura MCP HTTP server.
    pub endpoint: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for ObscuraConfig {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_ENDPOINT.to_string(),
            timeout_secs: 30,
        }
    }
}

/// Result of a `browser_snapshot` call.
#[derive(Debug, Clone)]
pub struct PageSnapshot {
    /// Current page URL.
    pub url: String,
    /// Page title.
    pub title: String,
    /// Readable body text.
    pub body_text: String,
    /// Element references from the snapshot (for click/fill by ref).
    pub element_refs: Vec<ElementRef>,
}

/// A reference to an interactive element from a snapshot.
#[derive(Debug, Clone)]
pub struct ElementRef {
    /// Stable reference ID for this element within the snapshot.
    pub ref_id: String,
    /// Element tag name.
    pub tag: String,
    /// Accessible text or label.
    pub text: String,
    /// Whether the element is interactive (clickable, fillable).
    pub interactive: bool,
}

/// Client for interacting with an Obscura browser backend over HTTP JSON-RPC.
pub struct ObscuraClient {
    http: reqwest::Client,
    endpoint: String,
    next_id: AtomicU64,
}

impl ObscuraClient {
    /// Connect to an Obscura MCP server at the given configuration.
    pub async fn connect(config: &ObscuraConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            http,
            endpoint: config.endpoint.clone(),
            next_id: AtomicU64::new(0),
        })
    }

    /// Check if the Obscura server is reachable.
    pub async fn ping(&self) -> Result<bool> {
        match self.call("browser_close", json!({})).await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Server might reject browser_close with no open page;
                // if we got a JSON-RPC error back (not a network error), it's alive.
                let msg = e.to_string();
                if msg.contains("JSON-RPC") || msg.contains("error") {
                    Ok(true)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Navigate to a URL.
    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.call(
            "browser_navigate",
            json!({ "url": url, "waitUntil": "load" }),
        )
        .await
        .with_context(|| format!("Failed to navigate to {url}"))?;
        Ok(())
    }

    /// Wait for a CSS selector to appear.
    pub async fn wait_for(&self, selector: &str, timeout_secs: u64) -> Result<()> {
        self.call(
            "browser_wait_for",
            json!({ "selector": selector, "timeout": timeout_secs }),
        )
        .await
        .with_context(|| format!("Timeout waiting for selector {selector}"))?
        ;
        Ok(())
    }

    /// Get a snapshot of the current page (URL, title, text, element refs).
    pub async fn snapshot(&self) -> Result<PageSnapshot> {
        let result = self
            .call("browser_snapshot", json!({}))
            .await
            .context("Failed to get page snapshot")?
            .context("Obscura returned null for browser_snapshot")?;

        let url = result["url"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let title = result["title"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let body_text = result["body_text"]
            .as_str()
            .or_else(|| result.get("text").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();

        let element_refs = result
            .get("elements")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|el| {
                        Some(ElementRef {
                            ref_id: el["ref"].as_str()?.to_string(),
                            tag: el.get("tag").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                            text: el.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            interactive: el.get("interactive").and_then(|v| v.as_bool()).unwrap_or(false),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(PageSnapshot {
            url,
            title,
            body_text,
            element_refs,
        })
    }

    /// Click an element by snapshot reference ID or CSS selector.
    pub async fn click(&self, target: &str) -> Result<()> {
        // Try as ref first (e.g. "e3"), fall back to selector
        if target.starts_with('e') && target[1..].chars().all(|c| c.is_ascii_digit()) {
            self.call("browser_click", json!({ "ref": target }))
                .await
                .with_context(|| format!("Failed to click ref {target}"))?;
        } else {
            self.call("browser_click", json!({ "selector": target }))
                .await
                .with_context(|| format!("Failed to click selector {target}"))?;
        }
        Ok(())
    }

    /// Fill an input field by reference or selector.
    pub async fn fill(&self, target: &str, value: &str) -> Result<()> {
        if target.starts_with('e') && target[1..].chars().all(|c| c.is_ascii_digit()) {
            self.call(
                "browser_fill",
                json!({ "ref": target, "value": value }),
            )
            .await
            .with_context(|| format!("Failed to fill ref {target}"))?;
        } else {
            self.call(
                "browser_fill",
                json!({ "selector": target, "value": value }),
            )
            .await
            .with_context(|| format!("Failed to fill selector {target}"))?;
        }
        Ok(())
    }

    /// Evaluate JavaScript on the current page.
    pub async fn evaluate(&self, expression: &str) -> Result<Value> {
        let result = self
            .call("browser_evaluate", json!({ "expression": expression }))
            .await
            .context("Failed to evaluate JavaScript")?
            .context("Obscura returned null for browser_evaluate")?;
        Ok(result["result"].clone())
    }

    /// Get the rendered HTML of the current page.
    pub async fn get_html(&self) -> Result<String> {
        let result = self
            .evaluate("document.documentElement.outerHTML")
            .await
            .context("Failed to get page HTML")?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Close the current page and reset browser state.
    pub async fn close(&self) -> Result<()> {
        let _ = self.call("browser_close", json!({})).await;
        Ok(())
    }

    /// Execute a raw JSON-RPC 2.0 call to the Obscura MCP server.
    async fn call(&self, method: &str, params: Value) -> Result<Option<Value>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let response = self
            .http
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Failed to reach Obscura at {}", self.endpoint))?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Obscura returned HTTP {}: {}", status, response.text().await.unwrap_or_default());
        }

        let rpc_response: Value = response.json().await.context("Invalid JSON from Obscura")?;

        if let Some(error) = rpc_response.get("error") {
            anyhow::bail!("JSON-RPC error {}: {}", error["code"], error["message"]);
        }

        Ok(rpc_response.get("result").cloned())
    }
}
