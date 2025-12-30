# YouTube Comment Fetcher Shutdown Test

This specification tests the shutdown behavior of the YouTube Comment Fetcher when receiving SIGINT or SIGTERM signals.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher handles SIGINT

* Start the fetcher application
* Wait for fetcher to connect and receive messages
* Verify fetcher outputs valid JSON stream
* Send SIGINT signal to fetcher
* Wait for fetcher to exit gracefully
* Verify fetcher logged shutdown message
* Verify fetcher exited with code "0"

## Test fetcher handles SIGTERM

* Start the fetcher application
* Wait for fetcher to connect and receive messages
* Verify fetcher outputs valid JSON stream
* Send SIGTERM signal to fetcher
* Wait for fetcher to exit gracefully
* Verify fetcher logged shutdown message
* Verify fetcher exited with code "0"
