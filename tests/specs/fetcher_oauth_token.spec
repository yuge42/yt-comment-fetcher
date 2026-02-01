# YouTube Comment Fetcher OAuth Token Authentication Tests

This specification tests OAuth 2.0 token authentication and refresh functionality using the mock server's pseudo token generation feature.

**Prerequisites**: The mock server (v0.3.0+) must be running with `REQUIRE_AUTH=true` to enable OAuth token validation.

* Server address from environment variable "SERVER_ADDRESS" or default "https://localhost:8080"
* Mock server has OAuth2 token generation endpoint at `/oauth2/token`
* Mock server v0.3.0+ rejects tokens starting with `invalid-` prefix

## Test OAuth token authentication with valid token

This test verifies that the fetcher can authenticate using a valid OAuth token.

* Generate a valid OAuth token from the mock server
* Create a token file with the generated token
* Start the fetcher with OAuth token authentication
* Wait for fetcher to connect and stream messages
* Verify fetcher receives live chat messages
* Stop the fetcher

## Test OAuth token refresh when token expires

This test verifies that the fetcher automatically refreshes an expired OAuth token.

* Generate an OAuth token with short expiry from the mock server
* Create a token file with the token
* Start the fetcher with OAuth token and client credentials
* Wait for initial connection
* Wait for token to expire
* Verify fetcher automatically refreshes the token
* Verify fetcher continues streaming messages after refresh
* Verify token file is updated with new token
* Stop the fetcher

## Test OAuth authentication fails with expired token and no client credentials

This test verifies that the fetcher fails appropriately when the token is expired and no client credentials are provided for refresh.

* Generate an expired OAuth token from the mock server
* Create a token file with the expired token
* Start the fetcher with OAuth token but without client credentials
* Wait for fetcher to attempt connection
* Verify fetcher fails with token refresh error
* Verify appropriate error message is displayed

## Test OAuth token validation with invalid token

This test verifies that the fetcher handles authentication failures with invalid tokens (tokens with 'invalid-' prefix are rejected by mock server v0.3.0+).

* Create a token file with an invalid token starting with invalid- prefix
* Start the fetcher with the invalid OAuth token
* Wait for fetcher to attempt connection
* Verify fetcher fails with authentication error
* Verify appropriate error message is displayed
