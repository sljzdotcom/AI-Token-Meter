#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_NAME="AI Meter"
EXECUTABLE_NAME="AIMeterApp"
DIST_DIR="$PROJECT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
LOCAL_BUILD_ROOT="${TMPDIR:-/tmp}"
BUILD_DIR="${AI_METER_BUILD_DIR:-${LOCAL_BUILD_ROOT%/}/com.millerpan.AIMeter-build}"
SWIFTPM_STATE_DIR="$BUILD_DIR/swiftpm-state"
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-$BUILD_DIR/clang-module-cache}"
SWIFT_BUILD_ARGS=(
    -c release
    --disable-sandbox
    --cache-path "$SWIFTPM_STATE_DIR/cache"
    --config-path "$SWIFTPM_STATE_DIR/config"
    --security-path "$SWIFTPM_STATE_DIR/security"
    --scratch-path "$BUILD_DIR"
)

cd "$PROJECT_DIR"
swift build "${SWIFT_BUILD_ARGS[@]}" --product "$EXECUTABLE_NAME"
BIN_DIR="$(swift build "${SWIFT_BUILD_ARGS[@]}" --show-bin-path)"

if [[ -e "$APP_BUNDLE" ]]; then
    rm -rf "$APP_BUNDLE"
fi
mkdir -p "$CONTENTS_DIR/MacOS" "$CONTENTS_DIR/Resources"

install -m 755 "$BIN_DIR/$EXECUTABLE_NAME" "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME"
RESOURCE_BUNDLE="$(find "$BIN_DIR" -maxdepth 1 -type d -name '*AIMeterApp*.bundle' -print -quit)"
if [[ -n "$RESOURCE_BUNDLE" ]]; then
    cp -R "$RESOURCE_BUNDLE" "$CONTENTS_DIR/Resources/"
fi
install -m 644 \
    "$PROJECT_DIR/Sources/AIMeterApp/Resources/Info.plist" \
    "$CONTENTS_DIR/Info.plist"

plutil -lint "$CONTENTS_DIR/Info.plist" >/dev/null
codesign --force --deep --sign - "$APP_BUNDLE"
codesign --verify --deep --strict "$APP_BUNDLE"

echo "Built and verified: $APP_BUNDLE"
