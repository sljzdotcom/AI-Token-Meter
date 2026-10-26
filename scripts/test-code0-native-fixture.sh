#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURE_ROOT="${AI_METER_NATIVE_FIXTURE_BUILD_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/aimeter-code0-native-fixtures}"
mkdir -p "$FIXTURE_ROOT"
FIXTURE_LOG="$FIXTURE_ROOT/fixture-output.log"

# This build has a test-only define and must never supply a release executable.
AI_METER_TEST_BUILD_DIR="$FIXTURE_ROOT" AI_METER_SCREEN_TESTS=1 \
    bash "$PROJECT_DIR/scripts/test.sh" --no-parallel \
    -Xswiftc -D -Xswiftc AI_METER_NATIVE_FIXTURES \
    --filter Code0NativeWebKitFixtureTests 2>&1 | tee "$FIXTURE_LOG"

# A missing/disabled suite can otherwise finish with zero tests and exit zero.
python3 - "$FIXTURE_LOG" <<'PYTHON'
import json
import sys
from pathlib import Path

prefix = "CODE0_NATIVE_PROBE "
records = []
for line in Path(sys.argv[1]).read_text().splitlines():
    if prefix not in line:
        continue
    payload = line.split(prefix, 1)[1].strip()
    if len(payload.encode("utf-8")) > 2048:
        raise SystemExit("Native fixture evidence exceeded its limit")
    records.append(json.loads(payload))
if len(records) != 1 or records[0].get("schema") != 1 or records[0].get("result") != "completed-observation":
    raise SystemExit("Expected exactly one completed native origin observation")
print("Native origin observation executed; this does not establish production browser acceptance.")
PYTHON
