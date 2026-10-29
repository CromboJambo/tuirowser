//! tuirowser - A TUI web browser that simplifies pages to markdown.
//!
//! Usage:
//!   tuirowser                    # Start with about:blank
//!   tuirowser <url>              # Open a specific URL
//!   tuirowser --dump <url>       # Print page as markdown and exit (no TUI)

use anyhow::Result;
use std::env;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    // Check for --dump mode (scripting-friendly, no TUI)
    if args.contains(&"--dump".to_string()) {
        let url = args
            .iter()
            .find(|arg| arg.starts_with("http"))
            .ok_or_else(|| anyhow::anyhow!("No URL provided for --dump mode"))?;

        let config = tuirowser_fetch::FetchConfig::default();
        let page = tuirowser_fetch::fetch_page(url, &config).await?;
        let markdown = tuirowser_parser::html_to_markdown(&page.html)?;
        print!("{}", markdown);
        return Ok(());
    }

    // Normal TUI mode
    let default_url = "about:blank".to_string();
    let url = args
        .iter()
        .find(|arg| arg.starts_with("http"))
        .unwrap_or(&default_url);

    // Fetch the page
    let config = tuirowser_fetch::FetchConfig::default();
    let page = tuirowser_fetch::fetch_page(url, &config).await?;
    let markdown = tuirowser_parser::html_to_markdown(&page.html)?;

    // Setup UI state
    let state = tuirowser_ui::AppState {
        url: page.url,
        content: markdown,
        running: true,
    };

    // Run the TUI
    tuirowser_ui::run_app(state)?;

    Ok(())
}