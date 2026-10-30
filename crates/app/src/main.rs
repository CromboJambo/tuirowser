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

mod config;

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
        let results = tuirowser_fetch::web_search(query, None, &fetch_config).await?;
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

    // Fetch the page
    let fetch_config = tuirowser_fetch::FetchConfig {
        user_agent: app_config.fetch.user_agent.clone(),
        max_redirects: app_config.fetch.max_redirects,
        timeout_secs: app_config.fetch.timeout_secs,
    };
    let page = tuirowser_fetch::fetch_page(url, &fetch_config).await?;
    let markdown = tuirowser_parser::html_to_markdown(&page.html)?;

    // Setup UI state
    let state = tuirowser_ui::AppState {
        url: page.url,
        content: markdown,
        running: true,
        mode: tuirowser_ui::InputMode::Normal,
        url_input: String::new(),
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

    Ok(())
}