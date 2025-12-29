FROM rust:1.85 AS builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy the entire project for building
COPY . .

# Build the release binary
RUN cargo build --release

FROM node:20-slim

# Gauge version and checksum
ARG GAUGE_VERSION=1.6.22
ARG GAUGE_CHECKSUM=4488b8241e7760af70a0b098a71b47f4922130bcad94f20c85b3c86c2048b8fb

# Install required packages and Gauge via zip installer for reliability
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    unzip \
    netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

# Install Gauge from official zip release with checksum verification
RUN curl -fsSL --max-time 300 https://github.com/getgauge/gauge/releases/download/v${GAUGE_VERSION}/gauge-${GAUGE_VERSION}-linux.x86_64.zip -o /tmp/gauge.zip \
    && echo "${GAUGE_CHECKSUM}  /tmp/gauge.zip" | sha256sum -c - \
    && unzip /tmp/gauge.zip -d /tmp/gauge \
    && mv /tmp/gauge/gauge /usr/local/bin/gauge \
    && chmod +x /usr/local/bin/gauge \
    && rm -rf /tmp/gauge /tmp/gauge.zip

# Install Gauge JavaScript plugin
RUN gauge install js

# Set working directory
WORKDIR /tests

# Copy package files first for better caching
COPY tests/package*.json ./

# Install npm dependencies (if any)
RUN npm install

# Copy the rest of the test files
COPY tests/ .

# Copy the built fetcher binary from builder
COPY --from=builder /build/target/release/yt-comment-fetcher /fetcher/yt-comment-fetcher

# Copy and trust the CA certificate
COPY certs/ca-cert.pem /usr/local/share/ca-certificates/yt-comment-fetcher-ca.crt
RUN update-ca-certificates

# Default command to run tests
CMD ["npm", "test"]
