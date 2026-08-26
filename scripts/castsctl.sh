#!/usr/bin/env bash
# Talk to the deployed casts API without spelling out certificates every time.
#
#   scripts/castsctl.sh feeds                     # ListFeeds
#   scripts/castsctl.sh entries                   # ListEntries (whole stream)
#   scripts/castsctl.sh feed <feed-id>            # GetFeed
#   scripts/castsctl.sh add <url>                 # AddFeed  (writes!)
#   scripts/castsctl.sh raw <Method> '<json>'     # anything else
#
# Override with CASTS_ENDPOINT, CASTS_STREAM_ID, CASTS_CERT/KEY/CA.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
PROTO_PATH="$(pwd)/casts/src/proto"

ENDPOINT="${CASTS_ENDPOINT:-casts.duane.cc:443}"
# The stream the macOS/iOS client uses (playApp.swift STREAM_ID).
STREAM_ID="${CASTS_STREAM_ID:-34c3c08d-0d55-4af4-a126-170c1380f07e}"

PKI="${PKI_DIR:-/Users/duane/Projects/infra/pki}"
# Intermediate only, never the chain: including the root would accept a
# server certificate from any intermediate under it.
CA="${CASTS_CA:-$PKI/dist/casts-ca.cert.pem}"
CERT="${CASTS_CERT:-$PKI/issued/casts-ca/duane-mac/cert.pem}"
KEY="${CASTS_KEY:-$PKI/issued/casts-ca/duane-mac/key.pem}"

for f in "$CA" "$CERT" "$KEY"; do
  [ -r "$f" ] || { echo "cannot read $f" >&2; exit 66; }
done
command -v grpcurl >/dev/null || { echo "grpcurl not installed (brew install grpcurl)" >&2; exit 69; }

call() {
  grpcurl -cacert "$CA" -cert "$CERT" -key "$KEY" \
    -import-path "$PROTO_PATH" -proto casts.proto \
    -d "$2" "$ENDPOINT" "GrpcInterface.Casts/$1"
}

CMD="${1:-feeds}"
shift || true
case "$CMD" in
  feeds)   call ListFeeds   "{\"streamId\":\"$STREAM_ID\"}" ;;
  # ListEntriesRequest carries only streamId -- there is no per-feed filter.
  entries) call ListEntries "{\"streamId\":\"$STREAM_ID\"}" ;;
  feed)    [ $# -ge 1 ] || { echo "usage: castsctl.sh feed <feed-id>" >&2; exit 64; }
           call GetFeed     "{\"streamId\":\"$STREAM_ID\",\"feedId\":\"$1\"}" ;;
  add)     [ $# -ge 1 ] || { echo "usage: castsctl.sh add <feed-url>" >&2; exit 64; }
           # A write, and casts has no authorization -- any accepted client
           # can do this. Named explicitly so it is never a typo away.
           echo "adding $1 to stream $STREAM_ID" >&2
           call AddFeed     "{\"streamId\":\"$STREAM_ID\",\"url\":\"$1\"}" ;;
  raw)     [ $# -ge 2 ] || { echo "usage: castsctl.sh raw <Method> '<json>'" >&2; exit 64; }
           call "$1" "$2" ;;
  *)       echo "unknown command: $CMD (feeds|entries|feed|add|raw)" >&2; exit 64 ;;
esac
