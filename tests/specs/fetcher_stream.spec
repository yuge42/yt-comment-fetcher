# YouTube Comment Fetcher E2E Test

This specification tests the YouTube Comment Fetcher application that connects to the mock server and outputs JSON stream.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher prints expected JSON stream

* Start the fetcher application
* Wait for fetcher to connect and receive messages
* Verify fetcher outputs valid JSON stream
* Verify each JSON line has kind "youtube#liveChatMessageListResponse"
* Verify each JSON line has author details in items
* Verify received at least "5" JSON messages
* Stop the fetcher application
