#!/usr/bin/env bash
# Build the casts macOS app and copy play.app somewhere predictable.
#
# Usage:
#   scripts/build-app.sh                 # Debug build -> casts-ios/build/play.app
#   scripts/build-app.sh --release       # Release build
#   scripts/build-app.sh --open          # ...then open it
#   scripts/build-app.sh --run           # ...then run it directly
#
# xcodebuild writes into a DerivedData directory named by a hash of the
# project path, which changes across machines and Xcode versions -- fine for
# Xcode itself, useless for a script or for `open`ing the result by a fixed
# path. This pins -derivedDataPath to a local, gitignored directory so the
# product always lands in the same place.
#
# Signed with a stable local identity ("casts-local-dev", a self-signed
# code-signing cert in the login keychain) rather than ad-hoc. Ad-hoc
# signatures are derived from the binary's own content, so they change on
# every rebuild -- and macOS ties a Keychain item's access grant to the exact
# signing identity that created it, so each ad-hoc rebuild re-triggers the
# authorization prompt for the app's stored credentials, which (arriving
# mid-launch, easy to miss) looks exactly like the app hanging. A stable
# identity means "Always Allow" granted once actually stays granted.
CODE_SIGN_IDENTITY="casts-local-dev"
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../casts-ios"

CONFIGURATION=Debug
ACTION=none
for arg in "$@"; do
  case "$arg" in
    --release) CONFIGURATION=Release ;;
    --open) ACTION=open ;;
    --run) ACTION=run ;;
    *) echo "unknown option: $arg" >&2; exit 64 ;;
  esac
done

DERIVED_DATA="build/DerivedData"
OUT_DIR="build"
APP_NAME="play.app"

echo "==> building play (macOS), configuration=${CONFIGURATION}"
if ! security find-identity -v -p codesigning 2>/dev/null | grep -q "$CODE_SIGN_IDENTITY"; then
  echo "warning: no '${CODE_SIGN_IDENTITY}' code-signing identity found in the login keychain;" >&2
  echo "falling back to ad-hoc signing. Keychain items will re-prompt on every rebuild." >&2
  CODE_SIGN_IDENTITY="-"
fi

xcodebuild \
  -project play.xcodeproj \
  -scheme "play (macOS)" \
  -configuration "$CONFIGURATION" \
  -destination 'platform=macOS' \
  -derivedDataPath "$DERIVED_DATA" \
  CODE_SIGN_IDENTITY="$CODE_SIGN_IDENTITY" \
  CODE_SIGN_STYLE=Manual \
  DEVELOPMENT_TEAM="" \
  build

BUILT="${DERIVED_DATA}/Build/Products/${CONFIGURATION}/${APP_NAME}"
if [ ! -d "$BUILT" ]; then
  echo "expected build output at ${BUILT} but it is not there" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
rm -rf "${OUT_DIR}/${APP_NAME}"
cp -R "$BUILT" "$OUT_DIR/"
echo "==> ${OUT_DIR}/${APP_NAME}"

case "$ACTION" in
  open) open "${OUT_DIR}/${APP_NAME}" ;;
  run)  exec "${OUT_DIR}/${APP_NAME}/Contents/MacOS/play" ;;
  none) ;;
esac
