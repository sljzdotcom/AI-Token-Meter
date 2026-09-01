#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -lt 1 || "$#" -gt 2 ]]; then
    echo "Usage: $0 <AI Token Meter.app> [expected-team-id]" >&2
    exit 2
fi

APP="$1"
EXPECTED_TEAM_ID="${2:-}"
APPEX="$APP/Contents/PlugIns/AITokenMeterWidget.appex"
WIDGET_EXECUTABLE="$APPEX/Contents/MacOS/AIMeterWidgetExtension"

test -x "$WIDGET_EXECUTABLE"
plutil -lint "$APPEX/Contents/Info.plist" >/dev/null
codesign --verify --deep --strict --verbose=2 "$APP"

if [[ -z "$EXPECTED_TEAM_ID" ]]; then
    EXPECTED_TEAM_ID="$(codesign -dv --verbose=4 "$APP" 2>&1 | awk -F= '/^TeamIdentifier=/{ print $2; exit }')"
fi
if [[ -z "$EXPECTED_TEAM_ID" || "$EXPECTED_TEAM_ID" == "not set" ]]; then
    echo "Could not determine the signed Team ID." >&2
    exit 1
fi

EXPECTED_GROUP="$EXPECTED_TEAM_ID.com.millerpan.AIMeter"
ACTUAL_INFO_GROUP="$(plutil -extract AIWidgetAppGroupIdentifier raw -o - "$APPEX/Contents/Info.plist")"
if [[ "$ACTUAL_INFO_GROUP" != "$EXPECTED_GROUP" ]]; then
    echo "Widget Info.plist App Group mismatch." >&2
    exit 1
fi

VERIFY_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ai-token-meter-widget-verify.XXXXXX")"
trap 'rm -rf "$VERIFY_DIR"' EXIT
codesign -d --entitlements :- "$APP" >"$VERIFY_DIR/app-entitlements.plist" 2>/dev/null
codesign -d --entitlements :- "$APPEX" >"$VERIFY_DIR/widget-entitlements.plist" 2>/dev/null

APP_GROUPS="$(plutil -extract com.apple.security.application-groups json -o - "$VERIFY_DIR/app-entitlements.plist")"
WIDGET_GROUPS="$(plutil -extract com.apple.security.application-groups json -o - "$VERIFY_DIR/widget-entitlements.plist")"
WIDGET_SANDBOX="$(plutil -extract com.apple.security.app-sandbox raw -o - "$VERIFY_DIR/widget-entitlements.plist")"

if [[ "$APP_GROUPS" != *"\"$EXPECTED_GROUP\""* || "$WIDGET_GROUPS" != *"\"$EXPECTED_GROUP\""* ]]; then
    echo "Signed App Group entitlements do not match." >&2
    exit 1
fi
if [[ "$WIDGET_SANDBOX" != "true" ]]; then
    echo "Widget App Sandbox entitlement is missing." >&2
    exit 1
fi

echo "Verified signed Widget bundle for Team ID $EXPECTED_TEAM_ID."
