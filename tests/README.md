# YouTube Comment Fetcher - E2E Tests

This directory contains end-to-end (E2E) tests for the YouTube Comment Fetcher application using [Gauge](https://gauge.org/) with JavaScript.

## Overview

The tests verify that the YouTube Comment Fetcher application correctly:
1. Connects to the mock YouTube API server
2. Streams live chat messages
3. Outputs valid JSON to stdout
4. Each JSON message contains the expected structure with author details

## Prerequisites

**Important**: Before running tests, you must generate TLS certificates:

```bash
# From the project root
cd certs
./generate-certs.sh
cd ..
```

This creates the CA and server certificates required for the TLS-enabled mock server. Certificates are git-ignored and must be generated in each environment.

## Quick Start with Docker (Recommended)

The easiest way to run tests without installing dependencies locally:

```bash
cd tests
docker compose up --build --abort-on-container-exit
```

Or using the Makefile:

```bash
cd tests
make docker-test
```

## Prerequisites (for local setup)

- Node.js (v14 or later)
- Gauge test framework
- Rust toolchain (for building the fetcher)
- **The mock server must be running** before executing tests

The JavaScript plugin for Gauge should be installed. If not, run:
```bash
gauge install js
```

## Setup (for local development)

1. **Build the fetcher binary:**
```bash
cd ..
cargo build --release
```

2. **Install test dependencies:**
```bash
cd tests
npm install
```

3. **Generate the gRPC client code from proto files:**
```bash
npm run proto:generate
```

Note: The generated proto files are gitignored and will be regenerated automatically when needed.

## Running Tests

### Docker Approach (Recommended)

Run tests using Docker Compose:

```bash
cd tests
docker compose up --build --abort-on-container-exit --exit-code-from tests
```

Or using the Makefile for convenience:

```bash
cd tests
make docker-test
```

This will:
- Build the fetcher binary using the Rust toolchain
- Start the mock YouTube API server with health checks (gRPC on port 50051, REST on port 8080)
- Build the test environment with Gauge and Node.js
- Wait for the server to be healthy (using `nc -z` to verify both ports 50051 and 8080 are listening)
- Run the Gauge E2E tests
- Stop all containers when tests complete
- Exit with the test container's exit code

### Manual Approach

1. **Start the mock server** in a separate terminal:
```bash
# From the project root
docker compose up
```

2. **Build the fetcher** (if not already built):
```bash
cargo build --release
```

3. **Run the tests** in another terminal:
```bash
cd tests
npm test
```

Or run using gauge directly:
```bash
gauge run specs/
```

## Test Structure

- `specs/` - Contains Gauge specification files (`.spec`)
  - `fetcher_stream.spec` - Tests for the fetcher's JSON streaming output
  - `fetcher_error_handling.spec` - Tests for error handling scenarios
  - `fetcher_auth.spec` - Tests for API key authentication
  - `fetcher_reconnection.spec` - Tests for automatic reconnection feature
- `tests/` - Contains step implementation files
  - `step_implementation.js` - JavaScript implementation of test steps
- `proto-gen/` - Generated gRPC client code (auto-generated, gitignored)

## What the Tests Cover

The E2E tests verify:
1. Starting the fetcher application as a subprocess
2. Capturing stdout output from the fetcher
3. Verifying that each line of output is valid JSON
4. Checking that each JSON message has the correct kind field
5. Validating that each message contains items with author details
6. Ensuring a minimum number of messages are received
7. Proper cleanup and shutdown of the fetcher process
8. Error handling for missing arguments and invalid video IDs
9. API key authentication when required
10. Automatic reconnection after connection loss with configurable wait time

## Test Reports

After running tests, HTML reports are generated in:
```
reports/html-report/index.html
```

## Environment Variables

- `SERVER_ADDRESS` - Address of the mock server (default: `localhost:50051`)
- `FETCHER_BINARY` - Path to the fetcher binary (default: `../target/debug/yt-comment-fetcher`)

## Dependencies

- `@grpc/grpc-js` - gRPC client library (for proto generation)
- `@grpc/proto-loader` - Proto file loader
- `grpc-tools` - Tools for code generation from proto files
- `google-protobuf` - Protocol Buffers runtime library

## Cleaning Up

To clean up Docker resources:
```bash
cd tests
make docker-clean
```

To clean local test artifacts:
```bash
cd tests
make clean
```

## Reference

This implementation is based on the E2E test structure from:
https://github.com/yuge42/yt-api-mock/tree/main/tests
