#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

git clone --quiet --no-hardlinks "$PROJECT_DIR" "$TEST_ROOT/repository"
cp "$PROJECT_DIR/scripts/check-cross-platform-contracts.rb" \
    "$TEST_ROOT/repository/scripts/check-cross-platform-contracts.rb"

release_entry="$TEST_ROOT/repository/scripts/package-cross-platform-release.sh"
chmod a-x "$release_entry"
if ! ruby "$TEST_ROOT/repository/scripts/check-cross-platform-contracts.rb" \
    "$TEST_ROOT/repository" >"$TEST_ROOT/tracked-executable.log" 2>&1; then
    cat "$TEST_ROOT/tracked-executable.log" >&2
    echo "Git-tracked executable mode must be authoritative across host filesystems." >&2
    exit 1
fi

git -C "$TEST_ROOT/repository" update-index \
    --chmod=-x scripts/package-cross-platform-release.sh
chmod a+x "$release_entry"
if ruby "$TEST_ROOT/repository/scripts/check-cross-platform-contracts.rb" \
    "$TEST_ROOT/repository" >"$TEST_ROOT/tracked-non-executable.log" 2>&1; then
    echo "A release entry tracked as non-executable must be rejected." >&2
    exit 1
fi
grep -Fq "Cross-platform release entry must be executable" \
    "$TEST_ROOT/tracked-non-executable.log"

echo "Cross-platform contract portability tests passed."
