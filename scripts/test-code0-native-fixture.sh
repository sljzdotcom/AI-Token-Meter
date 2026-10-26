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
    --filter 'Code0Native(WebKit|Driver)FixtureTests' 2>&1 | tee "$FIXTURE_LOG"

# A missing/disabled suite can otherwise finish with zero tests and exit zero.
python3 - "$FIXTURE_LOG" <<'PYTHON'
import json
import sys
from pathlib import Path

prefix = "CODE0_NATIVE_PROBE "
records = []
driver_prefix = "CODE0_NATIVE_DRIVER "
driver_records = []
for line in Path(sys.argv[1]).read_text().splitlines():
    if "nearly matches optional requirement" in line:
        raise SystemExit("Native fixture contains a mismatched optional delegate signature")
    if driver_prefix in line:
        payload = line.split(driver_prefix, 1)[1].strip()
        if len(payload.encode("utf-8")) > 2048:
            raise SystemExit("Native driver evidence exceeded its limit")
        driver_records.append(json.loads(payload))
        continue
    if prefix not in line:
        continue
    payload = line.split(prefix, 1)[1].strip()
    if len(payload.encode("utf-8")) > 2048:
        raise SystemExit("Native fixture evidence exceeded its limit")
    records.append(json.loads(payload))
if len(records) != 1 or records[0].get("schema") != 1 or records[0].get("result") != "completed-observation":
    raise SystemExit("Expected exactly one completed native origin observation")
expected_cases = {
    "cold-driver", "unloaded-policies", "capture", "retain-window", "retain-view",
    "alert", "confirm", "prompt", "popup", "file-panel", "external-navigation",
    "download", "media", "geolocation", "notification", "hidden-display", "http-auth",
    "visible-media-denial", "visible-geolocation-denial",
}
visible_cases = {"visible-media-denial", "visible-geolocation-denial"}
if len(driver_records) != len(expected_cases) or {row.get("case") for row in driver_records} != expected_cases:
    raise SystemExit("Expected exactly one native driver record for every required case")
for row in driver_records:
    if row.get("schema") != 1 or any(row.get(key) is not True for key in ("windowReleased", "viewReleased", "hiddenAfter", "unfocusedAfter")):
        raise SystemExit("Native driver cleanup or visibility evidence is incomplete")
    if row["case"] == "cold-driver":
        if row.get("outcome") != "cold-no-browser" or row.get("policiesInstalled") is not False:
            raise SystemExit("Cold driver unexpectedly initialized a browser")
    else:
        if row.get("outcome") != "driver-observation" or any(row.get(key) is not True for key in ("policiesInstalled", "releasedAck")):
            raise SystemExit("Required native driver case did not observe installed policies and actual release")
        if row["case"] in visible_cases:
            if row.get("displayMode") != "explicit-login" or row.get("visibleObserved") is not True or row.get("captureInactive") is not True:
                raise SystemExit("Visible permission evidence must come from an explicit synthetic login with inactive capture")
            if row.get("denialEvidence") not in {"native-delegate-deny", "page-permission-denied"}:
                raise SystemExit("Visible permission case did not observe an actual denial")
        elif any(row.get(key) is not True for key in ("hiddenBefore", "unfocusedBefore")):
            raise SystemExit("A hidden native case was shown or focused")
        if row["case"] in {"media", "geolocation"}:
            if (row.get("permissionEvidence") != "pending-before-permission" or
                    row.get("sessionTermination") != "cancelled-by-session" or
                    row.get("deadline") != "injected-monotonic-45s" or
                    row.get("requestStarted") is not True or row.get("captureInactive") is not True):
                raise SystemExit("Hidden permission wait lacks actual request and session-deadline cancellation evidence")
print("Native origin and all required driver cases executed; real website account acceptance is separate.")
PYTHON
