#!/usr/bin/env bash

# Prints "present" or "absent" for a GitHub Release tag. Only an explicit
# HTTP 404 is considered absent; authentication, network, and server failures
# remain failures so publication never overwrites an unknown feed state.
probe_github_release() {
    local tag="$1"
    local probe_dir="${2:-${RUNNER_TEMP:-/tmp}/release-probes}"
    local headers="$probe_dir/${tag}.headers"
    local errors="$probe_dir/${tag}.errors"
    local status=0

    mkdir -p "$probe_dir"
    if gh api --include --silent \
        "repos/${GITHUB_REPOSITORY}/releases/tags/${tag}" \
        >"$headers" 2>"$errors"; then
        printf 'present\n'
        return 0
    else
        status=$?
    fi

    if grep -Eq '^HTTP/[0-9.]+ 404([[:space:]]|$)' "$headers"; then
        printf 'absent\n'
        return 0
    fi

    printf 'Unable to determine whether GitHub Release %s exists (exit %s).\n' \
        "$tag" "$status" >&2
    cat "$headers" >&2 || true
    cat "$errors" >&2 || true
    return "$status"
}

# A Release can be returned to draft only when this run attempted the original
# draft-to-public transition and every feed restoration/verification succeeded.
release_may_return_to_draft() {
    local backup_dir="$1"
    local rollback_failed="$2"
    local feeds_safe="$3"

    [[ -f "$backup_dir/publication-attempted" \
        && "$rollback_failed" == "0" \
        && "$feeds_safe" == "1" ]]
}

# Prints the greatest numeric CFBundleVersion found across semantic release
# tags. Moving channel tags and unrelated names are ignored.
maximum_release_build() {
    local repository="$1"
    local plist_path="$2"
    local scan_dir
    local plist_file
    local tag
    local build
    local maximum=0
    local found=0

    scan_dir="$(mktemp -d "${TMPDIR:-/tmp}/ai-token-meter-release-builds.XXXXXX")" || return 1
    plist_file="$scan_dir/Info.plist"
    while IFS= read -r tag; do
        [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-preview\.[0-9]+)?$ ]] || continue
        if ! git -C "$repository" show "$tag:$plist_path" >"$plist_file" 2>/dev/null; then
            continue
        fi
        build="$(plutil -extract CFBundleVersion raw -o - "$plist_file" 2>/dev/null || true)"
        if [[ "$build" =~ ^[0-9]+$ ]] && (( build > maximum )); then
            maximum="$build"
            found=1
        fi
    done < <(git -C "$repository" tag --list 'v*' --sort=-version:refname)
    rm -rf "$scan_dir"

    if [[ "$found" == "1" ]]; then
        printf '%s\n' "$maximum"
    fi
}
