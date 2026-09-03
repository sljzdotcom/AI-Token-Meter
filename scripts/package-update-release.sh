#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
INFO_PLIST="$PROJECT_DIR/Sources/AIMeterApp/Resources/Info.plist"
APP_BUNDLE="$PROJECT_DIR/dist/AI Token Meter.app"
KEY_ACCOUNT="com.millerpan.AIMeter"
REPOSITORY_SLUG="sljzdotcom/AI-Token-Meter"

VERSION="${1:-}"
BUILD="${2:-}"
if [[ -z "$VERSION" || -z "$BUILD" || $# -ne 2 ]]; then
    echo "usage: SPARKLE_TOOLS_DIR=/absolute/path/to/Sparkle/bin $0 VERSION BUILD" >&2
    exit 2
fi
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-preview\.[0-9]+)?$ || ! "$BUILD" =~ ^[1-9][0-9]*$ ]]; then
    echo "version must be semantic and build must be a positive integer" >&2
    exit 2
fi

SPARKLE_TOOLS_DIR="${SPARKLE_TOOLS_DIR:-}"
if [[ -z "$SPARKLE_TOOLS_DIR" || "$SPARKLE_TOOLS_DIR" != /* ]]; then
    echo "SPARKLE_TOOLS_DIR must be an absolute directory" >&2
    exit 2
fi
GENERATE_KEYS="$SPARKLE_TOOLS_DIR/generate_keys"
SIGN_UPDATE="$SPARKLE_TOOLS_DIR/sign_update"
GENERATE_APPCAST="$SPARKLE_TOOLS_DIR/generate_appcast"
for tool in "$GENERATE_KEYS" "$SIGN_UPDATE" "$GENERATE_APPCAST"; do
    if [[ ! -x "$tool" ]]; then
        echo "required Sparkle release tool is unavailable" >&2
        exit 1
    fi
done

plist_value() {
    /usr/libexec/PlistBuddy -c "Print :$1" "$INFO_PLIST" 2>/dev/null || true
}

SOURCE_VERSION="$(plist_value CFBundleShortVersionString)"
SOURCE_BUILD="$(plist_value CFBundleVersion)"
SOURCE_PUBLIC_KEY="$(plist_value SUPublicEDKey)"
if [[ "$VERSION" != "$SOURCE_VERSION" || "$BUILD" != "$SOURCE_BUILD" ]]; then
    echo "release arguments do not match the app Info.plist" >&2
    exit 1
fi

LATEST_TAG="$(git -C "$PROJECT_DIR" tag --sort=-version:refname | head -n 1)"
if [[ -n "$LATEST_TAG" ]]; then
    PREVIOUS_PLIST="$(mktemp "${TMPDIR:-/tmp}/ai-token-meter-previous-plist.XXXXXX")"
    trap 'rm -f "$PREVIOUS_PLIST"' EXIT
    git -C "$PROJECT_DIR" show "$LATEST_TAG:Sources/AIMeterApp/Resources/Info.plist" > "$PREVIOUS_PLIST"
    PREVIOUS_BUILD="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$PREVIOUS_PLIST" 2>/dev/null || true)"
    if [[ "$PREVIOUS_BUILD" =~ ^[0-9]+$ ]] && (( BUILD <= PREVIOUS_BUILD )); then
        echo "release build must be greater than the latest tagged build" >&2
        exit 1
    fi
fi

if [[ "${AI_METER_RELEASE_ALLOW_DIRTY:-0}" != "1" ]] \
    && [[ -n "$(git -C "$PROJECT_DIR" status --porcelain --untracked-files=all)" ]]; then
    echo "release packaging requires a clean Git worktree" >&2
    exit 1
fi

KEYCHAIN_PUBLIC_KEY="$("$GENERATE_KEYS" --account "$KEY_ACCOUNT" -p 2>/dev/null || true)"
if [[ -z "$KEYCHAIN_PUBLIC_KEY" || "$KEYCHAIN_PUBLIC_KEY" != "$SOURCE_PUBLIC_KEY" ]]; then
    echo "the required Sparkle signing key is unavailable or does not match the app" >&2
    exit 1
fi

RELEASE_DIR="$PROJECT_DIR/dist/releases/$VERSION"
ARCHIVE_NAME="AI-Token-Meter-${VERSION}-macOS-arm64.zip"
ARCHIVE="$RELEASE_DIR/$ARCHIVE_NAME"
SHA_NAME="$ARCHIVE_NAME.sha256"
DOWNLOAD_PREFIX="https://github.com/$REPOSITORY_SLUG/releases/download/v${VERSION}/"
GENERATED_APPCAST="$RELEASE_DIR/appcast.xml"

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

"$PROJECT_DIR/scripts/test.sh"
AI_METER_INCLUDE_WIDGET=0 "$PROJECT_DIR/scripts/build-app.sh"
"$PROJECT_DIR/scripts/verify-update-bundle.sh" "$APP_BUNDLE"

ditto -c -k --keepParent "$APP_BUNDLE" "$ARCHIVE"
if [[ ! -s "$ARCHIVE" ]]; then
    echo "release archive was not created" >&2
    exit 1
fi

(
    cd "$RELEASE_DIR"
    shasum -a 256 "$ARCHIVE_NAME" > "$SHA_NAME"
)

SIGNATURE_OUTPUT="$("$SIGN_UPDATE" --account "$KEY_ACCOUNT" "$ARCHIVE" 2>/dev/null || true)"
if [[ "$SIGNATURE_OUTPUT" != *"sparkle:edSignature="* || "$SIGNATURE_OUTPUT" != *"length="* ]]; then
    echo "Sparkle did not produce a valid archive signature" >&2
    exit 1
fi

if [[ -f "$PROJECT_DIR/appcast.xml" ]]; then
    cp "$PROJECT_DIR/appcast.xml" "$GENERATED_APPCAST"
fi
"$GENERATE_APPCAST" \
    --account "$KEY_ACCOUNT" \
    --download-url-prefix "$DOWNLOAD_PREFIX" \
    --link "https://github.com/$REPOSITORY_SLUG" \
    --maximum-versions 3 \
    -o "$GENERATED_APPCAST" \
    "$RELEASE_DIR"

if [[ ! -s "$GENERATED_APPCAST" ]] \
    || ! grep -Fq "<sparkle:version>$BUILD</sparkle:version>" "$GENERATED_APPCAST" \
    || ! grep -Fq "<sparkle:shortVersionString>$VERSION</sparkle:shortVersionString>" "$GENERATED_APPCAST" \
    || ! grep -Fq "sparkle:edSignature=" "$GENERATED_APPCAST" \
    || ! grep -Fq "url=\"$DOWNLOAD_PREFIX$ARCHIVE_NAME\"" "$GENERATED_APPCAST"; then
    echo "generated appcast does not contain the expected signed release" >&2
    exit 1
fi

cp "$GENERATED_APPCAST" "$PROJECT_DIR/appcast.xml"
SPARKLE_TOOLS_DIR="$SPARKLE_TOOLS_DIR" \
    "$PROJECT_DIR/scripts/verify-update-archive.sh" "$PROJECT_DIR/appcast.xml" "$ARCHIVE"
"$PROJECT_DIR/scripts/check-public-release.sh" --repository "$PROJECT_DIR" --archive "$ARCHIVE"

echo "Signed update release prepared: $ARCHIVE"
echo "SHA-256: $RELEASE_DIR/$SHA_NAME"
echo "Appcast: $PROJECT_DIR/appcast.xml"
