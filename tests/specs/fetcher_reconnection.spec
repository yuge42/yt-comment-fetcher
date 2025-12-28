# YouTube Comment Fetcher Reconnection Tests

This specification tests the automatic reconnection feature of the YouTube Comment Fetcher application.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher starts successfully with reconnect configuration and pagination

Tags: reconnection, pagination

* Start the fetcher application with reconnect wait time "3" seconds
* Wait for fetcher to connect and receive messages
* Verify received at least "5" JSON messages
* Verify fetcher outputs valid JSON stream
* Record the current message count
* Wait for stream timeout to occur
* Wait for fetcher to reconnect
* Add new messages via mock control endpoint
* Wait for fetcher to receive new messages
* Verify fetcher received additional messages with correct pagination
* Stop the fetcher application

