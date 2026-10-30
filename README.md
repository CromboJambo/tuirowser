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

TUIrowser is built as a Rust workspace with five crates:

| Crate | Purpose |
|-------|---------|
| `tuirowser-fetch` | HTTP client with user-agent spoofing and timeout handling |
| `tuirowser-parser` | HTML parsing and markdown conversion pipeline |
| `tuirowser-ui` | Ratatui-based terminal UI with keyboard navigation |
| `tuirowser-session` | SQLite-backed session management (bookmarks, history) |
| `tuirowser-app` | Main application crate that ties everything together |

## Building

```bash
# Clone the repository
git clone https://github.com/your-username/tuirowser.git
cd tuirowser

# Build the release binary
cargo build --release

# Run the browser
./target/release/tuirowser
```

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
- [ ] JavaScript execution (via headless browser)
- [ ] Image support (ASCII art or external viewer)
- [ ] Tabbed browsing
- [ ] CSS-aware layout (basic)
- [ ] Plugin system

## License

AGPL-3.0 — See [LICENSE](LICENSE) for details.