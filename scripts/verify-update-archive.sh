#!/usr/bin/env bash
set -euo pipefail

APPCAST="${1:-}"
ARCHIVE="${2:-}"
if [[ -z "$APPCAST" || -z "$ARCHIVE" || $# -ne 2 ]]; then
    echo "usage: SPARKLE_TOOLS_DIR=/absolute/path/to/Sparkle/bin $0 appcast.xml update.zip" >&2
    exit 2
fi
if [[ ! -f "$APPCAST" || ! -f "$ARCHIVE" ]]; then
    echo "appcast and update archive must both exist" >&2
    exit 1
fi

SPARKLE_TOOLS_DIR="${SPARKLE_TOOLS_DIR:-}"
SIGN_UPDATE="$SPARKLE_TOOLS_DIR/sign_update"
if [[ -z "$SPARKLE_TOOLS_DIR" || "$SPARKLE_TOOLS_DIR" != /* || ! -x "$SIGN_UPDATE" ]]; then
    echo "SPARKLE_TOOLS_DIR must contain the official sign_update tool" >&2
    exit 1
fi

KEY_ACCOUNT="com.millerpan.AIMeter"
SIGNATURE="$(xmllint --xpath 'string(//*[local-name()="enclosure"]/@*[local-name()="edSignature"])' "$APPCAST")"
EXPECTED_LENGTH="$(xmllint --xpath 'string(//*[local-name()="enclosure"]/@length)' "$APPCAST")"
EXPECTED_URL="$(xmllint --xpath 'string(//*[local-name()="enclosure"]/@url)' "$APPCAST")"
EXPECTED_VERSION="$(xmllint --xpath 'string(//*[local-name()="shortVersionString"])' "$APPCAST")"
EXPECTED_BUILD="$(xmllint --xpath 'string(//*[local-name()="version"])' "$APPCAST")"
ACTUAL_LENGTH="$(wc -c < "$ARCHIVE" | tr -d '[:space:]')"

if [[ -z "$SIGNATURE" || -z "$EXPECTED_LENGTH" || -z "$EXPECTED_VERSION" || -z "$EXPECTED_BUILD" ]]; then
    echo "appcast is missing signed release metadata" >&2
    exit 1
fi
if [[ "$EXPECTED_LENGTH" != "$ACTUAL_LENGTH" || "$(basename "$EXPECTED_URL")" != "$(basename "$ARCHIVE")" ]]; then
    echo "archive does not match the appcast enclosure" >&2
    exit 1
fi

"$SIGN_UPDATE" --account "$KEY_ACCOUNT" --verify "$ARCHIVE" "$SIGNATURE" >/dev/null

TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ai-token-meter-update-verification.XXXXXX")"
trap 'rm -rf "$TEMP_DIR"' EXIT
TAMPERED_ARCHIVE="$TEMP_DIR/tampered.zip"
cp "$ARCHIVE" "$TAMPERED_ARCHIVE"
printf 'tamper' >> "$TAMPERED_ARCHIVE"
if "$SIGN_UPDATE" --account "$KEY_ACCOUNT" --verify "$TAMPERED_ARCHIVE" "$SIGNATURE" \
    >/dev/null 2>&1; then
    echo "tampered archive unexpectedly passed signature verification" >&2
    exit 1
fi

ARCHIVE_PLIST="$TEMP_DIR/Info.plist"
unzip -p "$ARCHIVE" 'AI Token Meter.app/Contents/Info.plist' > "$ARCHIVE_PLIST"
ARCHIVE_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$ARCHIVE_PLIST")"
ARCHIVE_BUILD="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$ARCHIVE_PLIST")"
if [[ "$ARCHIVE_VERSION" != "$EXPECTED_VERSION" || "$ARCHIVE_BUILD" != "$EXPECTED_BUILD" ]]; then
    echo "archive version does not match the appcast" >&2
    exit 1
fi

echo "Signed update archive verified; tampered copy rejected."
