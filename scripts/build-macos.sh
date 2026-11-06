#!/usr/bin/env bash
set -euo pipefail

echo "=== Building Memora for macOS ==="

# 1. Compile release binary
cargo build --release

APP_NAME="Memora"
VERSION="$(grep '^version' Cargo.toml | head -n 1 | cut -d '"' -f 2)"
APP_BUNDLE="dist/${APP_NAME}.app"
CONTENTS_DIR="${APP_BUNDLE}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "=== Creating .app Bundle Structure ==="
rm -rf "${APP_BUNDLE}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# 2. Copy binary
cp "target/release/memora" "${MACOS_DIR}/memora"
chmod +x "${MACOS_DIR}/memora"

# 3. Create Info.plist
cat <<EOF > "${CONTENTS_DIR}/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>memora</string>
    <key>CFBundleIdentifier</key>
    <string>com.memora.monitor</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Memora</string>
    <key>CFBundleDisplayName</key>
    <string>Memora</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

if [ -f "assets/icons/AppIcon.icns" ]; then
    cp "assets/icons/AppIcon.icns" "${RESOURCES_DIR}/AppIcon.icns"
fi

# 4. Ad-hoc code sign the bundle for Apple Silicon / Intel macOS Gatekeeper compliance
echo "=== Signing .app Bundle ==="
codesign --force --deep --sign - "${APP_BUNDLE}"

echo "Successfully built and signed ${APP_BUNDLE}"
