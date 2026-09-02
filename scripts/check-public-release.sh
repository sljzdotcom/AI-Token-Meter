#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPOSITORY="$(cd "$SCRIPT_DIR/.." && pwd)"
ARCHIVE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --repository)
            [[ $# -ge 2 ]] || { echo "Missing value for --repository" >&2; exit 2; }
            REPOSITORY="$2"
            shift 2
            ;;
        --archive)
            [[ $# -ge 2 ]] || { echo "Missing value for --archive" >&2; exit 2; }
            ARCHIVE="$2"
            shift 2
            ;;
        *)
            if [[ -z "$ARCHIVE" ]]; then
                ARCHIVE="$1"
                shift
            else
                echo "Unexpected argument" >&2
                exit 2
            fi
            ;;
    esac
done

REPOSITORY="$(cd "$REPOSITORY" && pwd)"
git -C "$REPOSITORY" rev-parse --git-dir >/dev/null 2>&1 || {
    echo "Public release safety check requires a Git repository." >&2
    exit 2
}

TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ai-token-meter-public-release.XXXXXX")"
trap 'rm -rf "$TEMP_DIR"' EXIT

HIGH_CONFIDENCE_PATTERN='([0-9]{8,12}:AA[A-Za-z0-9_-]{25,})|(sk-(proj-)?[A-Za-z0-9_-]{20,})|(sk-ant-[A-Za-z0-9_-]{20,})|(gh[pousr]_[A-Za-z0-9]{20,})|(AKIA[0-9A-Z]{16})|(-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----)|(Authorization:[[:space:]]*Bearer[[:space:]]+[A-Za-z0-9._~+/-]{20,})'
ALLOWED_TEST_TOKENS=(
    "sk-private""-regression-token-1234567890"
    "sk-sensitive""-model-value"
    "sk-widget""-reset-secret-123456"
    "sk-widget""-private-123456789"
)

fail() {
    echo "Public release safety checks failed: $1" >&2
    exit 1
}

sanitize_test_values() {
    local sed_arguments=()
    local token
    for token in "${ALLOWED_TEST_TOKENS[@]}"; do
        sed_arguments+=("-e" "s|$token|[ALLOWED-TEST-VALUE]|g")
    done
    sed "${sed_arguments[@]}"
}

contains_credential() {
    local file="$1"
    strings "$file" 2>/dev/null \
        | sanitize_test_values \
        | grep -Eaq "$HIGH_CONFIDENCE_PATTERN"
}

while IFS= read -r -d '' relative_path; do
    lower_path="$(printf '%s' "$relative_path" | tr '[:upper:]' '[:lower:]')"
    case "/$lower_path" in
        */.env|*/.env.*|*.pem|*.p12|*.pfx|*.mobileprovision|*id_rsa|*id_ed25519)
            [[ "$lower_path" == *.example ]] || fail "a credential-bearing filename is tracked"
            ;;
    esac
done < <(git -C "$REPOSITORY" ls-files -z)

while IFS= read -r -d '' relative_path; do
    file="$REPOSITORY/$relative_path"
    [[ -f "$file" ]] || continue
    if contains_credential "$file"; then
        fail "current repository content contains a high-confidence credential pattern"
    fi
done < <(git -C "$REPOSITORY" ls-files -co --exclude-standard -z)

if git -C "$REPOSITORY" rev-parse --verify HEAD >/dev/null 2>&1; then
    git -C "$REPOSITORY" log -p --all --full-history --no-ext-diff > "$TEMP_DIR/history.patch"
    if contains_credential "$TEMP_DIR/history.patch"; then
        fail "Git history contains a high-confidence credential pattern"
    fi
fi

GITLEAKS_CONFIG="$SCRIPT_DIR/../.gitleaks.toml"
if command -v gitleaks >/dev/null 2>&1; then
    gitleaks dir "$REPOSITORY" --config "$GITLEAKS_CONFIG" --no-banner --redact \
        > "$TEMP_DIR/gitleaks-worktree.log" 2>&1 \
        || fail "Gitleaks rejected the current repository content"
    gitleaks git "$REPOSITORY" --config "$GITLEAKS_CONFIG" --no-banner --redact \
        > "$TEMP_DIR/gitleaks-history.log" 2>&1 \
        || fail "Gitleaks rejected the Git history"
fi

if [[ -n "$ARCHIVE" ]]; then
    if [[ "$ARCHIVE" != /* ]]; then
        ARCHIVE="$PWD/$ARCHIVE"
    fi
    [[ -f "$ARCHIVE" ]] || fail "the requested release archive does not exist"
    mkdir -p "$TEMP_DIR/archive"
    unzip -qq "$ARCHIVE" -d "$TEMP_DIR/archive" \
        || fail "the requested release archive cannot be extracted"
    while IFS= read -r -d '' file; do
        if contains_credential "$file"; then
            fail "the release archive contains a high-confidence credential pattern"
        fi
    done < <(find "$TEMP_DIR/archive" -type f -print0)
    if command -v gitleaks >/dev/null 2>&1; then
        gitleaks dir "$TEMP_DIR/archive" --config "$GITLEAKS_CONFIG" --no-banner --redact \
            > "$TEMP_DIR/gitleaks-archive.log" 2>&1 \
            || fail "Gitleaks rejected the release archive"
    fi
fi

echo "Public release safety checks passed."
