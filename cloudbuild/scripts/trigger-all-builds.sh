#!/bin/bash
# Trigger all 4 independent GCB build jobs in parallel.
#
# Usage:
#   ./cloudbuild/scripts/trigger-all-builds.sh v0.1.2
#   ./cloudbuild/scripts/trigger-all-builds.sh v0.1.2 nightly   # custom release branch

set -euo pipefail

TAG_NAME="${1:?Usage: $0 <TAG_NAME> [RELEASE_BRANCH]}"
RELEASE_BRANCH="${2:-release}"
REGION="us-west1"

CONFIGS=(
  cloudbuild/linux-x86_64-build.yaml
  cloudbuild/linux-aarch64-build.yaml
  cloudbuild/windows-x86_64-build.yaml
  cloudbuild/windows-aarch64-build.yaml
)

echo "Triggering 4 builds: TAG_NAME=${TAG_NAME}  RELEASE_BRANCH=${RELEASE_BRANCH}"
echo ""

for cfg in "${CONFIGS[@]}"; do
  echo "  → ${cfg}"
  gcloud builds submit \
    --config="${cfg}" \
    --substitutions="TAG_NAME=${TAG_NAME},_RELEASE_BRANCH=${RELEASE_BRANCH}" \
    --region="${REGION}" \
    --async
  echo ""
done

echo "All 4 builds submitted. Monitor at:"
echo "  gcloud builds list --region=${REGION} --limit=4"
