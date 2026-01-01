# YouTube Comment Fetcher File Output Test

This specification tests the YouTube Comment Fetcher application's ability to write to files and resume from saved state.

**Prerequisites**: The mock server must be running before executing this test.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"
* API key path from environment variable "API_KEY_PATH"

## Test fetcher writes to output file

* Create a temporary output file
* Start the fetcher application with output file
* Wait for fetcher to connect and receive messages
* Verify output file exists
* Verify output file contains valid JSON lines
* Verify each JSON line has kind "youtube#liveChatMessageListResponse"
* Verify file contains at least "3" JSON messages
* Stop the fetcher application
* Clean up temporary output file

## Test fetcher can resume from output file

* Create a temporary output file
* Start the fetcher application with output file
* Wait for fetcher to connect and receive messages
* Verify file contains at least "3" JSON messages
* Stop the fetcher application
* Count messages in output file
* Add "3" new messages via mock control endpoint
* Start the fetcher application with resume flag
* Wait for fetcher to connect and receive messages
* Verify file contains more messages than before
* Verify file contains no duplicate messages
* Stop the fetcher application
* Clean up temporary output file
