# TUIrowser

A TUI web browser that simplifies pages to clean markdown, built with Rust and ratatui.

## Philosophy

TUIrowser takes a different approach to web browsing: instead of rendering HTML as-is, it converts web pages to clean, readable markdown text. This makes it ideal for:

- Reading articles without distractions
- Quick information lookup in the terminal
- Scripting and automation workflows
- Accessibility (screen reader friendly)

## Features

- **Clean output**: Converts HTML to simplified markdown, stripping navigation bars, footers, ads, and other non-content elements
- **Keyboard-driven**: Full keyboard navigation with no mouse required
- **Session management**: Persistent bookmarks, history, and preferences stored in SQLite
- **Plugin architecture**: Support for Bitwarden integration and custom plugins
- **Cross-platform**: Works on Linux, macOS, and Windows

## Architecture

TUIrowser is built as a Rust workspace with seven crates:

| Crate | Purpose |
|-------|---------|
| `tuirowser-fetch` | HTTP client with user-agent spoofing, timeout handling, and optional Obscura backend |
| `tuirowser-parser` | HTML parsing and markdown conversion pipeline |
| `tuirowser-ui` | Ratatui-based terminal UI with keyboard navigation |
| `tuirowser-session` | SQLite-backed session management (bookmarks, history) |
| `tuirowser-auth` | Ephemeral token auth server for interactive login flows |
| `tuirowser-core` | Shared types: document model, config, layout helpers |
| `tuirowser-app` | Main application crate that ties everything together |

## Building

```bash
# Clone the repository
git clone https://github.com/CromboJambo/tuirowser.git
cd tuirowser

# Build the release binary (default: no Obscura)
cargo build --release

# Build with Obscura browser backend support
cargo build --release --features tuirowser-fetch/obscura

# Run the browser
./target/release/tuirowser
```

### Obscura Browser Backend

For JavaScript-heavy pages (SPAs, dynamic content), TUIrowser can delegate rendering to an external [Obscura](https://github.com/h4ckf0r0day/obscura) headless browser server. This is optional and feature-gated:

```bash
# 1. Start an Obscura MCP server (separate from tuirowser)
cargo install obscura-cli  # or download from releases
obscura mcp --http --port 8080

# 2. Build tuirowser with Obscura support
cargo build --release --features tuirowser-fetch/obscura
```

TUIrowser connects to Obscura over HTTP JSON-RPC (`http://127.0.0.1:8080/mcp` by default). No compile-time dependency on Obscura's Rust crates — just the external server process. Configure the endpoint in `~/.tuirowser.toml` under `[obscura]`. See [docs/config.md](docs/config.md) for details.

## Usage

### Command-line options

```bash
tuirowser [options] [url]

Options:
  -h, --help       Show help information
  -u, --url <URL>  URL to visit (alternative to positional arg)
  -b, --bookmarks  List saved bookmarks
  -d, --dump <URL> Fetch and print markdown without TUI
  --port <PORT>    Enable HTTP command server on specified port
```

### HTTP Command Server

When started with `--port <port>`, TUIrowser exposes a local HTTP API for programmatic control by agents or scripts:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/navigate?url=<url>` | POST | Navigate to URL |
| `/reload` | POST | Reload current page |
| `/quit` | POST | Exit browser |

Example with curl:
```bash
curl -X POST "http://127.0.0.1:8090/navigate?url=https://news.ycombinator.com"
```

This enables agent-driven automation without needing terminal emulation or tmux session control.

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `g` | Go to top of page |
| `G` | Go to bottom of page |
| `Enter` | Follow link under cursor |
| `b` | Bookmark current page |
| `B` | List bookmarks |
| `h` | View history |
| `/` | Search within page |
| `:` | Enter URL bar mode |
| `q` | Quit |

## Roadmap

- [ ] Bitwarden integration for form autofill
- [x] JavaScript execution (via [Obscura](https://github.com/h4ckf0r0day/obscura) headless browser, `--features tuirowser-fetch/obscura`)
- [ ] Image support (ASCII art or external viewer)
- [ ] Tabbed browsing
- [ ] CSS-aware layout (basic)
- [ ] Plugin system

## License

AGPL-3.0 — See [LICENSE](LICENSE) for details.
---

**Contact**: [projects@crombo.dev](mailto:projects@crombo.dev) · **Security**: [Private vulnerability reporting](https://github.com/CromboJambo/tuirowser/security/advisories/new)
## Discussions

Discussions open when there is enough signal from users to justify the thread. If you have a use case that does not fit an issue, or want to explore a feature direction before committing to it, say so — that is how threads start.