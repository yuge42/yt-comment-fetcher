# YouTube Comment Fetcher Fail-Fast Tests

This specification tests the fail-fast behavior of the YouTube Comment Fetcher application when the server is not available.

**Prerequisites**: The mock server should NOT be running for these tests.

Tags: failfast, no-mock

## Test fetcher fails fast on initial connection error

* Start the fetcher application and expect failure
* Wait for fetcher to attempt connection
* Verify fetcher exits with connection error
* Verify fetcher does not log reconnection attempts
