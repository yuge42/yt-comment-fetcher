#!/bin/bash

# Script to generate a private CA and server certificate for development/testing
# This creates self-signed certificates that should NOT be used in production

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERTS_DIR="${SCRIPT_DIR}"

# Certificate configuration
CA_SUBJECT="/C=US/ST=Dev/L=Dev/O=YT Comment Fetcher Dev/CN=YT Comment Fetcher Dev CA"
SERVER_SUBJECT="/C=US/ST=Dev/L=Dev/O=YT Comment Fetcher Dev/CN=yt-api-mock"

echo "Generating private CA and certificates for development/testing..."
echo "Output directory: ${CERTS_DIR}"

# Generate CA private key
echo "Generating CA private key..."
openssl genrsa -out "${CERTS_DIR}/ca-key.pem" 4096

# Generate CA certificate
echo "Generating CA certificate..."
openssl req -new -x509 -days 3650 -key "${CERTS_DIR}/ca-key.pem" \
    -out "${CERTS_DIR}/ca-cert.pem" \
    -subj "${CA_SUBJECT}"

# Generate server private key
echo "Generating server private key..."
openssl genrsa -out "${CERTS_DIR}/server-key.pem" 4096

# Generate server certificate signing request (CSR)
echo "Generating server CSR..."
openssl req -new -key "${CERTS_DIR}/server-key.pem" \
    -out "${CERTS_DIR}/server-csr.pem" \
    -subj "${SERVER_SUBJECT}"

# Create server certificate extensions file
cat > "${CERTS_DIR}/server-ext.cnf" <<EOF
basicConstraints = CA:FALSE
nsCertType = server
nsComment = "YT Comment Fetcher Dev Server Certificate"
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer:always
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = yt-api-mock
DNS.2 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# Sign the server certificate with the CA
echo "Signing server certificate with CA..."
openssl x509 -req -days 365 \
    -in "${CERTS_DIR}/server-csr.pem" \
    -CA "${CERTS_DIR}/ca-cert.pem" \
    -CAkey "${CERTS_DIR}/ca-key.pem" \
    -CAcreateserial \
    -out "${CERTS_DIR}/server-cert.pem" \
    -extfile "${CERTS_DIR}/server-ext.cnf"

# Clean up temporary files
rm -f "${CERTS_DIR}/server-csr.pem" "${CERTS_DIR}/server-ext.cnf" "${CERTS_DIR}/ca-cert.srl"

# Set appropriate permissions
chmod 600 "${CERTS_DIR}/ca-key.pem" "${CERTS_DIR}/server-key.pem"
chmod 644 "${CERTS_DIR}/ca-cert.pem" "${CERTS_DIR}/server-cert.pem"

echo ""
echo "Certificate generation complete!"
echo ""
echo "Files created:"
echo "  CA Certificate:     ${CERTS_DIR}/ca-cert.pem"
echo "  CA Private Key:     ${CERTS_DIR}/ca-key.pem"
echo "  Server Certificate: ${CERTS_DIR}/server-cert.pem"
echo "  Server Private Key: ${CERTS_DIR}/server-key.pem"
echo ""
echo "To trust the CA on your system:"
echo "  - Linux (Debian/Ubuntu): sudo cp ${CERTS_DIR}/ca-cert.pem /usr/local/share/ca-certificates/yt-comment-fetcher-ca.crt && sudo update-ca-certificates"
echo "  - macOS: sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ${CERTS_DIR}/ca-cert.pem"
echo ""
echo "WARNING: These certificates are for DEVELOPMENT/TESTING ONLY!"
echo "         DO NOT use them in production environments."
