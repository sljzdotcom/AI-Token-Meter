#!/usr/bin/env bash
set -euo pipefail

APP_BUNDLE="${1:-}"
if [[ -z "$APP_BUNDLE" || ! -d "$APP_BUNDLE" ]]; then
    echo "usage: $0 /path/to/AI Token Meter.app" >&2
    exit 2
fi

CONTENTS_DIR="$APP_BUNDLE/Contents"
INFO_PLIST="$CONTENTS_DIR/Info.plist"
EXECUTABLE="$CONTENTS_DIR/MacOS/AIMeterApp"
SPARKLE_FRAMEWORK="$CONTENTS_DIR/Frameworks/Sparkle.framework"
SPARKLE_VERSION="$SPARKLE_FRAMEWORK/Versions/B"

required_paths=(
    "$SPARKLE_FRAMEWORK"
    "$SPARKLE_VERSION/Sparkle"
    "$SPARKLE_VERSION/Autoupdate"
    "$SPARKLE_VERSION/Updater.app/Contents/MacOS/Updater"
    "$SPARKLE_VERSION/XPCServices/Downloader.xpc/Contents/MacOS/Downloader"
    "$SPARKLE_VERSION/XPCServices/Installer.xpc/Contents/MacOS/Installer"
)

for path in "${required_paths[@]}"; do
    if [[ ! -e "$path" ]]; then
        echo "missing required Sparkle component" >&2
        exit 1
    fi
done

feed_url="$(/usr/libexec/PlistBuddy -c 'Print :SUFeedURL' "$INFO_PLIST" 2>/dev/null || true)"
public_key="$(/usr/libexec/PlistBuddy -c 'Print :SUPublicEDKey' "$INFO_PLIST" 2>/dev/null || true)"
automatic_checks="$(/usr/libexec/PlistBuddy -c 'Print :SUEnableAutomaticChecks' "$INFO_PLIST" 2>/dev/null || true)"
automatic_install="$(/usr/libexec/PlistBuddy -c 'Print :SUAutomaticallyUpdate' "$INFO_PLIST" 2>/dev/null || true)"

if [[ "$feed_url" != "https://raw.githubusercontent.com/sljzdotcom/AI-Token-Meter/main/appcast.xml" ]]; then
    echo "invalid Sparkle SUFeedURL" >&2
    exit 1
fi
if [[ ! "$public_key" =~ ^[A-Za-z0-9+/]{43}=$ ]]; then
    echo "invalid or missing Sparkle SUPublicEDKey" >&2
    exit 1
fi
if [[ "$automatic_checks" != "false" || "$automatic_install" != "false" ]]; then
    echo "automatic Sparkle behavior must remain disabled" >&2
    exit 1
fi

main_links="$(otool -L "$EXECUTABLE" | tail -n +2)"
if [[ "$main_links" != *"@rpath/Sparkle.framework"* ]]; then
    echo "main executable is not linked to @rpath/Sparkle.framework" >&2
    exit 1
fi

main_load_commands="$(otool -l "$EXECUTABLE")"
if [[ "$main_load_commands" != *"@executable_path/../Frameworks"* ]]; then
    echo "main executable cannot resolve its embedded Frameworks directory" >&2
    exit 1
fi

if [[ "$main_links" == *".build"* || "$main_links" == *".worktrees"* ]]; then
    echo "main executable contains a local build dependency" >&2
    exit 1
fi

codesign --verify --strict --verbose=2 "$SPARKLE_FRAMEWORK"
codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"

echo "Signed Sparkle update bundle verified: $APP_BUNDLE"
