#!/bin/bash
set -e

VERSION="$1"
GITHUB_REPO="$2"
GITHUB_TOKEN="$3"

if [ -z "$VERSION" ] || [ -z "$GITHUB_REPO" ] || [ -z "$GITHUB_TOKEN" ]; then
  echo "Usage: upload-to-release.sh <version> <owner/repo> <github-token>"
  exit 1
fi

export GH_TOKEN="$GITHUB_TOKEN"

# Create draft release if it doesn't exist
if ! gh release view "$VERSION" --repo "$GITHUB_REPO" &> /dev/null; then
  echo "Creating draft release $VERSION ..."
  gh release create "$VERSION" \
    --repo "$GITHUB_REPO" \
    --title "Release build - $VERSION" \
    --draft \
    --notes "Build in progress"
fi

# Upload all artifacts from /workspace/artifacts/
echo "Uploading artifacts to draft release $VERSION ..."
for file in /workspace/artifacts/*; do
  if [ -f "$file" ]; then
    BASENAME=$(basename "$file")
    echo "  Uploading: $BASENAME"
    gh release upload "$VERSION" "$file" \
      --repo "$GITHUB_REPO" \
      --clobber
  fi
done

echo "All artifacts uploaded."
