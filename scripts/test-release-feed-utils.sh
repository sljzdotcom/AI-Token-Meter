#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "$TEST_ROOT"' EXIT

mkdir -p "$TEST_ROOT/bin"
cat >"$TEST_ROOT/bin/gh" <<'EOF'
#!/usr/bin/env bash
case "${MOCK_GH_RESPONSE:?}" in
    present)
        printf 'HTTP/2.0 200 OK\n'
        exit 0
        ;;
    absent)
        printf 'HTTP/2.0 404 Not Found\n'
        exit 1
        ;;
    unauthorized)
        printf 'HTTP/2.0 401 Unauthorized\n'
        exit 1
        ;;
    network)
        printf 'network unavailable\n' >&2
        exit 2
        ;;
esac
EOF
chmod +x "$TEST_ROOT/bin/gh"

export PATH="$TEST_ROOT/bin:$PATH"
export GITHUB_REPOSITORY="sljzdotcom/AI-Token-Meter"
source "$SCRIPT_DIR/release-feed-utils.sh"

export MOCK_GH_RESPONSE=present
test "$(probe_github_release windows-preview-feed "$TEST_ROOT/present")" = "present"

export MOCK_GH_RESPONSE=absent
test "$(probe_github_release windows-preview-feed "$TEST_ROOT/absent")" = "absent"

export MOCK_GH_RESPONSE=unauthorized
if probe_github_release windows-preview-feed "$TEST_ROOT/unauthorized" >/dev/null 2>&1; then
    echo "401 must not be treated as an absent release" >&2
    exit 1
fi

export MOCK_GH_RESPONSE=network
if probe_github_release windows-preview-feed "$TEST_ROOT/network" >/dev/null 2>&1; then
    echo "network failure must not be treated as an absent release" >&2
    exit 1
fi

mkdir -p "$TEST_ROOT/transaction"
if release_may_return_to_draft "$TEST_ROOT/transaction" 0 1; then
    echo "an already-public recovery run must not be returned to draft" >&2
    exit 1
fi
touch "$TEST_ROOT/transaction/publication-attempted"
if release_may_return_to_draft "$TEST_ROOT/transaction" 1 1; then
    echo "failed feed restoration must keep the release public" >&2
    exit 1
fi
if release_may_return_to_draft "$TEST_ROOT/transaction" 0 0; then
    echo "a feed that still references the target must keep the release public" >&2
    exit 1
fi
release_may_return_to_draft "$TEST_ROOT/transaction" 0 1

echo "Release feed probe tests passed."
