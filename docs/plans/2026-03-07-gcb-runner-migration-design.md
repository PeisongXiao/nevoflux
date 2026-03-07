# Google Cloud Build Runner Migration Design

**Date:** 2026-03-07
**Status:** Approved
**Scope:** CI/CD runner migration — Linux and Windows builds from self-hosted GitHub Actions to Google Cloud Build

## Overview

Move Linux and Windows browser builds from self-hosted GitHub Actions runners to Google Cloud Build (GCB) for better performance (on-demand high-CPU machines) and reduced maintenance burden (no OS/disk/uptime management). macOS builds remain on self-hosted runners. GitHub Actions continues as the release orchestrator.

## Architecture

### Two-System Design

```
GitHub Push/Tag (version tag: ^[0-9]+\.[0-9]+\.[0-9]+)
       |
       +------------------------------+
       v                              v
  GitHub Actions                 Google Cloud Build
  ---------------                ------------------
  - buildid, build-data          - Linux build (x86_64, aarch64)
  - lint                         - Windows PGO stage 1 (cross-compile)
  - source tarball               - Windows PGO stage 3 (cross-compile)
  - macOS builds (self-hosted)   - Windows aarch64 build (no PGO)
  - Windows PGO stage 2          - AppImage packaging
    (windows-latest)             - Upload artifacts to GitHub draft release
  - Release finalization
  - Flatpak (optional)
```

### Artifact Flow

1. **GCB builds complete** -> uploads `.tar.xz`, `.zip`, `.mar`, `.AppImage` to GitHub draft release via `gh` CLI
2. **GCB Windows PGO stage 1** -> uploads PGO zip to GCS bucket for GHA stage 2
3. **GHA Windows PGO stage 2** (windows-latest) -> uploads profile data to GCS bucket for GCB stage 3
4. **GHA macOS builds** -> uploads DMG/MAR as standard GHA artifacts
5. **GHA release job** -> downloads all artifacts (GHA artifacts + draft release assets), finalizes release

### Key Principles

- GCB and GHA triggered by the same tag/push event
- GCS bucket `gs://nevoflux-builds/` used for intermediate PGO artifact exchange
- Secrets stored in GCP Secret Manager
- Custom Docker builder image for fast GCB startup

## Google Cloud Build Configuration

### Trigger Setup

- **Trigger type:** Push to tag matching `^[0-9]+\.[0-9]+\.[0-9]+`
- **Config file:** `cloudbuild/build.yaml`
- **Connected repo:** GitHub via Cloud Build GitHub App connection
- **Substitution variables:** `_GITHUB_TOKEN` (from Secret Manager), `_RELEASE_BRANCH` (default: `release`)

### Machine Type

`E2_HIGHCPU_32` (32 vCPU, 32 GB RAM). Fallback to `N1_HIGHCPU_32` if E2 unavailable. Build timeout: 180 minutes.

### Build Steps (cloudbuild.yaml)

```
Step 1: Setup & Download Firefox Source (serial)
  - npm ci, npm run download, npm run import

Step 2a: Linux x86_64 build          (parallel group)
Step 2b: Linux aarch64 build         (parallel group)
Step 2c: Windows x86_64 PGO stage 1  (parallel group)

Step 3: Upload PGO stage 1 artifact to GCS
  - Blocks until step 2c completes

Step 4: Wait for PGO stage 2 (GHA)
  - Poll GCS for profile data from GHA windows-latest job

Step 5a: Windows x86_64 final build  (after PGO data available)
Step 5b: Windows aarch64 build       (no PGO, can run earlier)

Step 6: Package & Upload
  - AppImage packaging (x86_64 + aarch64)
  - Upload all artifacts to GitHub draft release via gh CLI
```

### Custom Docker Builder Image

Based on `ubuntu:22.04`, pre-installed with:
- Build essentials, LLVM, LLD, yasm, nasm
- Node.js (matching `.nvmrc`)
- Rust toolchain (matching `.rust-toolchain`)
- Wine + VS2022 cross-compile tools (for Windows builds)
- `gh` CLI for GitHub release uploads
- `gsutil` for GCS operations

Image stored at `gcr.io/{project}/nevoflux-builder:latest`. Built and pushed separately.

## GitHub Actions Changes

### Modified Workflows

#### `build.yml` (Main Orchestrator)

| Change | Detail |
|--------|--------|
| Remove `linux` job | Moved to GCB |
| Remove `windows-step-1` job | Moved to GCB |
| Remove `windows-step-3` job | Moved to GCB |
| Remove `appimage` job | Moved to GCB |
| Keep `windows-step-2` | PGO profiling on `windows-latest` |
| Keep `mac`, `mac-uni` | Self-hosted macOS |
| Modify `windows-step-2` | Download PGO stage 1 from GCS, upload profile data to GCS |
| Modify `release` job | Wait for GCB draft release assets, combine with macOS, finalize release |

#### `windows-profile-build.yml`

- Change artifact source: `gsutil cp gs://nevoflux-builds/{version}/pgo-stage-1.zip .`
- Change artifact destination: `gsutil cp merged.profdata gs://nevoflux-builds/{version}/`

### Deleted Workflows

| Workflow | Reason |
|----------|--------|
| `linux-release-build.yml` | Replaced by GCB steps |
| `windows-release-build.yml` | Replaced by GCB steps |

### New Workflows

#### `wait-for-gcb.yml` (reusable workflow)

- Called by `build.yml` release job
- Polls GitHub API for expected draft release assets (linux tarballs, windows zips, AppImages)
- Timeout: 180 minutes
- Outputs: confirmation that all expected assets exist

### Unchanged Workflows

- `macos-release-build.yml`
- `macos-universal-release-build.yml`
- `code-linter.yml`
- `pr-check.yml`
- `clear-all-cache.yml`
- `issue-labeler.yml`
- `issue-metrics.yml`

#### `test-runners.yml`

Updated to test GCS connectivity instead of self-hosted Linux runner.

## Secrets & Permissions

### GCP Secret Manager

| Secret | Purpose |
|--------|---------|
| `github-deploy-token` | GitHub PAT for `gh release upload` from GCB |
| `github-token` | For `gh release download` (nevoflux-agent) |
| `zen-safebrowsing-key` | Safe Browsing API key |

### GitHub Secrets

| Secret | Status | Purpose |
|--------|--------|---------|
| `DEPLOY_KEY` | Existing | Git ops, release creation |
| macOS signing secrets | Existing | Code signing, notarization |
| `GOOGLE_CLOUD_PROJECT` | **New** | GCP project ID |
| `GCS_SERVICE_ACCOUNT_KEY` | **New** | Service account JSON for `gsutil` in `windows-profile-build.yml` |

### IAM Permissions

- Cloud Build service account:
  - `roles/storage.objectAdmin` on `gs://nevoflux-builds/`
  - `roles/secretmanager.secretAccessor` for Secret Manager secrets
- GitHub Actions service account:
  - `roles/storage.objectAdmin` on `gs://nevoflux-builds/` (for PGO stage 2)

### GCS Bucket Structure

```
gs://nevoflux-builds/
  {version}/
    pgo-stage-1.zip          # Windows PGO gen output (GCB -> GHA)
    merged.profdata           # PGO profile data (GHA -> GCB)
    en-US.log                # PGO jar log (GHA -> GCB)
    done-marker              # Signal that stage 2 is complete
```

## New Files in Repository

```
cloudbuild/
  build.yaml                     # Main Cloud Build config
  Dockerfile                     # Custom builder image
  scripts/
    wait-for-pgo.sh             # Poll GCS for PGO profile data
    upload-to-release.sh        # Upload artifacts to GitHub draft release
```

## Runner Strategy (Updated)

| Task | Before | After |
|------|--------|-------|
| Linux browser build | Self-hosted Linux | Google Cloud Build (E2_HIGHCPU_32) |
| Windows cross-compile | Self-hosted Linux | Google Cloud Build (E2_HIGHCPU_32) |
| Windows PGO profiling | GitHub-hosted (windows-latest) | GitHub-hosted (windows-latest) — unchanged |
| macOS browser build | Self-hosted macOS | Self-hosted macOS — unchanged |
| macOS universal + signing | Self-hosted macOS | Self-hosted macOS — unchanged |
| AppImage packaging | Self-hosted Linux | Google Cloud Build |
| Lint / PR check | GitHub-hosted (ubuntu-latest) | GitHub-hosted (ubuntu-latest) — unchanged |
| Release publishing | GitHub-hosted (ubuntu-latest) | GitHub-hosted (ubuntu-latest) — unchanged |

## Migration Plan

1. Build and push the custom Docker builder image
2. Create `cloudbuild/build.yaml` with all Linux + Windows build steps
3. Set up GCS bucket and IAM permissions
4. Store secrets in GCP Secret Manager
5. Add `GOOGLE_CLOUD_PROJECT` and `GCS_SERVICE_ACCOUNT_KEY` to GitHub Secrets
6. Create Cloud Build trigger connected to the GitHub repo
7. Modify `build.yml` to remove Linux/Windows jobs, add GCB wait logic
8. Modify `windows-profile-build.yml` to use GCS for artifact exchange
9. Create `wait-for-gcb.yml` reusable workflow
10. Test end-to-end with a test tag
11. Delete `linux-release-build.yml` and `windows-release-build.yml`
12. Update `test-runners.yml`
