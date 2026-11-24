# TUIrowser Configuration File Format

Create a `.tuirowser.toml` in your home directory to customize behavior.

## Example Configuration

```toml
[fetch]
user_agent = "Mozilla/5.0 (compatible; TUIrowser/1.0)"
timeout_secs = 30
max_redirects = 10

[search]
backend = "searxng"
url = "http://localhost:8080"  # Your SearXNG instance URL

[obscura]
enabled = true
endpoint = "http://127.0.0.1:8080/mcp"  # Obscura MCP HTTP server
timeout_secs = 30

[ui]
theme = "dark"  # "dark" or "light"
```

## Search Configuration

TUIrowser integrates with [SearXNG](https://docs.searxng.org/) for web search functionality.

### Self-hosted SearXNG (recommended)

Deploy your own SearXNG instance for privacy and reliability:

```bash
# Using Docker
docker run -d --name searxng -p 8080:8080 searxng/searxng

# Then configure in ~/.tuirowser.toml
[search]
url = "http://localhost:8080"
```

### Public Instances

Some public SearXNG instances support the JSON API. Check https://searx.space/ for current instances. Note that public instances may be rate-limited or unavailable.

## Command Line Search

Use `--search` to perform a search from the command line:

```bash
tuirowser --search "Rust programming language"
```

This outputs formatted search results directly to stdout, useful for scripting and piping.

## Obscura Browser Backend

The `[obscura]` section configures the optional [Obscura](https://github.com/h4ckf0r0day/obscura) headless browser backend. This requires building TUIrowser with `--features tuirowser-fetch/obscura` and running an external Obscura MCP server.

### Starting the Obscura Server

```bash
# Install Obscura
cargo install obscura-cli
# Or download a prebuilt binary from https://github.com/h4ckf0r0day/obscura/releases

# Start the MCP HTTP server
obscura mcp --http --port 8080

# With stealth mode (anti-fingerprinting + tracker blocking)
obscura mcp --http --port 8080 --stealth
```

### Configuration Options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `false` | Enable Obscura backend when available |
| `endpoint` | string | `http://127.0.0.1:8080/mcp` | URL of the Obscura MCP HTTP server |
| `timeout_secs` | int | `30` | Request timeout for Obscura operations |

### When to Use Obscura

- **JavaScript-heavy pages** (SPAs like GitHub issues, Jira, modern dashboards)
- **Dynamic content** that requires JS execution to render
- **Anti-bot protected sites** (with `--stealth` mode enabled)

For static pages and articles, TUIrowser's native reqwest pipeline is faster and doesn't require an external server. The Obscura backend is a fallback for when the native path isn't sufficient.