#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPOSITORY_SLUG="sljzdotcom/AI-Token-Meter"

VERSION="${1:-}"
BUILD="${2:-}"
if [[ $# -ne 2 || ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-preview\.[0-9]+)?$ || ! "$BUILD" =~ ^[1-9][0-9]*$ ]]; then
    echo "usage: SPARKLE_TOOLS_DIR=/absolute/path/to/Sparkle/bin $0 VERSION BUILD" >&2
    exit 2
fi

cd "$PROJECT_DIR"
if [[ "$(git branch --show-current)" != "main" ]]; then
    echo "cross-platform releases must be prepared from main" >&2
    exit 1
fi
if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
    echo "cross-platform release preparation requires a clean worktree" >&2
    exit 1
fi
if [[ "$(tr -d '[:space:]' < VERSION)" != "$VERSION" ]]; then
    echo "VERSION does not match the requested release" >&2
    exit 1
fi
if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    echo "tag v$VERSION already exists" >&2
    exit 1
fi

gh auth status --hostname github.com >/dev/null
if [[ "$VERSION" == *-preview.* ]]; then
    RELEASE_CHANNEL="preview"
else
    RELEASE_CHANNEL="stable"
fi
AI_METER_RELEASE_CHANNEL="$RELEASE_CHANNEL" \
    SPARKLE_TOOLS_DIR="${SPARKLE_TOOLS_DIR:-}" \
    "$SCRIPT_DIR/package-update-release.sh" "$VERSION" "$BUILD"

RELEASE_DIR="$PROJECT_DIR/dist/releases/$VERSION"
MAC_ARCHIVE="AI-Token-Meter-${VERSION}-macOS-arm64.zip"
MAC_SHA="$MAC_ARCHIVE.sha256"
if [[ "$RELEASE_CHANNEL" == "preview" ]]; then
    APPCAST_ASSET="$RELEASE_DIR/preview-appcast.xml"
    cp "$RELEASE_DIR/appcast.xml" "$APPCAST_ASSET"
else
    APPCAST_ASSET="$PROJECT_DIR/appcast.xml"
fi
for asset in "$RELEASE_DIR/$MAC_ARCHIVE" "$RELEASE_DIR/$MAC_SHA" "$APPCAST_ASSET"; do
    if [[ ! -s "$asset" ]]; then
        echo "required release asset is missing: $asset" >&2
        exit 1
    fi
done

if [[ "$RELEASE_CHANNEL" == "stable" ]]; then
    git add appcast.xml
    if ! git diff --cached --quiet; then
        git commit -m "release: prepare appcast v$VERSION"
    fi
fi
if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
    echo "only the generated appcast may change during release preparation" >&2
    exit 1
fi

git tag -a "v$VERSION" -m "AI Token Meter v$VERSION"
git push origin main
git push origin "v$VERSION"

release_args=("v$VERSION" --repo "$REPOSITORY_SLUG" --verify-tag --draft --generate-notes)
if [[ "$VERSION" == *-preview.* ]]; then
    release_args+=(--prerelease)
fi
gh release create "${release_args[@]}" \
    "$RELEASE_DIR/$MAC_ARCHIVE" \
    "$RELEASE_DIR/$MAC_SHA" \
    "$APPCAST_ASSET"

gh workflow run release.yml \
    --repo "$REPOSITORY_SLUG" \
    --ref "v$VERSION" \
    -f "version=$VERSION" \
    -f "build=$BUILD"

echo "Draft v$VERSION created with verified macOS assets."
echo "The cross-platform workflow will publish it only after the signed Windows build succeeds."
