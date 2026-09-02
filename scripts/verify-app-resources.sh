#!/usr/bin/env bash
set -euo pipefail

APP_BUNDLE="${1:-}"
if [[ -z "$APP_BUNDLE" || ! -d "$APP_BUNDLE" ]]; then
    echo "usage: $0 /path/to/AI Token Meter.app" >&2
    exit 2
fi

CONTENTS_DIR="$APP_BUNDLE/Contents"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

if ! plutil -lint "$CONTENTS_DIR/Info.plist" >/dev/null 2>&1; then
    echo "invalid or missing app Info.plist: $CONTENTS_DIR/Info.plist" >&2
    exit 1
fi

required_resources=(
    "Logos/claude.png"
    "Logos/codex.svg"
    "Logos/deepseek.svg"
    "Backgrounds/floating-strip-deep-sea.png"
)

for resource in "${required_resources[@]}"; do
    if [[ ! -s "$RESOURCES_DIR/$resource" ]]; then
        echo "missing portable app resource: Contents/Resources/$resource" >&2
        exit 1
    fi
done

echo "Portable app resources verified: $APP_BUNDLE"
