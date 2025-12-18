FROM rust:1.85 AS builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy the entire project for building
COPY . .

# Build the release binary
RUN cargo build --release

FROM node:20-slim

# Install required packages including Gauge and netcat for healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    unzip \
    netcat-openbsd \
    && curl -SsL https://downloads.gauge.org/stable | sh \
    && rm -rf /var/lib/apt/lists/*

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

# Default command to run tests
CMD ["npm", "test"]
