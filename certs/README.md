# Development Certificates

This directory contains scripts for generating self-signed certificates for development and testing purposes.

## Generating Certificates

Run the certificate generation script:

```bash
./generate-certs.sh
```

This will create:
- `ca-cert.pem` - Certificate Authority certificate
- `ca-key.pem` - Certificate Authority private key
- `server-cert.pem` - Server certificate
- `server-key.pem` - Server private key

## Important Notes

⚠️ **WARNING**: These certificates are for **DEVELOPMENT/TESTING ONLY**. Do NOT use them in production environments.

The generated certificates are git-ignored for security reasons. Each developer/environment should generate their own certificates.

**Security Note**: The server private key (`server-key.pem`) is set to world-readable permissions (644) to allow Docker containers to read it when mounted as a volume. This is acceptable for development because:
- The certificates are self-signed and only valid for local development
- They are not trusted by any production systems
- The keys are git-ignored and never committed to the repository

In production environments, private keys should be stored securely (e.g., Kubernetes secrets, cloud secret managers) with proper access controls.

## Certificate Details

- The CA certificate is valid for 10 years (3650 days)
- The server certificate is valid for 1 year (365 days)
- The server certificate includes Subject Alternative Names (SANs) for:
  - `yt-api-mock` (Docker service name)
  - `localhost`
  - `127.0.0.1`
  - `::1`

## Trusting the CA

To trust the generated CA certificate on your local system (so browsers and tools don't show certificate warnings):

### Linux (Debian/Ubuntu)
```bash
sudo cp ca-cert.pem /usr/local/share/ca-certificates/yt-comment-fetcher-ca.crt
sudo update-ca-certificates
```

### macOS
```bash
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ca-cert.pem
```

### Docker Containers
The test Docker containers automatically trust the CA certificate by copying it to `/usr/local/share/ca-certificates/` and running `update-ca-certificates`.
