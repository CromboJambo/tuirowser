#!/bin/bash
# Demo: Using tuiwright MCP server to drive TUIrowser
#
# This script demonstrates the workflow for testing a Rust TUI app
# using the tuiwright MCP server and Hermes Agent.
#
# Prerequisites:
#   - TUIrowser built (cargo build --release)
#   - tuiwright installed (cargo install tuiwright-mcp)
#   - Hermes configured with tuiwright MCP server

echo "=== TUIrowser + tuiwright Demo ==="
echo ""

# Step 1: Show the config
echo "Step 1: Verify tuiwright config in ~/.hermes/config.yaml"
grep -A 4 "tuiwright:" ~/.hermes/config.yaml | head -5
echo ""

# Step 2: Show the project config
echo "Step 2: Project-specific tuiwright.toml"
cat /home/crombo/projects/experiments/tuirowser/tuiwright.toml
echo ""

# Step 3: Demonstrate CLI usage (without MCP)
echo "Step 3: Direct TUIrowser CLI usage"
echo "  Fetching example.com..."
/home/crombo/projects/experiments/tuirowser/target/release/tuirowser --dump https://example.com 2>&1 | head -5
echo ""

# Step 4: Show how to use via MCP (conceptual)
echo "Step 4: MCP workflow (requires fresh Hermes session)"
echo "  In a new Hermes session, these tools become available:"
echo "    - tui_open()         -> Launch TUIrowser in live pane"
echo "    - tui_send_keys()    -> Send keyboard input"
echo "    - tui_snapshot()     -> Capture screen as text/PNG"
echo "    - tui_wait_for()     -> Wait for expected content"
echo "    - tui_assert()       -> Verify rendered output"
echo "    - tui_close()        -> Close the session"
echo ""

# Step 5: Example test scenario
echo "Step 5: Example test scenario (pseudocode)"
cat << 'EOF'
// Test: TUIrowser can load and display a webpage
let result = tui_open(cols=80, rows=24);
result = tui_send_keys(keys="https://example.com\r");
result = tui_wait_for(text="Example Domain", timeout_ms=5000);
result = tui_assert(contains=["Example Domain"], not_contains=["Loading"]);
result = tui_snapshot(format="text");  // Save for baseline comparison
result = tui_close();

// Test: Verify markdown conversion works
let result = tui_open(cols=80, rows=24);
result = tui_send_keys(keys="https://news.ycombinator.com\r");
result = tui_wait_for(text="[new]", timeout_ms=10000);
result = tui_assert(contains=["points by", "comments"]);
result = tui_close();
EOF
echo ""

echo "=== Demo Complete ==="
echo ""
echo "To use the MCP tools, restart your Hermes session:"
echo "  /stop && /new"