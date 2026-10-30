//! tuirowser - A TUI web browser that simplifies pages to markdown.
//!
//! Usage:
//!   tuirowser                    # Start with about:blank
//!   tuirowser <url>              # Open a specific URL
//!   tuirowser --dump <url>       # Print page as markdown and exit (no TUI)
//!   tuirowser --port 8090        # Enable HTTP command server on specified port

use anyhow::Result;
use std::env;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing_subscriber;

mod bookmarks;
mod config;
mod rss;

/// Download an image and convert it to ASCII art.
async fn download_image_as_ascii(url: &str, width: usize) -> Result<String> {
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        anyhow::bail!("Failed to download image from {}", url);
    }

    let bytes = response.bytes().await?;

    // Convert to ASCII art (blocking, but images are typically small)
    let ascii = tokio::task::spawn_blocking(move || {
        tuirowser_parser::image_to_ascii(&bytes, width)
    })
    .await??;

    Ok(ascii)
}

/// Process markdown content: download images and convert to ASCII art.
async fn process_images_in_markdown(content: &str, page_url: &str) -> Result<String> {
    use regex::Regex;

    // Find all markdown image syntax: ![alt](url)
    let image_pattern = Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)")?;

    let mut result = content.to_string();
    for caps in image_pattern.captures_iter(content) {
        let full_match = &caps[0];
        let url = &caps[2];

        // Resolve relative URLs
        let abs_url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if url.starts_with("/") {
            // Absolute path on the same host
            let scheme_host = page_url.split("/").take(3).collect::<Vec<_>>().join("/");
            format!("{}{}", scheme_host, url)
        } else {
            // Relative path
            let base = page_url.rsplit_once("/").map(|(b, _)| b).unwrap_or(page_url);
            format!("{}/{}", base, url)
        };

        // Try to download and convert to ASCII art
        match download_image_as_ascii(&abs_url, 60).await {
            Ok(ascii_art) => {
                result = result.replace(full_match, &ascii_art);
            }
            Err(e) => {
                // If download fails, keep the original markdown image syntax
                tracing::debug!("Failed to render image {}: {}", abs_url, e);
            }
        }
    }

    Ok(result)
}

async fn http_command_server(
    port: u16,
    tx: mpsc::Sender<tuirowser_ui::BrowserCommand>,
) -> Result<()> {
    use axum::{extract::Query, routing::post, Json, Router};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct NavigateQuery { url: String }

    let app_state = Arc::new(Mutex::new(tx));

    async fn handle_navigate(
        Query(q): Query<NavigateQuery>,
        state: axum::extract::State<Arc<Mutex<mpsc::Sender<tuirowser_ui::BrowserCommand>>>>,
    ) -> Json<serde_json::Value> {
        let tx = state.lock().await;
        let _ = tx.send(tuirowser_ui::BrowserCommand::Navigate(q.url)).await;
        Json(serde_json::json!({"status": "navigating"}))
    }

    async fn handle_reload(
        state: axum::extract::State<Arc<Mutex<mpsc::Sender<tuirowser_ui::BrowserCommand>>>>,
    ) -> Json<serde_json::Value> {
        let tx = state.lock().await;
        let _ = tx.send(tuirowser_ui::BrowserCommand::Reload).await;
        Json(serde_json::json!({"status": "reloading"}))
    }

    async fn handle_quit(
        state: axum::extract::State<Arc<Mutex<mpsc::Sender<tuirowser_ui::BrowserCommand>>>>,
    ) -> Json<serde_json::Value> {
        let tx = state.lock().await;
        let _ = tx.send(tuirowser_ui::BrowserCommand::Quit).await;
        Json(serde_json::json!({"status": "quitting"}))
    }

    let app = Router::new()
        .route("/navigate", post(handle_navigate))
        .route("/reload", post(handle_reload))
        .route("/quit", post(handle_quit))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("HTTP command server listening on 127.0.0.1:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let app_config = config::Config::load()?;
    tracing::info!("Loaded configuration");

    // Start ephemeral auth token server
    let auth_server = tuirowser_auth::start_server(tuirowser_auth::ServerConfig {
        bind_addr: "127.0.0.1".to_string(),
        port: 8765,
    }).await?;
    tracing::info!("Auth token server started on {}", auth_server.base_url());

    let args: Vec<String> = env::args().collect();

    // Check for --dump mode (scripting-friendly, no TUI)
    if args.contains(&"--dump".to_string()) {
        let url = args
            .iter()
            .find(|arg| arg.starts_with("http"))
            .ok_or_else(|| anyhow::anyhow!("No URL provided for --dump mode"))?;

        let fetch_config = tuirowser_fetch::FetchConfig {
            user_agent: app_config.fetch.user_agent.clone(),
            max_redirects: app_config.fetch.max_redirects,
            timeout_secs: app_config.fetch.timeout_secs,
        };
        let page = tuirowser_fetch::fetch_page(url, &fetch_config).await?;
        let markdown = tuirowser_parser::html_to_markdown(&page.html)?;
        print!("{}", markdown);
        return Ok(());
    }

    // Check for --search mode (scripting-friendly search)
    if args.contains(&"--search".to_string()) {
        let query = args
            .iter()
            .find(|arg| !arg.starts_with("--") && arg.as_str() != &args[0])
            .ok_or_else(|| anyhow::anyhow!("No search query provided"))?;

        let fetch_config = tuirowser_fetch::FetchConfig {
            user_agent: app_config.fetch.user_agent.clone(),
            max_redirects: app_config.fetch.max_redirects,
            timeout_secs: app_config.fetch.timeout_secs,
        };
        let results = tuirowser_fetch::web_search(
            query,
            Some(&app_config.search.searxng_url),
            &fetch_config,
        ).await?;
        print!("{}", results);
        return Ok(());
    }

    // Check for --port flag to enable HTTP command server
    let http_port = args
        .iter()
        .position(|arg| arg == "--port")
        .and_then(|pos| args.get(pos + 1))
        .and_then(|p| p.parse::<u16>().ok());

    // Normal TUI mode
    let default_url = "about:blank".to_string();
    let url = args
        .iter()
        .find(|arg| arg.starts_with("http"))
        .unwrap_or(&default_url);

    // Fetch the page or feed
    let fetch_config = tuirowser_fetch::FetchConfig {
        user_agent: app_config.fetch.user_agent.clone(),
        max_redirects: app_config.fetch.max_redirects,
        timeout_secs: app_config.fetch.timeout_secs,
    };

    let (page_url, markdown) = if url.starts_with("about:") {
        (url.clone(), "# TUIrowser\n\nWelcome to TUIrowser - a TUI web browser and RSS reader.\n\nType `g` to navigate to a URL or feed.".to_string())
    } else {
        // Try to detect if it's an RSS/Atom feed
        match rss::fetch_feed(url).await {
            Ok(feed) => {
                let markdown = rss::feed_to_markdown(&feed);
                (url.clone(), markdown)
            }
            Err(_) => {
                // Not a feed, fetch as regular page
                let page = tuirowser_fetch::fetch_page(url, &fetch_config).await?;
                let markdown = tuirowser_parser::html_to_markdown(&page.html)?;
                (page.url.clone(), markdown)
            }
        }
    };

    // Process images: download and convert to ASCII art
    let markdown_with_images = process_images_in_markdown(&markdown, &page_url).await?;

    // Setup UI state
    let bm_manager = bookmarks::BookmarksManager::new()?;
    let bm_list = bm_manager.list_bookmarks();
    let history_list = bm_manager.get_recent_history(50);

    let state = tuirowser_ui::AppState {
        url: page_url,
        content: markdown_with_images,
        running: true,
        mode: tuirowser_ui::InputMode::Normal,
        url_input: String::new(),
        scroll_offset: 0,
        bookmarks: bm_list.iter().map(|b| (b.url.clone(), b.title.clone())).collect(),
        history: history_list.iter().map(|h| (h.url.clone(), h.title.clone(), h.timestamp)).collect(),
    };

    // If HTTP port is specified, start command server and pass channel to run_app
    if let Some(port) = http_port {
        let (tx, rx) = mpsc::channel(32);
        let server_task = tokio::spawn(http_command_server(port, tx));

        // Run TUI with HTTP command channel
        tuirowser_ui::run_app_with_commands(state, rx).await?;

        server_task.abort();
    } else {
        // Run standard TUI without HTTP commands
        tuirowser_ui::run_app(state)?;
    }

    // Shut down auth server
    auth_server.shutdown().await;
    tracing::info!("Auth token server shut down");

    Ok(())
}