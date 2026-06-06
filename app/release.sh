#!/usr/bin/env bash
# Build a signed + notarized + stapled macOS release of Lattice.
#
# Requires app/.env.release (git-ignored) with:
#   APPLE_SIGNING_IDENTITY  "Developer ID Application: NAME (TEAMID)"
#   APPLE_API_KEY           App Store Connect key id   (notarization)
#   APPLE_API_ISSUER        App Store Connect issuer id (notarization)
#   APPLE_API_KEY_PATH      absolute path to AuthKey_*.p8
#
# One-time prereq: a "Developer ID Application" certificate in the login
# keychain — Xcode > Settings > Accounts > Manage Certificates > + .
set -euo pipefail
cd "$(dirname "$0")"

if [[ ! -f .env.release ]]; then
  echo "missing app/.env.release — see header comment" >&2
  exit 1
fi
set -a; source .env.release; set +a

echo "==> tauri build (signs + notarizes + staples the .app, builds the .dmg)"
npm run tauri build

DMG=$(ls -t src-tauri/target/release/bundle/dmg/*.dmg | head -1)
APP="src-tauri/target/release/bundle/macos/Lattice.app"

# Tauri notarizes the .app but builds the .dmg afterward, so the DMG wrapper
# is signed but not notarized/stapled. Submit + staple it so the DMG also
# verifies offline (no Gatekeeper prompt on mount).
echo "==> notarize + staple the .dmg"
xcrun notarytool submit "$DMG" \
  --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" \
  --wait
xcrun stapler staple "$DMG"

echo "==> verify"
codesign --verify --strict --verbose=2 "$APP"
spctl -a -vvv "$APP"
spctl -a -t open --context context:primary-signature -vvv "$DMG"
xcrun stapler validate "$DMG"

echo
echo "Signed + notarized + stapled release: $DMG"
