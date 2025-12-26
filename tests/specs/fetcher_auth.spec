# YouTube Comment Fetcher Authentication Tests

This specification tests API key authentication in the YouTube Comment Fetcher application.

**Prerequisites**: The mock server must be running with authentication enabled (REQUIRE_AUTH=true).

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"

## Test fetcher fails without API key when auth is required

* Start the fetcher application without API key
* Wait for fetcher to attempt connection
* Verify fetcher exits with authentication error
