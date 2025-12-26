# YouTube Live Comment Fetcher

> **Note**: This documentation is for developers. User-facing documentation will be added in the future.

## Usage

### First-time Setup

Generate development TLS certificates (required for local development):

```bash
cd certs
./generate-certs.sh
cd ..
```

This creates a private Certificate Authority (CA) and server certificates for secure communication. See [certs/README.md](certs/README.md) for more details.

### Running with Docker (Recommended)

The easiest way to run the application is using Docker, which handles certificate trust automatically without needing to install the CA on your host system.

1. **Start the mock server**:
   ```bash
   docker compose up -d
   ```

2. **Build the fetcher image**:
   ```bash
   docker build -t yt-comment-fetcher .
   ```

3. **Run the fetcher**:

   **For local development/testing with the mock server**:
   ```bash
   docker run --rm --network host \
     -e SERVER_ADDRESS=https://localhost:50051 \
     -e REST_API_ADDRESS=https://localhost:8080 \
     yt-comment-fetcher --video-id test-video-1
   ```

   **For production use with official YouTube API**:
   ```bash
   docker run --rm yt-comment-fetcher --video-id YOUR_VIDEO_ID --api-key-path /path/to/api-key.txt
   ```

   If the server requires authentication, provide an API key:
   ```bash
   echo "your-api-key" > api-key.txt
   docker run --rm --network host \
     -e SERVER_ADDRESS=https://localhost:50051 \
     -e REST_API_ADDRESS=https://localhost:8080 \
     -v $(pwd)/api-key.txt:/api-key.txt \
     yt-comment-fetcher --video-id test-video-1 --api-key-path /api-key.txt
   ```

The application will:
1. Fetch the live chat ID from the videos.list endpoint using the provided video ID
2. Connect to the gRPC server and stream comments to stdout as JSON

Press Ctrl+C to stop.

**Cleanup**:
```bash
docker compose down
```

### Running without Docker (Development)

To run the application directly with Cargo, you need to trust the CA certificate at the system level first:

**Linux (Debian/Ubuntu)**:
```bash
sudo cp certs/ca-cert.pem /usr/local/share/ca-certificates/yt-comment-fetcher-ca.crt
sudo update-ca-certificates
```

**macOS**:
```bash
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain certs/ca-cert.pem
```

Then start the mock server and run the application with environment variables to point to the local mock server:

```bash
docker compose up -d
export SERVER_ADDRESS=https://localhost:50051
export REST_API_ADDRESS=https://localhost:8080
cargo run -- --video-id test-video-1
```

If the server requires authentication, provide an API key:

```bash
echo "your-api-key" > api-key.txt
export SERVER_ADDRESS=https://localhost:50051
export REST_API_ADDRESS=https://localhost:8080
cargo run -- --video-id test-video-1 --api-key-path api-key.txt
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