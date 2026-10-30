//! In-memory token store for ephemeral auth tokens.
//!
//! Tokens are stored in a thread-safe HashMap with automatic expiration.
//! No persistent storage — tokens die when the process exits.

use anyhow::{anyhow, Result};
use chrono::Utc;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Default token TTL in seconds (60s = 1 minute)
pub const DEFAULT_TTL_SECS: u64 = 60;

/// A request to generate a new auth token.
#[derive(Debug, Clone)]
pub struct TokenRequest {
    /// The URL the user should be redirected to after auth.
    pub redirect_url: String,
    /// Optional TTL override in seconds (defaults to 60).
    pub ttl_secs: Option<u64>,
    /// Whether the token can be used multiple times (default: false).
    pub multi_use: bool,
}

/// A stored auth token with metadata.
#[derive(Debug)]
struct StoredToken {
    /// The raw token string.
    token: String,
    /// When this token expires (Unix timestamp).
    expires_at: u64,
    /// Whether this token has been used already.
    used: bool,
    /// Whether the token can be used multiple times.
    multi_use: bool,
    /// The redirect URL associated with this token.
    redirect_url: String,
}

/// Thread-safe in-memory store for ephemeral auth tokens.
#[derive(Debug)]
pub struct TokenStore {
    tokens: Arc<RwLock<HashMap<String, StoredToken>>>,
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TokenStore {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens.clone(),
        }
    }
}

impl TokenStore {
    /// Create a new empty token store.
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new cryptographically random auth token.
    ///
    /// Returns the token string and stores it with expiration metadata.
    pub async fn generate(&self, request: &TokenRequest) -> Result<String> {
        // Generate 64-byte random hex token (128 bytes of randomness)
        let bytes: [u8; 64] = rand::random();
        let token: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();

        let ttl = request.ttl_secs.unwrap_or(DEFAULT_TTL_SECS);
        let expires_at = Utc::now().timestamp() + ttl as i64;

        let stored = StoredToken {
            token: token.clone(),
            expires_at: expires_at as u64,
            used: false,
            multi_use: request.multi_use,
            redirect_url: request.redirect_url.clone(),
        };

        self.tokens.write().await.insert(token.clone(), stored);

        // Log token generation (in production, use tracing)
        tracing::info!(
            "Generated auth token {} for {}, expires in {}s",
            Uuid::new_v4(),
            request.redirect_url,
            ttl
        );

        Ok(token)
    }

    /// Validate a token and mark it as used (if single-use).
    ///
    /// Returns the redirect URL if valid, or an error if expired/used/invalid.
    pub async fn validate(&self, token: &str) -> Result<String> {
        let tokens = self.tokens.read().await;

        let stored = tokens.get(token).ok_or_else(|| anyhow!("Invalid token"))?;

        // Check expiration
        let now = Utc::now().timestamp() as u64;
        if now > stored.expires_at {
            return Err(anyhow!("Token expired"));
        }

        // Check if already used (single-use tokens)
        if stored.used && !stored.multi_use {
            return Err(anyhow!("Token already used"));
        }

        Ok(stored.redirect_url.clone())
    }

    /// Mark a token as used.
    pub async fn mark_used(&self, token: &str) -> Result<()> {
        let mut tokens = self.tokens.write().await;

        if let Some(stored) = tokens.get_mut(token) {
            stored.used = true;
            tracing::info!("Token marked as used");
        }

        Ok(())
    }

    /// Clean up expired and used tokens.
    ///
    /// Should be called periodically (e.g., every minute).
    pub async fn cleanup(&self) -> usize {
        let mut tokens = self.tokens.write().await;

        let now = Utc::now().timestamp() as u64;
        let before = tokens.len();

        // Remove expired tokens
        tokens.retain(|_, stored| now <= stored.expires_at);

        // Remove used single-use tokens
        tokens.retain(|_, stored| !stored.used || stored.multi_use);

        let removed = before.saturating_sub(tokens.len());
        if removed > 0 {
            tracing::info!("Cleaned up {} expired/used tokens", removed);
        }

        removed
    }

    /// Get the number of active (non-expired, non-used) tokens.
    pub async fn active_count(&self) -> usize {
        let tokens = self.tokens.read().await;

        let now = Utc::now().timestamp() as u64;
        tokens.values().filter(|t| now <= t.expires_at && !t.used).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_and_validate() {
        let store = TokenStore::new();

        let request = TokenRequest {
            redirect_url: "http://example.com/callback".to_string(),
            ttl_secs: Some(120),
            multi_use: false,
        };

        let token = store.generate(&request).await.unwrap();
        assert_eq!(token.len(), 128); // 64 bytes hex-encoded

        // Validate should succeed
        let redirect = store.validate(&token).await.unwrap();
        assert_eq!(redirect, "http://example.com/callback");
    }

    #[tokio::test]
    async fn test_single_use_token() {
        let store = TokenStore::new();

        let request = TokenRequest {
            redirect_url: "http://example.com".to_string(),
            ttl_secs: None,
            multi_use: false,
        };

        let token = store.generate(&request).await.unwrap();

        // First use should succeed
        store.validate(&token).await.unwrap();
        store.mark_used(&token).await.unwrap();

        // Second use should fail
        assert!(store.validate(&token).await.is_err());
    }

    #[tokio::test]
    async fn test_multi_use_token() {
        let store = TokenStore::new();

        let request = TokenRequest {
            redirect_url: "http://example.com".to_string(),
            ttl_secs: None,
            multi_use: true,
        };

        let token = store.generate(&request).await.unwrap();

        // Multiple uses should succeed
        store.validate(&token).await.unwrap();
        store.mark_used(&token).await.unwrap();
        store.validate(&token).await.unwrap();
    }

    #[tokio::test]
    async fn test_expired_token() {
        let store = TokenStore::new();

        let request = TokenRequest {
            redirect_url: "http://example.com".to_string(),
            ttl_secs: Some(1), // Expire in 1 second
            multi_use: false,
        };

        let token = store.generate(&request).await.unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        assert!(store.validate(&token).await.is_err());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let store = TokenStore::new();

        // Generate some tokens that will expire quickly
        for _ in 0..5 {
            let request = TokenRequest {
                redirect_url: "http://example.com".to_string(),
                ttl_secs: Some(1),
                multi_use: false,
            };
            store.generate(&request).await.unwrap();
        }

        assert_eq!(store.active_count().await, 5);

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        let removed = store.cleanup().await;
        assert_eq!(removed, 5);
        assert_eq!(store.active_count().await, 0);
    }
}