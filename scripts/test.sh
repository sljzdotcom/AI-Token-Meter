#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LOCAL_BUILD_ROOT="${TMPDIR:-/tmp}"
TEST_BUILD_DIR="${AI_METER_TEST_BUILD_DIR:-${LOCAL_BUILD_ROOT%/}/com.millerpan.AIMeter-test}"
SWIFTPM_STATE_DIR="$TEST_BUILD_DIR/swiftpm-state"
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-$TEST_BUILD_DIR/clang-module-cache}"

cd "$PROJECT_DIR"
run_swift_tests() {
    swift test \
        --disable-sandbox \
        --cache-path "$SWIFTPM_STATE_DIR/cache" \
        --config-path "$SWIFTPM_STATE_DIR/config" \
        --security-path "$SWIFTPM_STATE_DIR/security" \
        --scratch-path "$TEST_BUILD_DIR/build" \
        "$@"
}

if [[ "$#" -eq 0 ]]; then
    run_swift_tests --skip PTYCommandRunnerTests
    run_swift_tests --skip-build --filter PTYCommandRunnerTests
else
    run_swift_tests "$@"
fi

ruby "$PROJECT_DIR/scripts/check-cross-platform-contracts.rb" "$PROJECT_DIR"
"/usr/bin/env" bash "$PROJECT_DIR/scripts/test-cross-platform-contracts.sh"
"/usr/bin/env" bash "$PROJECT_DIR/scripts/test-normalize-windows-release-assets.sh"
"/usr/bin/env" bash "$PROJECT_DIR/scripts/test-release-feed-utils.sh"
"$PROJECT_DIR/scripts/check-docs.sh"
"$PROJECT_DIR/scripts/check-public-release.sh"
