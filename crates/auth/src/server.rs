//! Localhost HTTP server for ephemeral auth tokens.
//!
//! Runs on 127.0.0.1 with endpoints:
//! - POST /token/new - Generate a new auth token
//! - GET /token/validate?token=<token> - Validate and use a token
//! - GET /health - Health check

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::token::{TokenRequest, TokenStore};

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind address (default: 127.0.0.1).
    pub bind_addr: String,
    /// Port to listen on (default: 8765).
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".to_string(),
            port: 8765,
        }
    }
}

/// Start the auth token server.
pub async fn start_server(config: ServerConfig) -> Result<ServerHandle> {
    let store = Arc::new(RwLock::new(TokenStore::new()));

    let addr: SocketAddr = format!("{}:{}", config.bind_addr, config.port)
        .parse()
        .expect("Invalid bind address");

    tracing::info!("Auth server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    let store = store.clone();
                    tokio::spawn(async move {
                        handle_connection(stream, store).await;
                    });
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Auth server shutting down");
                    break;
                }
            }
        }
    });

    Ok(ServerHandle { config, shutdown_tx })
}

/// Handle for controlling the auth server.
pub struct ServerHandle {
    config: ServerConfig,
    shutdown_tx: mpsc::Sender<()>,
}

impl ServerHandle {
    /// Get the base URL of the running server.
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.config.bind_addr, self.config.port)
    }

    /// Shut down the server.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    store: Arc<RwLock<TokenStore>>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    if n == 0 {
        return;
    }

    let request = String::from_utf8_lossy(&buf[..n]);
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return;
    }

    // Parse request line
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 3 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    // Route
    if path == "/health" && method == "GET" {
        write_response(&mut stream, 200, r#"{"status":"ok"}"#).await;
    } else if path == "/token/new" && method == "POST" {
        handle_new_token(&request, &mut stream, store).await;
    } else if path.starts_with("/token/validate") && method == "GET" {
        handle_validate_token(path, &mut stream, store).await;
    } else {
        write_response(&mut stream, 404, r#"{"error":"not found"}"#).await;
    }
}

async fn write_response(stream: &mut tokio::net::TcpStream, status: u16, body: &str) {
    use tokio::io::AsyncWriteExt;

    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, status_text, body.len(), body
    );

    let _ = stream.write_all(response.as_bytes()).await;
}

async fn handle_new_token(
    request: &str,
    stream: &mut tokio::net::TcpStream,
    store: Arc<RwLock<TokenStore>>,
) {
    // Parse JSON body (simplified extraction)
    let redirect_url = extract_json_string(request, "redirect_url");
    if redirect_url.is_empty() {
        write_response(stream, 400, r#"{"error":"missing redirect_url"}"#).await;
        return;
    }

    let request_obj = TokenRequest {
        redirect_url,
        ttl_secs: None,
        multi_use: false,
    };

    match store.write().await.generate(&request_obj).await {
        Ok(token) => {
            let response = format!(
                r#"{{"token":"{}","expires_in_secs":60,"auth_url":"http://127.0.0.1:8765/token/validate?token={}"}}"#,
                token, token
            );
            write_response(stream, 200, &response).await;
        }
        Err(e) => {
            let response = format!(r#"{{"error":"{}"}}"#, e);
            write_response(stream, 500, &response).await;
        }
    }
}

async fn handle_validate_token(
    path: &str,
    stream: &mut tokio::net::TcpStream,
    store: Arc<RwLock<TokenStore>>,
) {
    // Extract token from query string
    let token = if let Some(query) = path.split('?').nth(1) {
        if let Some(pair) = query.split('&').find(|p| p.starts_with("token=")) {
            pair.split('=').nth(1).unwrap_or("").to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    if token.is_empty() {
        write_response(stream, 400, r#"{"error":"missing token"}"#).await;
        return;
    }

    match store.read().await.validate(&token).await {
        Ok(redirect_url) => {
            let _ = store.write().await.mark_used(&token).await;
            let response = format!(r#"{{"valid":true,"redirect_url":"{}"}}"#, redirect_url);
            write_response(stream, 200, &response).await;
        }
        Err(e) => {
            let response = format!(r#"{{"valid":false,"error":"{}"}}"#, e);
            write_response(stream, 401, &response).await;
        }
    }
}

/// Simple JSON string extraction (no dependency on serde_json for parsing).
fn extract_json_string(body: &str, key: &str) -> String {
    let pattern = format!("\"{}\":\"", key);
    if let Some(start) = body.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = body[value_start..].find('"') {
            return body[value_start..value_start + end].to_string();
        }
    }
    String::new()
}