# YouTube Comment Fetcher Reconnection Tests

This specification tests the automatic reconnection feature of the YouTube Comment Fetcher application.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher reconnects after server becomes unavailable

* Start the fetcher application with reconnect wait time "3" seconds
* Wait for fetcher to connect and receive messages
* Verify received at least "3" JSON messages
* Stop the mock server
* Wait "5" seconds for connection to drop
* Verify fetcher logs connection error
* Verify fetcher logs reconnection attempt
* Start the mock server
* Wait for fetcher to reconnect and receive messages
* Verify received new JSON messages after reconnection
* Stop the fetcher application

## Test fetcher uses configured reconnect wait time

* Start the fetcher application with reconnect wait time "2" seconds
* Wait for fetcher to connect and receive messages
* Verify received at least "3" JSON messages
* Stop the mock server
* Wait "3" seconds for connection to drop
* Verify fetcher logs connection error
* Verify reconnect wait time is "2" seconds in logs
* Stop the fetcher application
