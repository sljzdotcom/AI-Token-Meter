#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-meter-windows-assets.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

SOURCE_DIR="$TEST_ROOT/source"
STAGE_DIR="$TEST_ROOT/stage"
VERSION="0.3.0-preview.0"
mkdir -p "$SOURCE_DIR"

printf 'signed installer fixture' > "$SOURCE_DIR/AI Token Meter_${VERSION}_x64-setup.exe"
printf 'trusted-signature-fixture\n' > "$SOURCE_DIR/AI Token Meter_${VERSION}_x64-setup.exe.sig"

node "$PROJECT_DIR/scripts/normalize-windows-release-assets.mjs" \
    "$SOURCE_DIR" "$STAGE_DIR" "$VERSION"

INSTALLER="AI-Token-Meter-${VERSION}-windows-x64-setup.exe"
test -s "$STAGE_DIR/$INSTALLER"
test -s "$STAGE_DIR/$INSTALLER.sig"
test -s "$STAGE_DIR/$INSTALLER.sha256"
test "$(find "$STAGE_DIR" -type f | wc -l | tr -d '[:space:]')" = "3"
test "$(cat "$STAGE_DIR/$INSTALLER.sig")" = "trusted-signature-fixture"
(
    cd "$STAGE_DIR"
    shasum -a 256 -c "$INSTALLER.sha256"
)

printf 'duplicate installer' > "$SOURCE_DIR/duplicate-setup.exe"
printf 'duplicate signature\n' > "$SOURCE_DIR/duplicate-setup.exe.sig"
if node "$PROJECT_DIR/scripts/normalize-windows-release-assets.mjs" \
    "$SOURCE_DIR" "$TEST_ROOT/duplicate-stage" "$VERSION" 2>/dev/null; then
    echo "Normalizer accepted multiple signed NSIS installers" >&2
    exit 1
fi

rm "$SOURCE_DIR/duplicate-setup.exe" "$SOURCE_DIR/duplicate-setup.exe.sig"
rm "$SOURCE_DIR/AI Token Meter_${VERSION}_x64-setup.exe.sig"
if node "$PROJECT_DIR/scripts/normalize-windows-release-assets.mjs" \
    "$SOURCE_DIR" "$TEST_ROOT/missing-signature-stage" "$VERSION" 2>/dev/null; then
    echo "Normalizer accepted an unsigned NSIS installer" >&2
    exit 1
fi

echo "Windows release asset normalization tests passed."
