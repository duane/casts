#!/bin/sh
# Materialize PEM *content* delivered through the environment into files, and
# point the matching *_PATH variable at them.
#
# ECS injects a Secrets Manager value as an environment variable, not a file --
# Fargate has no secret-volume equivalent. But feeding PEM content straight
# into tonic/rustls as an env-sourced String reproducibly breaks the mutual-TLS
# handshake with an HTTP/2 "frame with invalid size" error; commit-store hit
# this on real Fargate, not just Docker Desktop, and never found the proximate
# cause inside tonic. File-based *_PATH delivery has never failed. So convert
# here, once, before exec'ing the server.
set -eu

TLS_DIR=/tmp/casts-tls
mkdir -p "$TLS_DIR"
chmod 700 "$TLS_DIR"

materialize() {
    pem_var=$1
    path_var=$2
    dest=$3
    eval "pem_value=\${$pem_var:-}"
    if [ -n "$pem_value" ]; then
        printf '%s\n' "$pem_value" > "$dest"
        chmod 600 "$dest"
        export "$path_var=$dest"
    fi
}

materialize CASTS_TLS_CERT_PEM  CASTS_TLS_CERT_PATH  "$TLS_DIR/server-cert.pem"
materialize CASTS_TLS_KEY_PEM   CASTS_TLS_KEY_PATH   "$TLS_DIR/server-key.pem"
materialize CASTS_CLIENT_CA_PEM CASTS_CLIENT_CA_PATH "$TLS_DIR/client-ca.pem"

# The casts server is also a commit-store *client*, and mutual TLS applies in
# that direction too.
materialize COMMIT_STORE_CLIENT_CERT_PEM COMMIT_STORE_CLIENT_CERT "$TLS_DIR/cs-client-cert.pem"
materialize COMMIT_STORE_CLIENT_KEY_PEM  COMMIT_STORE_CLIENT_KEY  "$TLS_DIR/cs-client-key.pem"
materialize COMMIT_STORE_CA_PEM          COMMIT_STORE_CA_PEM_PATH "$TLS_DIR/cs-ca.pem"

exec "$@"
