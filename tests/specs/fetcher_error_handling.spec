# YouTube Comment Fetcher Error Handling Tests

This specification tests error handling in the YouTube Comment Fetcher application.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher requires video ID argument

* Start the fetcher application without video ID argument
* Verify fetcher exits with error about missing argument

## Test fetcher handles invalid video ID

* Start the fetcher application with invalid video ID "non-existent-video"
* Wait for fetcher to attempt connection
* Verify fetcher exits with error about video not found
