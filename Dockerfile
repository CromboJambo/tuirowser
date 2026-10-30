FROM rust:1.85-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

# Final stage - minimal runtime image
FROM debian:bookworm-slim

# Install dependencies for the binary
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/tuirowser /usr/local/bin/tuirowser

EXPOSE 8081

CMD ["tuirowser", "--port", "8081"]