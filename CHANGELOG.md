# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- HTTP command server for agent-driven browser control (`--port` flag)
- Podman container image for durable browser-as-a-service deployment
- `run_app_with_commands()` async API for channel-based command processing

### Changed
- Updated to Rust 2024 edition
- Changed license from MIT to AGPL-3.0
- Improved documentation across all crates

### Fixed
- Container networking issues (now uses host network mode)
- GLIBC compatibility (uses Debian bookworm-slim base image)

## [0.1.0] - 2024-10-29

### Added
- TUI markdown browser with ratatui frontend
- HTML to markdown conversion pipeline
- HTTP fetching with redirect and timeout support
- Modal input handling (Normal vs URL input modes)
- Keyboard shortcuts: `g` (go), `r` (reload), `q` (quit)
- Container build files (Dockerfile, podman-compose.yaml)
