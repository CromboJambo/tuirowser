# Minimal TUIrowser container with HTTP command server
# Build: podman build -t tuirowser .
# Run:   podman run -d --name browser -p 8090:8090 tuirowser

FROM rust:1.90 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM ubuntu:24.04 AS target
RUN apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/tuirowser /usr/local/bin/tuirowser

# Expose HTTP command server port
EXPOSE 8090

# Default: start with Hacker News, HTTP server on all interfaces
CMD ["tuirowser", "https://news.ycombinator.com", "--port", "8090"]