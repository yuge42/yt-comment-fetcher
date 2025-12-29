# Copilot Instructions for yt-comment-fetcher

## Project Overview

This is a **YouTube Live Comment Fetcher** written in Rust that streams live chat messages from YouTube live streams. The application connects to the YouTube Data API (both REST and gRPC) and outputs JSON-formatted comments to stdout.

**Key Technologies:**
- **Language:** Rust (edition 2024, minimum version 1.85)
- **Build System:** Cargo workspace
- **gRPC:** Tonic v0.12 with Protocol Buffers (prost v0.13)
- **Async Runtime:** Tokio (with macros and rt-multi-thread features)
- **HTTP Client:** reqwest v0.12 with rustls-tls
- **CLI:** clap v4.5 with derive feature
- **Testing:** Gauge E2E tests (JavaScript-based)

## Repository Structure

```
.
├── src/                    # Main application code
│   └── main.rs            # Entry point with streaming logic
├── crates/                # Workspace members
│   ├── yt-grpc-client/    # gRPC client library for YouTube API
│   └── example/           # Example crate
├── proto/                 # Git submodule with proto definitions
├── certs/                 # TLS certificates for development (git-ignored)
├── tests/                 # E2E tests using Gauge framework
├── docker-compose.yml     # Mock server setup for development
└── viewer.sh             # Helper script to format JSON output
```

## Build and Development Setup

### Prerequisites

**Critical:** Before any build or test operation:

1. **Initialize git submodules:**
   ```bash
   git submodule update --init --recursive
   ```
   The `proto/` directory is a submodule containing Protocol Buffer definitions.

2. **Install protobuf compiler:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install -y protobuf-compiler
   
   # macOS
   brew install protobuf
   ```
   Required for building the `yt-grpc-client` crate.

3. **Generate TLS certificates (for development/testing):**
   ```bash
   cd certs
   ./generate-certs.sh
   cd ..
   ```
   Required for running the mock server and E2E tests. Certificates are git-ignored.

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# The binary will be at target/release/yt-comment-fetcher
```

### Linting and Formatting

The CI pipeline enforces strict code quality standards:

```bash
# Check formatting (must pass)
cargo fmt --all -- --check

# Auto-format code
cargo fmt --all

# Run clippy with warnings as errors
cargo clippy --all-targets --all-features -- -D warnings
```

**Important:** All clippy warnings are treated as errors in CI. Fix all warnings before committing.

### Testing

**Unit Tests:**
```bash
cargo test --verbose
```

**E2E Tests with Docker (Recommended):**
```bash
cd tests
docker compose up --build --abort-on-container-exit --exit-code-from tests
```

**E2E Tests Manually:**
1. Start mock server: `docker compose up -d`
2. Build fetcher: `cargo build --release`
3. Run tests: `cd tests && npm test`

The E2E tests use:
- Gauge framework with JavaScript
- Mock YouTube API server (https://github.com/yuge42/yt-api-mock)
- Tests for streaming, error handling, authentication, and reconnection

## Code Conventions and Style

### Rust Best Practices

1. **Edition 2024:** Use the latest Rust edition features where appropriate.

2. **Error Handling:**
   - Use `Result<T, Box<dyn std::error::Error>>` for functions that can fail
   - Provide descriptive error messages with context
   - Use `?` operator for error propagation
   - Fail-fast on initial connection errors (appropriate for CLI tools)

3. **Async Code:**
   - Use `#[tokio::main]` for the main function
   - Use `async fn` for async functions
   - Use `tokio_stream::StreamExt` for stream operations

4. **CLI Arguments:**
   - Use `clap` with derive macros
   - All flags should use `--long-form` (e.g., `--video-id`, not `-v`)
   - Required arguments should be marked with `required = true`
   - Provide helpful descriptions for all arguments

5. **Serialization:**
   - Use `serde` for JSON serialization/deserialization
   - Auto-derive traits in proto definitions via `tonic_build` configuration
   - Output JSON to stdout using `println!("{}", serde_json::to_string(&message)?)`

### Project-Specific Patterns

1. **Reconnection Logic:**
   - Initial connection failures cause immediate exit (fail-fast)
   - Stream errors during operation trigger automatic reconnection
   - Use pagination tokens (`next_page_token`) to resume from where stream dropped
   - Configurable wait time between reconnection attempts
   - Log all reconnection attempts to stderr

2. **Logging:**
   - User-facing messages go to **stderr** using `eprintln!()`
   - JSON data goes to **stdout** using `println!()`
   - This allows piping JSON to files while seeing status messages

3. **Environment Variables:**
   - `SERVER_ADDRESS`: gRPC server URL (default: `https://youtube.googleapis.com`)
   - `REST_API_ADDRESS`: REST API URL (default: `https://www.googleapis.com`)
   - Always include `https://` prefix for TLS connections

4. **API Authentication:**
   - API keys read from file path (not directly from command line for security)
   - REST API: send as `key` query parameter
   - gRPC: send as `x-goog-api-key` metadata header

5. **TLS/HTTPS:**
   - All connections use HTTPS/TLS (both production and development)
   - Development uses self-signed certificates
   - Use `rustls-tls-native-roots` for certificate validation

## Protocol Buffers and gRPC

### Proto Definitions

- Located in `proto/` submodule (from https://github.com/yuge42/yt-api-proto)
- Licensed under Apache License 2.0
- Build script: `crates/yt-grpc-client/build.rs`
- Auto-generated code includes serde traits: `#[derive(serde::Serialize, serde::Deserialize)]`

### gRPC Client Usage

```rust
// Connect to server
let mut client = YouTubeClient::connect(server_url, api_key).await?;

// Stream comments with optional pagination
let stream = client.stream_comments(Some(chat_id), page_token).await?;

// Process stream
while let Some(result) = stream.next().await {
    match result {
        Ok(message) => { /* handle message */ }
        Err(e) => { /* handle error/reconnect */ }
    }
}
```

## Docker and Mock Server

### Development with Mock Server

The project includes a docker-compose setup with the YouTube API Mock Server:

**Services:**
- `yt-api-mock`: Mock server on ports 50051 (gRPC) and 8080 (REST)
- TLS enabled with certificates from `certs/` directory
- Environment variables:
  - `TLS_CERT_PATH=/certs/server-cert.pem`
  - `TLS_KEY_PATH=/certs/server-key.pem`
  - `CHAT_STREAM_TIMEOUT=5`

**Usage:**
```bash
# Start mock server
docker compose up -d

# Run fetcher against mock server
export SERVER_ADDRESS=https://localhost:50051
export REST_API_ADDRESS=https://localhost:8080
cargo run -- --video-id test-video-1
```

### Production Dockerfile

Multi-stage build:
1. Builder stage: Rust 1.85, installs protobuf-compiler, builds release binary
2. Runtime stage: Debian bookworm-slim, installs ca-certificates, copies binary and CA cert

## Testing Strategy

### CI Pipeline (`.github/workflows/ci.yml`)

Three jobs:
1. **test**: Format check, clippy, build, unit tests
2. **e2e-test**: E2E tests with mock server
3. **e2e-test-failfast**: E2E tests for fail-fast behavior (no server)

All jobs:
- Use Ubuntu latest
- Require submodule checkout: `submodules: recursive`
- Install protobuf-compiler
- Generate TLS certificates before E2E tests

### Test Files

E2E test specs in `tests/specs/`:
- `fetcher_stream.spec`: Basic streaming functionality
- `fetcher_error_handling.spec`: Missing arguments, invalid video ID
- `fetcher_auth.spec`: API key authentication
- `fetcher_reconnection.spec`: Automatic reconnection with pagination
- `fetcher_failfast.spec`: Initial connection failures

## Common Tasks and Commands

### Adding New Dependencies

1. Add to workspace dependencies in root `Cargo.toml` if shared across crates
2. Reference with `workspace = true` in specific crate `Cargo.toml`
3. Use specific features only when needed (e.g., `features = ["tls", "tls-roots"]`)

### Updating Proto Definitions

1. Update submodule: `cd proto && git pull origin main && cd ..`
2. Commit submodule update: `git add proto && git commit -m "Update proto definitions"`
3. Rebuild: `cargo build` (build script will regenerate code)

### Debugging Connection Issues

1. Verify certificates exist: `ls -la certs/`
2. Check mock server health: `curl --cacert certs/ca-cert.pem https://localhost:8080/health`
3. Test gRPC connection: `grpcurl -cacert certs/ca-cert.pem -import-path ./proto -proto stream_list.proto localhost:50051 list`
4. Enable verbose logging in environment

## Security Considerations

1. **API Keys:**
   - Never commit API keys to the repository
   - Git-ignored patterns: `apikey`, `apikey.txt`, `key`, `key.txt`, `api-key`, `api-key.txt`
   - Read from file path, not command line arguments

2. **Certificates:**
   - Development certificates in `certs/` are git-ignored
   - Server private key has 644 permissions (world-readable for Docker)
   - This is acceptable only because they're self-signed dev certs
   - Production should use proper secret management

3. **Dependencies:**
   - Use `rustls-tls` (not OpenSSL) for better security and portability
   - Use `rustls-tls-native-roots` to trust system certificate store

## Common Errors and Solutions

### Error: "protoc not found"
**Solution:** Install protobuf-compiler:
```bash
sudo apt-get install -y protobuf-compiler
```

### Error: "No such file or directory: ../../proto/stream_list.proto"
**Solution:** Initialize submodules:
```bash
git submodule update --init --recursive
```

### Error: "certificate verify failed" when running against mock server
**Solution:** Generate certificates:
```bash
cd certs && ./generate-certs.sh && cd ..
```

### E2E tests fail with "connection refused"
**Solution:** Ensure mock server is running:
```bash
docker compose up -d
# Wait for health check
sleep 5
```

### Clippy warnings in CI
**Solution:** Fix all warnings locally before pushing:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Workspace Structure

The project uses a Cargo workspace with shared dependencies:

- **Root crate:** `yt-comment-fetcher` (main application)
- **Member crates:**
  - `yt-grpc-client`: Reusable gRPC client library
  - `example`: Example/demo crate

Shared workspace configuration:
- Edition: 2024
- Rust version: 1.85
- License: MIT OR Apache-2.0
- All metadata centralized in workspace

## Helper Scripts

### `viewer.sh`
Formats JSON output for human reading using `jq`:
- Requires `jq` to be installed
- Extracts author name and message text
- Applies ANSI colors for readability
- Usage: `./yt-comment-fetcher --video-id ID | ./viewer.sh`

### `certs/generate-certs.sh`
Generates self-signed TLS certificates for development:
- Creates CA certificate (10 years validity)
- Creates server certificate (1 year validity)
- Includes SANs for `yt-api-mock`, `localhost`, `127.0.0.1`, `::1`
- **Must be run before starting mock server or E2E tests**

## Making Changes

### General Guidelines

1. **Minimal changes:** Only modify what's necessary to fix the issue
2. **Follow existing patterns:** Match the style and structure of existing code
3. **Test thoroughly:** Run unit tests, E2E tests, and manual verification
4. **Format and lint:** Always run `cargo fmt` and `cargo clippy` before committing
5. **Update documentation:** Keep README and comments in sync with code changes

### Before Committing

Run this checklist:
```bash
# 1. Format code
cargo fmt --all

# 2. Check for clippy warnings
cargo clippy --all-targets --all-features -- -D warnings

# 3. Build successfully
cargo build

# 4. Run unit tests
cargo test --verbose

# 5. (Optional) Run E2E tests
cd tests && docker compose up --build --abort-on-container-exit
```

### Git Ignore Patterns

Notable ignored paths:
- `target/` - Build artifacts
- `certs/*.pem`, `certs/*.key` - TLS certificates
- `api*.txt`, `key*.txt` - API key files
- `tests/proto-gen/` - Generated proto code
- `tests/reports/` - Test reports
- `**/*.rs.bk` - Rustfmt backups
- `**/mutants.out*/` - Mutation testing data

## Additional Resources

- **Proto definitions:** https://github.com/yuge42/yt-api-proto
- **Mock server:** https://github.com/yuge42/yt-api-mock
- **Tonic documentation:** https://docs.rs/tonic/
- **Clap documentation:** https://docs.rs/clap/
- **Gauge framework:** https://gauge.org/

## Notes for AI Coding Agents

1. **Always initialize submodules first** - This is the most common build failure cause
2. **Install protobuf-compiler** - Required for build, not a Rust dependency
3. **Generate certificates for testing** - E2E tests will fail without them
4. **Stderr vs Stdout** - Keep logging on stderr, JSON on stdout
5. **Fail-fast pattern** - Initial connections fail immediately, stream errors reconnect
6. **Workspace dependencies** - Use workspace-level dependencies for consistency
7. **TLS is required** - All connections (prod and dev) use HTTPS/TLS
8. **API key security** - Never expose keys in command line or code, use file paths
9. **Clippy is strict** - All warnings are errors, fix them before committing
10. **Test with mock server** - Always verify changes against the mock server setup
