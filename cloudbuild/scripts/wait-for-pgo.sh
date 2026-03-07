#!/bin/bash
set -e

VERSION="$1"
COMMIT_SHA="$2"
BUCKET="gs://nevoflux-builds"
PREFIX="${BUCKET}/${VERSION}/${COMMIT_SHA}"
MARKER="${PREFIX}/done-marker"
TIMEOUT=7200 # 2 hours
INTERVAL=30

echo "Waiting for PGO profile data at ${PREFIX}/ ..."
elapsed=0
while [ $elapsed -lt $TIMEOUT ]; do
  if gsutil -q stat "$MARKER" 2>/dev/null; then
    echo "PGO profile data is ready."
    gsutil cp "${PREFIX}/merged.profdata" ./merged.profdata
    gsutil cp "${PREFIX}/en-US.log" ./en-US.log
    echo "Downloaded profile data."
    exit 0
  fi
  echo "Waiting... (${elapsed}s / ${TIMEOUT}s)"
  sleep $INTERVAL
  elapsed=$((elapsed + INTERVAL))
done

echo "ERROR: Timed out waiting for PGO profile data."
exit 1
