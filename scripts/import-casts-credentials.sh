#!/bin/sh
# Import the casts client credentials into the login keychain.
#
# The app reads these at launch (see Shared/CastsConnection.swift) rather than
# carrying them in its bundle, so the private key is not shipped inside the
# .app where anyone with a copy could read it.
#
# Usage:
#   scripts/import-casts-credentials.sh <ca.pem> <client-cert.pem> <client-key.pem>
set -eu

if [ $# -ne 3 ]; then
    echo "usage: $0 <ca.pem> <client-cert.pem> <client-key.pem>" >&2
    exit 64
fi

SERVICE=cc.duane.casts

import_one() {
    account=$1
    file=$2
    [ -r "$file" ] || { echo "cannot read $file" >&2; exit 66; }
    # -U updates an existing item rather than failing, so re-running this
    # after issuing a new certificate just works.
    security add-generic-password \
        -s "$SERVICE" -a "$account" -w "$(cat "$file")" -U
    echo "imported $account"
}

import_one ca           "$1"
import_one client-cert  "$2"
import_one client-key   "$3"

echo
echo "Stored under keychain service $SERVICE."
echo "Remove with: security delete-generic-password -s $SERVICE -a <account>"
