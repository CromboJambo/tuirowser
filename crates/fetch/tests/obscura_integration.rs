#![cfg(feature = "obscura")]

use tuirowser_fetch::obscura::{ObscuraClient, ObscuraConfig};

/// Integration test: requires an Obscura MCP server running at 127.0.0.1:8080.
/// Run with: obscura mcp --http --port 8080 & cargo test -p tuirowser-fetch --features obscura --test obscura_integration
#[tokio::test]
#[ignore = "requires running Obscura MCP server"]
async fn test_obscura_navigate_and_snapshot() {
    let client = ObscuraClient::connect(&ObscuraConfig::default())
        .await
        .expect("Failed to create client");

    // Navigate to a simple page
    client.navigate("https://example.com").await.expect("navigate failed");

    // Get snapshot
    let snapshot = client.snapshot().await.expect("snapshot failed");
    assert!(snapshot.url.contains("example.com"));
    assert!(!snapshot.title.is_empty());
    assert!(snapshot.body_text.len() > 10);

    // Cleanup
    client.close().await.ok();
}

#[tokio::test]
#[ignore = "requires running Obscura MCP server"]
async fn test_obscura_evaluate_js() {
    let client = ObscuraClient::connect(&ObscuraConfig::default())
        .await
        .expect("Failed to create client");

    client.navigate("https://example.com").await.expect("navigate failed");

    // Evaluate JS to get the title
    let result = client.evaluate("document.title").await.expect("evaluate failed");
    let title = result.as_str().expect("title should be a string");
    assert!(title.contains("Example"));

    client.close().await.ok();
}
