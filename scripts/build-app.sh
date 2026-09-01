#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_NAME="AI Token Meter"
EXECUTABLE_NAME="AIMeterApp"
WIDGET_EXECUTABLE_NAME="AIMeterWidgetExtension"
DIST_DIR="$PROJECT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
WIDGET_APPEX="$APP_BUNDLE/Contents/PlugIns/AITokenMeterWidget.appex"
WIDGET_CONTENTS="$WIDGET_APPEX/Contents"
LOCAL_BUILD_ROOT="${TMPDIR:-/tmp}"
BUILD_DIR="${AI_METER_BUILD_DIR:-${LOCAL_BUILD_ROOT%/}/com.millerpan.AIMeter-build}"
SWIFTPM_STATE_DIR="$BUILD_DIR/swiftpm-state"
INCLUDE_WIDGET="${AI_METER_INCLUDE_WIDGET:-auto}"
CODESIGN_IDENTITY="${AI_METER_CODESIGN_IDENTITY:-}"
WIDGET_TEAM_ID="${AI_METER_WIDGET_TEAM_ID:-}"
APP_GROUP_IDENTIFIER=""
SHOULD_BUILD_WIDGET=0
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-$BUILD_DIR/clang-module-cache}"
SWIFT_BUILD_ARGS=(
    -c release
    --disable-sandbox
    --cache-path "$SWIFTPM_STATE_DIR/cache"
    --config-path "$SWIFTPM_STATE_DIR/config"
    --security-path "$SWIFTPM_STATE_DIR/security"
    --scratch-path "$BUILD_DIR"
)

detect_widget_signing() {
    local identities selected_line
    identities="$(security find-identity -v -p codesigning 2>/dev/null || true)"

    if [[ -z "$CODESIGN_IDENTITY" ]]; then
        selected_line="$(printf '%s\n' "$identities" | awk '/"Apple Development:/ { print; exit }')"
        if [[ -n "$selected_line" ]]; then
            CODESIGN_IDENTITY="$(printf '%s\n' "$selected_line" | sed -E 's/^[[:space:]]*[0-9]+\) [A-F0-9]+ "(.*)"$/\1/')"
        fi
    else
        selected_line="$(printf '%s\n' "$identities" | grep -F "$CODESIGN_IDENTITY" | head -n 1 || true)"
        if [[ "$selected_line" != *'"Apple Development:'* ]]; then
            CODESIGN_IDENTITY=""
        fi
    fi

    if [[ -z "$WIDGET_TEAM_ID" ]]; then
        if [[ -z "${selected_line:-}" && -n "$CODESIGN_IDENTITY" ]]; then
            selected_line="$CODESIGN_IDENTITY"
        fi
        WIDGET_TEAM_ID="$(printf '%s\n' "${selected_line:-}" | sed -nE 's/.*\(([A-Z0-9]{10})\).*/\1/p')"
    fi
    if [[ ! "$WIDGET_TEAM_ID" =~ ^[A-Z0-9]{10}$ ]]; then
        WIDGET_TEAM_ID=""
    fi
}

configure_widget_build() {
    case "$INCLUDE_WIDGET" in
        0)
            echo "Widget skipped: AI_METER_INCLUDE_WIDGET=0."
            ;;
        auto|1)
            detect_widget_signing
            if [[ -n "$CODESIGN_IDENTITY" && -n "$WIDGET_TEAM_ID" ]]; then
                SHOULD_BUILD_WIDGET=1
                APP_GROUP_IDENTIFIER="$WIDGET_TEAM_ID.com.millerpan.AIMeter"
            elif [[ "$INCLUDE_WIDGET" == "1" ]]; then
                echo "Widget requested but Apple Development signing is unavailable." >&2
                echo "Open Xcode > Settings > Accounts, sign in, and create an Apple Development certificate." >&2
                echo "You may then set AI_METER_CODESIGN_IDENTITY and AI_METER_WIDGET_TEAM_ID if auto-detection is unavailable." >&2
                exit 2
            else
                echo "Widget skipped: no Apple Development identity and Team ID were found."
            fi
            ;;
        *)
            echo "AI_METER_INCLUDE_WIDGET must be auto, 0, or 1." >&2
            exit 2
            ;;
    esac
}

copy_main_app() {
    mkdir -p "$CONTENTS_DIR/MacOS" "$CONTENTS_DIR/Resources"
    install -m 755 "$BIN_DIR/$EXECUTABLE_NAME" "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME"
    local resource_bundle
    resource_bundle="$(find "$BIN_DIR" -maxdepth 1 -type d -name '*AIMeterApp*.bundle' -print -quit)"
    if [[ -n "$resource_bundle" ]]; then
        cp -R "$resource_bundle" "$CONTENTS_DIR/Resources/"
    fi
    install -m 644 \
        "$PROJECT_DIR/Sources/AIMeterApp/Resources/Info.plist" \
        "$CONTENTS_DIR/Info.plist"

    local iconset_dir generated_icon
    iconset_dir="$BUILD_DIR/AppIcon.iconset"
    generated_icon="$BUILD_DIR/AppIcon.icns"
    swift "$PROJECT_DIR/scripts/generate-app-icon.swift" "$iconset_dir" "$generated_icon"
    install -m 644 "$generated_icon" "$CONTENTS_DIR/Resources/AppIcon.icns"
    test -s "$CONTENTS_DIR/Resources/AppIcon.icns"
}

copy_widget_extension() {
    mkdir -p "$WIDGET_CONTENTS/MacOS" "$WIDGET_CONTENTS/Resources/Logos" "$WIDGET_CONTENTS/Resources/Backgrounds"
    install -m 755 "$BIN_DIR/$WIDGET_EXECUTABLE_NAME" "$WIDGET_CONTENTS/MacOS/$WIDGET_EXECUTABLE_NAME"
    install -m 644 \
        "$PROJECT_DIR/Sources/AIMeterWidgetExtension/Resources/Info.plist" \
        "$WIDGET_CONTENTS/Info.plist"
    cp "$PROJECT_DIR"/Sources/AIMeterWidgetExtension/Resources/Logos/* "$WIDGET_CONTENTS/Resources/Logos/"
    cp "$PROJECT_DIR"/Sources/AIMeterWidgetExtension/Resources/Backgrounds/* "$WIDGET_CONTENTS/Resources/Backgrounds/"
    plutil -replace AIWidgetAppGroupIdentifier -string "$APP_GROUP_IDENTIFIER" "$WIDGET_CONTENTS/Info.plist"
    plutil -replace AIWidgetAppGroupIdentifier -string "$APP_GROUP_IDENTIFIER" "$CONTENTS_DIR/Info.plist"
}

prepare_entitlements() {
    APP_ENTITLEMENTS="$BUILD_DIR/AIMeterApp.entitlements"
    WIDGET_ENTITLEMENTS="$BUILD_DIR/AITokenMeterWidget.entitlements"
    cp "$PROJECT_DIR/Sources/AIMeterApp/Resources/AIMeterApp.entitlements" "$APP_ENTITLEMENTS"
    cp "$PROJECT_DIR/Sources/AIMeterWidgetExtension/Resources/AITokenMeterWidget.entitlements" "$WIDGET_ENTITLEMENTS"
    /usr/libexec/PlistBuddy \
        -c "Set :com.apple.security.application-groups:0 $APP_GROUP_IDENTIFIER" \
        "$APP_ENTITLEMENTS"
    /usr/libexec/PlistBuddy \
        -c "Set :com.apple.security.application-groups:0 $APP_GROUP_IDENTIFIER" \
        "$WIDGET_ENTITLEMENTS"
}

codesign_widget_extension() {
    codesign --force --options runtime --timestamp=none \
        --entitlements "$WIDGET_ENTITLEMENTS" \
        --sign "$CODESIGN_IDENTITY" \
        "$WIDGET_APPEX"
}

codesign_main_app() {
    codesign --force --options runtime --timestamp=none \
        --entitlements "$APP_ENTITLEMENTS" \
        --sign "$CODESIGN_IDENTITY" \
        "$APP_BUNDLE"
}

configure_widget_build
cd "$PROJECT_DIR"
swift build "${SWIFT_BUILD_ARGS[@]}" --product "$EXECUTABLE_NAME"
if [[ "$SHOULD_BUILD_WIDGET" == "1" ]]; then
    swift build "${SWIFT_BUILD_ARGS[@]}" --product "$WIDGET_EXECUTABLE_NAME"
fi
BIN_DIR="$(swift build "${SWIFT_BUILD_ARGS[@]}" --show-bin-path)"

if [[ -e "$APP_BUNDLE" ]]; then
    rm -rf "$APP_BUNDLE"
fi
copy_main_app

if [[ "$SHOULD_BUILD_WIDGET" == "1" ]]; then
    copy_widget_extension
    prepare_entitlements
    plutil -lint "$WIDGET_CONTENTS/Info.plist" >/dev/null
    codesign_widget_extension
    codesign_main_app
else
    codesign --force --deep --sign - "$APP_BUNDLE"
fi

plutil -lint "$CONTENTS_DIR/Info.plist" >/dev/null
codesign --verify --deep --strict "$APP_BUNDLE"

if [[ "$SHOULD_BUILD_WIDGET" == "1" ]]; then
    "$PROJECT_DIR/scripts/verify-widget-bundle.sh" "$APP_BUNDLE" "$WIDGET_TEAM_ID"
fi

echo "Built and verified: $APP_BUNDLE"
