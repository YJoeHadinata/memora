#!/usr/bin/env bash
set -euo pipefail

echo "=== Packaging Memora into macOS .dmg ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

# 1. Build app if needed
if [ ! -d "dist/Memora.app" ]; then
    echo "Memora.app not found in dist/. Building now..."
    "$SCRIPT_DIR/build-macos.sh"
fi

VERSION="$(grep '^version' Cargo.toml | head -n 1 | cut -d '"' -f 2)"
DMG_NAME="Memora-${VERSION}.dmg"
STAGING_DIR="dist/dmg_staging"
OUTPUT_DMG="dist/${DMG_NAME}"

# 2. Prepare staging directory
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"
cp -R "dist/Memora.app" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"

# 3. Create DMG with hdiutil
echo "Creating compressed DMG..."
rm -f "$OUTPUT_DMG"
hdiutil create \
    -volname "Memora" \
    -srcfolder "$STAGING_DIR" \
    -ov \
    -format UDZO \
    "$OUTPUT_DMG"

# 4. Generate SHA-256 checksum
echo "Generating SHA-256 checksum..."
shasum -a 256 "$OUTPUT_DMG" > "${OUTPUT_DMG}.sha256"

# Cleanup staging
rm -rf "$STAGING_DIR"

echo "=== Packaging Complete ==="
echo "DMG File: $OUTPUT_DMG"
echo "Checksum: $(cat "${OUTPUT_DMG}.sha256")"
