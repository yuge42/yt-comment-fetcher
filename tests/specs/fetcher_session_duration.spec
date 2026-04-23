# YouTube Comment Fetcher Session Duration Tests

This specification tests the maximum session duration feature of the YouTube Comment Fetcher application.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher stops automatically when max session duration is reached

Tags: session-duration

* Start the fetcher with max session duration of "5" seconds
* Wait for fetcher to connect and receive messages
* Verify received at least "1" JSON messages
* Wait for fetcher to exit due to session duration limit
* Verify fetcher logged session duration expiry message
* Verify fetcher exited with code "0"
