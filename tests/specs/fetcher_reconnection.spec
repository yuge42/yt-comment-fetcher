# YouTube Comment Fetcher Reconnection Tests

This specification tests the automatic reconnection feature of the YouTube Comment Fetcher application.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher starts successfully with reconnect configuration

* Start the fetcher application with reconnect wait time "3" seconds
* Wait for fetcher to connect and receive messages
* Verify received at least "5" JSON messages
* Verify fetcher outputs valid JSON stream
* Stop the fetcher application

## Test fetcher fails fast on initial connection error

* Stop the mock server
* Start the fetcher application and expect failure
* Wait for fetcher to attempt connection
* Verify fetcher exits with connection error
* Verify fetcher does not log reconnection attempts
* Start the mock server

