#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

git clone --quiet --no-hardlinks "$PROJECT_DIR" "$TEST_ROOT/repository"
cp "$PROJECT_DIR/scripts/check-cross-platform-contracts.rb" \
    "$TEST_ROOT/repository/scripts/check-cross-platform-contracts.rb"

cp -R "$PROJECT_DIR/contracts/." "$TEST_ROOT/repository/contracts/"

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

git -C "$TEST_ROOT/repository" update-index --chmod=+x scripts/package-cross-platform-release.sh

snapshot_schema="$TEST_ROOT/repository/contracts/schemas/usage-snapshot.schema.json"
cp "$snapshot_schema" "$TEST_ROOT/original-schema.json"
ruby -rjson -e 'path = ARGV.fetch(0); value = JSON.parse(File.read(path)); value["properties"]["displayName"]["enum"].delete("Gemini"); File.write(path, JSON.generate(value))' "$snapshot_schema"
if ruby "$TEST_ROOT/repository/scripts/check-cross-platform-contracts.rb" \
    "$TEST_ROOT/repository" >"$TEST_ROOT/schema-missing-gemini.log" 2>&1; then
    echo "Snapshot schema must accept the registered Gemini display name." >&2
    exit 1
fi
grep -Fq "Snapshot schema displayName enum is incomplete" "$TEST_ROOT/schema-missing-gemini.log"
grep -Fq "gemini-unavailable.json: displayName is rejected by snapshot schema" "$TEST_ROOT/schema-missing-gemini.log"
cp "$TEST_ROOT/original-schema.json" "$snapshot_schema"

gemini_fixture="$TEST_ROOT/repository/contracts/fixtures/gemini-unavailable.json"
cp "$gemini_fixture" "$TEST_ROOT/original-gemini.json"
ruby -rjson -e 'path = ARGV.fetch(0); value = JSON.parse(File.read(path)); value["displayName"] = "Unknown Product"; File.write(path, JSON.generate(value))' "$gemini_fixture"
if ruby "$TEST_ROOT/repository/scripts/check-cross-platform-contracts.rb" \
    "$TEST_ROOT/repository" >"$TEST_ROOT/fixture-invalid-name.log" 2>&1; then
    echo "Snapshot schema must reject an unknown fixture display name." >&2
    exit 1
fi
grep -Fq "gemini-unavailable.json: displayName is rejected by snapshot schema" "$TEST_ROOT/fixture-invalid-name.log"
cp "$TEST_ROOT/original-gemini.json" "$gemini_fixture"

ruby -rjson -e 'path = ARGV.fetch(0); value = JSON.parse(File.read(path)); value["usedRatio"] = 0; File.write(path, JSON.generate(value))' "$gemini_fixture"
if ruby "$TEST_ROOT/repository/scripts/check-cross-platform-contracts.rb" \
    "$TEST_ROOT/repository" >"$TEST_ROOT/gemini-false-zero.log" 2>&1; then
    echo "Unavailable Gemini fixture must reject fabricated zero quota." >&2
    exit 1
fi
grep -Fq "Gemini unavailable fixture must not invent quota" "$TEST_ROOT/gemini-false-zero.log"

echo "Cross-platform contract portability, schema display-name, and unavailable-quota tests passed."
