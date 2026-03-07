#!/bin/bash
set -e

VERSION="$1"
BUCKET="gs://nevoflux-builds"
MARKER="${BUCKET}/${VERSION}/done-marker"
TIMEOUT=7200 # 2 hours
INTERVAL=30

echo "Waiting for PGO profile data at ${BUCKET}/${VERSION}/ ..."
elapsed=0
while [ $elapsed -lt $TIMEOUT ]; do
  if gsutil -q stat "$MARKER" 2> /dev/null; then
    echo "PGO profile data is ready."
    gsutil cp "${BUCKET}/${VERSION}/merged.profdata" ./merged.profdata
    gsutil cp "${BUCKET}/${VERSION}/en-US.log" ./en-US.log
    echo "Downloaded profile data."
    exit 0
  fi
  echo "Waiting... (${elapsed}s / ${TIMEOUT}s)"
  sleep $INTERVAL
  elapsed=$((elapsed + INTERVAL))
done

echo "ERROR: Timed out waiting for PGO profile data."
exit 1
