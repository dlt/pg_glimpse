#!/bin/bash
# Generate self-signed certificates for testing PostgreSQL mutual TLS authentication

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERT_DIR="$SCRIPT_DIR/certs"

echo "Generating test certificates in $CERT_DIR..."

# Create certificate directory
mkdir -p "$CERT_DIR"
cd "$CERT_DIR"

# Generate CA private key and certificate
echo "1. Generating CA certificate..."
openssl req -new -x509 -days 3650 -nodes -out ca.crt -keyout ca.key \
    -subj "/CN=Test CA" 2>/dev/null

# Generate server private key and certificate signing request
echo "2. Generating server certificate..."
openssl req -new -nodes -out server.csr -keyout server.key \
    -subj "/CN=localhost" 2>/dev/null

# Sign server certificate with CA
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key \
    -CAcreateserial -out server.crt -days 3650 \
    -extensions v3_req \
    -extfile <(cat <<EOF
[v3_req]
subjectAltName = DNS:localhost,IP:127.0.0.1
EOF
) 2>/dev/null

# Generate client private key and certificate signing request
echo "3. Generating client certificate..."
openssl req -new -nodes -out client.csr -keyout client.key \
    -subj "/CN=test" 2>/dev/null

# Sign client certificate with CA
openssl x509 -req -in client.csr -CA ca.crt -CAkey ca.key \
    -CAcreateserial -out client.crt -days 3650 2>/dev/null

# Set correct permissions for private keys
chmod 600 server.key client.key ca.key

# Clean up CSR files
rm -f server.csr client.csr ca.srl

echo "✓ Certificates generated successfully!"
echo ""
echo "Files created:"
echo "  - ca.crt       (CA certificate)"
echo "  - ca.key       (CA private key)"
echo "  - server.crt   (Server certificate)"
echo "  - server.key   (Server private key)"
echo "  - client.crt   (Client certificate)"
echo "  - client.key   (Client private key)"
echo ""
echo "Server certificate is signed for: localhost, 127.0.0.1"
echo "Client certificate is signed for: CN=test"
