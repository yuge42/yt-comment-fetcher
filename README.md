# YouTube Live Comment Fetcher

## Usage

### Building the Application

Build the release binary:

```bash
cargo build --release
```

The binary will be available at `target/release/yt-comment-fetcher`.

### Running in Production

The application connects to the official YouTube API by default. You can authenticate using either an API key or OAuth 2.0.

#### Option 1: API Key Authentication

Use an API key for simple, public data access:

```bash
# Create an API key file
echo "YOUR_API_KEY" > api-key.txt

# Run the fetcher
./target/release/yt-comment-fetcher --video-id YOUR_VIDEO_ID --api-key-path api-key.txt
```

#### Option 2: OAuth 2.0 Authentication (Recommended)

OAuth 2.0 is required for accessing private live chats or when API quotas are a concern. The OAuth functionality is separated into two components:
- **yt-oauth-helper**: A helper tool for initial authorization (one-time setup)
- **yt-comment-fetcher**: Automatically refreshes tokens during streaming

**Step 1: Set up OAuth credentials**

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable the YouTube Data API v3
4. Go to "Credentials" > "Create credentials" > "OAuth client ID"
5. Choose application type (Desktop app recommended for this CLI tool)
6. Set authorized redirect URI to: `http://localhost:8080/oauth2callback`
7. Download the client ID and client secret

**Step 2: Obtain OAuth token (first time only)**

Use the helper tool to complete the initial authorization:

```bash
# Build the project first
cargo build --release

# Run the OAuth helper tool
./target/release/yt-oauth-helper \
  --client-id YOUR_CLIENT_ID \
  --client-secret YOUR_CLIENT_SECRET \
  --token-path oauth-token.json
```

The helper tool will:
1. Display an authorization URL in the terminal
2. Start a local callback server on port 8080
3. Wait for you to authorize the application in your browser
4. Exchange the authorization code for tokens
5. Save the tokens to `oauth-token.json` with secure permissions (600)

**Step 3: Stream comments with OAuth**

Once you have the token file, use it with the main fetcher:

```bash
# Run with OAuth token (client credentials required for token refresh)
./target/release/yt-comment-fetcher \
  --video-id YOUR_VIDEO_ID \
  --oauth-token-path oauth-token.json \
  --oauth-client-id YOUR_CLIENT_ID \
  --oauth-client-secret YOUR_CLIENT_SECRET
```

The fetcher will:
- Load the token from the file
- Automatically refresh the access token when expired
- Update the token file with the refreshed token
- Stream comments continuously

**OAuth Token File Format:**

The token file is stored as JSON with secure permissions (owner read/write only):

```json
{
  "access_token": "ya29.xxx...",
  "refresh_token": "1//xxx...",
  "token_type": "Bearer",
  "expires_at": 1234567890
}
```

**Note:** 
- API key and OAuth are mutually exclusive - use one or the other
- The access token expires after ~1 hour but is automatically refreshed by the fetcher
- The refresh token is long-lived and persists in the token file
- Client credentials are required for token refresh - always provide them when using OAuth
- Keep your token file secure - it grants access to your YouTube account
- Use `yt-oauth-helper` only once to obtain the initial token; the fetcher handles all subsequent refreshes

The application will:
1. Fetch the live chat ID from the videos.list endpoint using the provided video ID
2. Connect to the gRPC server and stream comments to stdout as JSON
3. Automatically reconnect if the stream times out during message reception (default: wait 5 seconds between attempts)

### Saving Comments to a File

You can save comments directly to a file using the `--output-file` option:

```bash
# With API key
./target/release/yt-comment-fetcher \
  --video-id YOUR_VIDEO_ID \
  --api-key-path api-key.txt \
  --output-file comments.json

# With OAuth
./target/release/yt-comment-fetcher \
  --video-id YOUR_VIDEO_ID \
  --oauth-token-path oauth-token.json \
  --oauth-client-id YOUR_CLIENT_ID \
  --oauth-client-secret YOUR_CLIENT_SECRET \
  --output-file comments.json
```

### Resuming from a Saved File

If the fetcher is interrupted, you can resume from where it left off using the `--resume` flag:

```bash
# Resume with API key
./target/release/yt-comment-fetcher \
  --output-file comments.json \
  --resume \
  --api-key-path api-key.txt

# Resume with OAuth
./target/release/yt-comment-fetcher \
  --output-file comments.json \
  --resume \
  --oauth-token-path oauth-token.json \
  --oauth-client-id YOUR_CLIENT_ID \
  --oauth-client-secret YOUR_CLIENT_SECRET
```

The `--resume` flag:
- Reads the last line from the output file
- Extracts the chat ID and pagination token
- Continues streaming from where it left off
- `--video-id` becomes optional when using `--resume`, but can be provided as a fallback if the chat ID cannot be extracted from the file

**Note:** When using `--resume`, the `--output-file` must be specified, but `--video-id` is optional.

**Reconnection:** If the gRPC stream times out or is lost during message reception, the fetcher will automatically attempt to reconnect. Initial connection failures will cause the application to exit immediately (fail-fast behavior appropriate for CLI tools). You can configure the wait time between reconnection attempts:

```bash
# Set reconnection wait time to 10 seconds
./target/release/yt-comment-fetcher --video-id YOUR_VIDEO_ID --api-key-path api-key.txt --reconnect-wait-secs 10
```

Press Ctrl+C to stop.

### Viewing Comments with the Viewer Script

The `viewer.sh` script formats JSON output into a readable colored format. It uses `jq` to extract the author name and message text.

**Requirements:** The viewer script requires `jq` to be installed. Install it with:
- Ubuntu/Debian: `sudo apt-get install jq`
- macOS: `brew install jq`
- Or see: https://jqlang.github.io/jq/download/

**Option 1: Using built-in file output**

The easiest way is to use the built-in `--output-file` option:

```bash
# Fetch and save comments to file
./target/release/yt-comment-fetcher \
  --video-id YOUR_VIDEO_ID \
  --api-key-path api-key.txt \
  --output-file comments.json

# In another terminal, view in real-time
tail -F comments.json | ./viewer.sh
```

**Option 2: Stream and view in real-time while saving to file**

```bash
TARGET_FILE=$HOME/yt-comments/$(date +%Y%m%d_%H%M%S).ndjson
stdbuf -oL ./target/release/yt-comment-fetcher --video-id YOUR_VIDEO_ID --api-key-path api-key.txt \
| tee $TARGET_FILE \
| stdbuf -oL ./viewer.sh
```

**Option 3: Save to file and view separately**

```bash
# Terminal 1: Fetch and save comments
TARGET_FILE=$HOME/yt-comments/$(date +%Y%m%d_%H%M%S).ndjson
./target/release/yt-comment-fetcher --video-id YOUR_VIDEO_ID --api-key-path api-key.txt >> $TARGET_FILE

# Terminal 2 (or add & to previous command): View comments in real-time
latest=$(ls -t "$HOME/yt-comments/"*.ndjson | head -n 1)
stdbuf -oL tail -F $latest \
| stdbuf -oL ./viewer.sh
```

## Development Setup

> **Note**: The following sections are for developers working on this project.

### First-time Setup

Generate development TLS certificates (required for local development):

```bash
cd certs
./generate-certs.sh
cd ..
```

This creates a private Certificate Authority (CA) and server certificates for secure communication. See [certs/README.md](certs/README.md) for more details.

### Running with Docker (Development)

Docker is recommended for development as it handles certificate trust automatically.

1. **Start the mock server**:
   ```bash
   docker compose up -d
   ```

2. **Build the fetcher image**:
   ```bash
   docker build -t yt-comment-fetcher .
   ```

3. **Run with the mock server**:
   ```bash
   docker run --rm --network host \
     -e SERVER_ADDRESS=https://localhost:50051 \
     -e REST_API_ADDRESS=https://localhost:8080 \
     yt-comment-fetcher --video-id test-video-1
   ```

**Cleanup**:
```bash
docker compose down
```

### Running with Cargo (Development)

To run the application directly with Cargo against the mock server:

1. Trust the CA certificate at the system level:

   **Linux (Debian/Ubuntu)**:
   ```bash
   sudo cp certs/ca-cert.pem /usr/local/share/ca-certificates/yt-comment-fetcher-ca.crt
   sudo update-ca-certificates
   ```

   **macOS**:
   ```bash
   sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain certs/ca-cert.pem
   ```

2. Start the mock server and run:

   ```bash
   docker compose up -d
   export SERVER_ADDRESS=https://localhost:50051
   export REST_API_ADDRESS=https://localhost:8080
   cargo run -- --video-id test-video-1
   ```

## Development

### Server Address Configuration

The application defaults to the **official YouTube API endpoints**:
- **REST API** (videos.list): `https://www.googleapis.com`
- **gRPC API** (live chat streaming): `https://youtube.googleapis.com`

For local development and testing with the mock server, override these defaults using environment variables:
```bash
export SERVER_ADDRESS=https://localhost:50051
export REST_API_ADDRESS=https://localhost:8080
```

### YouTube API Mock Server

For local development, you can use the YouTube API Mock server with TLS enabled:

```bash
docker compose up
```

This will start the mock server with:
- **gRPC server** (live chat) at `https://localhost:50051` (TLS enabled)
- **REST server** (videos API) at `https://localhost:8080` (TLS enabled)

To stop the server:

```bash
docker compose down
```

### TLS Certificate Setup

The mock server uses TLS for all connections. Development certificates are stored in the `certs/` directory and are git-ignored for security.

To generate certificates:
```bash
cd certs && ./generate-certs.sh
```

The application is configured to use HTTPS/TLS by default when connecting to both the official YouTube API and the local mock server.

### Authentication

The fetcher supports API key authentication for servers that require it:

```bash
# Create an API key file
echo "your-api-key-here" > api-key.txt

# Run with API key
cargo run -- --video-id test-video-1 --api-key-path api-key.txt
```

The API key is:
- Sent as a `key` query parameter for REST API requests
- Sent as `x-goog-api-key` metadata for gRPC streaming requests

This matches the authentication pattern used by the real YouTube Data API.

### Verifying the Mock Server

You can verify the server is running using `grpcurl` for gRPC endpoints and `curl` for REST endpoints.

**Get video with Live Chat ID (REST):**

```bash
curl --cacert certs/ca-cert.pem "https://localhost:8080/youtube/v3/videos?part=liveStreamingDetails&id=test-video-1"
```

**List available gRPC services:**

```bash
grpcurl -cacert certs/ca-cert.pem -import-path ./proto -proto stream_list.proto localhost:50051 list
```

**Stream chat messages (gRPC):**

```bash
grpcurl -cacert certs/ca-cert.pem -import-path ./proto -proto stream_list.proto localhost:50051 youtube.api.v3.V3DataLiveChatMessageService/StreamList
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Some external dependencies may carry additional copyright notices and license terms.
When building and distributing binaries, those external library licenses may be included.

### Proto Definitions

This project uses proto definitions from [yt-api-proto](https://github.com/yuge42/yt-api-proto), 
which is licensed under the Apache License, Version 2.0. Binaries distributed from this project 
will contain work derived from these proto definitions.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.