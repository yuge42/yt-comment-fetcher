FROM rust:1.85 AS builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy the entire project for building
COPY . .

# Build the release binary
RUN cargo build --release

FROM debian:bookworm-slim

# Install CA certificates for TLS support
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the built fetcher binary from builder
COPY --from=builder /build/target/release/yt-comment-fetcher /usr/local/bin/yt-comment-fetcher

# Copy the CA certificate from builder and trust it
COPY --from=builder /build/certs/ca-cert.pem /usr/local/share/ca-certificates/yt-comment-fetcher-ca.crt
RUN update-ca-certificates

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/yt-comment-fetcher"]
