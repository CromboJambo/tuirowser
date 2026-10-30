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