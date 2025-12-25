# YouTube Live Comment Fetcher

## Usage

Start the mock server and run the application with a video ID:

```bash
docker compose up -d
cargo run -- --video-id test-video-1
```

The application will:
1. Fetch the live chat ID from the videos.list endpoint using the provided video ID
2. Connect to the gRPC server and stream comments to stdout as JSON

Press Ctrl+C to stop.

## Development

### YouTube API Mock Server

For local development, you can use the YouTube API Mock server:

```bash
docker compose up
```

This will start the mock server with:
- **gRPC server** (live chat) at `localhost:50051`
- **REST server** (videos API) at `localhost:8080`

To stop the server:

```bash
docker compose down
```

### Verifying the Mock Server

You can verify the server is running using `grpcurl` for gRPC endpoints and `curl` for REST endpoints.

**Get video with Live Chat ID (REST):**

```bash
curl "http://localhost:8080/youtube/v3/videos?part=liveStreamingDetails&id=test-video-1"
```

**List available gRPC services:**

```bash
grpcurl -plaintext -import-path ./proto -proto stream_list.proto localhost:50051 list
```

**Stream chat messages (gRPC):**

```bash
grpcurl -plaintext -import-path ./proto -proto stream_list.proto localhost:50051 youtube.api.v3.V3DataLiveChatMessageService/StreamList
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