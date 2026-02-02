# YouTube Comment Fetcher OAuth Tests

This specification tests OAuth 2.0 authentication in the YouTube Comment Fetcher application.

**Prerequisites**: The mock server should support OAuth token authentication.

* Server address from environment variable "SERVER_ADDRESS" or default "localhost:50051"

## Test fetcher rejects both API key and OAuth token

When both --api-key-path and --oauth-token-path are provided, the application should reject the configuration as they are mutually exclusive.

* Start the fetcher application with both API key and OAuth token
* Wait for fetcher to exit
* Verify fetcher exits with mutual exclusivity error

## Test fetcher requires OAuth client credentials for first-time auth

When --oauth-token-path is provided but the token file doesn't exist, client ID and secret must be provided.

* Start the fetcher application with OAuth token path but no client credentials
* Wait for fetcher to exit
* Verify fetcher exits with missing client ID error
