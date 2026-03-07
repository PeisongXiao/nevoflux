# Google Cloud Build Runner Migration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate Linux and Windows browser builds from self-hosted GitHub Actions runners to Google Cloud Build for better performance and reduced maintenance.

**Architecture:** Two Cloud Build configs (linux-build.yaml, windows-build.yaml) triggered by version tags, each running on E2_HIGHCPU_32 machines. GCS bucket exchanges PGO artifacts with GitHub Actions. GCB uploads final artifacts directly to GitHub draft releases. GitHub Actions handles macOS builds (self-hosted), Windows PGO stage 2 (windows-latest), and release finalization.

**Tech Stack:** Google Cloud Build, GCS, Docker (custom builder image), GitHub Actions, bash scripts, `gh` CLI, `gsutil`

**Reference:** `docs/plans/2026-03-07-gcb-runner-migration-design.md`

---

## Task 1: Create the Custom Docker Builder Image

**Files:**
- Create: `cloudbuild/Dockerfile`

**Step 1: Create cloudbuild directory**

Run: `mkdir -p cloudbuild/scripts`

**Step 2: Write the Dockerfile**

The image must include everything needed to build Firefox from source on Linux, plus cross-compilation tools for Windows.

```dockerfile
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Core build dependencies (matching linux-release-build.yml and windows-release-build.yml)
RUN apt-get update && apt-get install -y \
    python3 python3-pip python3-launchpadlib python3-requests python3-toml \
    dos2unix yasm nasm build-essential \
    libgtk2.0-dev libpython3-dev m4 uuid \
    libasound2-dev libcurl4-openssl-dev libdbus-1-dev libdrm-dev \
    libdbus-glib-1-dev libgtk-3-dev libpulse-dev \
    libx11-xcb-dev libxt-dev xvfb lld llvm \
    git curl wget zip unzip jq \
    autoconf autoconf2.13 automake bison cmake flex gawk \
    gcc-multilib gnupg libbz2-dev libexpat1-dev libffi-dev \
    libncursesw5-dev libsqlite3-dev libssl-dev libtool \
    libucl-dev libxml2-dev msitools ninja-build \
    openssh-client p7zip-full pkg-config procps scons \
    subversion tar uuid-dev zlib1g-dev aria2 \
    libfuse2 desktop-file-utils appstream \
    ca-certificates software-properties-common \
    && rm -rf /var/lib/apt/lists/*

# Node.js 20
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Rust 1.83
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.83 \
    && . "$HOME/.cargo/env" \
    && rustup target add x86_64-unknown-linux-gnu \
    && rustup target add aarch64-unknown-linux-gnu \
    && rustup target add x86_64-pc-windows-msvc \
    && rustup target add aarch64-pc-windows-msvc

# gh CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update && apt-get install -y gh \
    && rm -rf /var/lib/apt/lists/*

# Google Cloud SDK (for gsutil)
RUN echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" > /etc/apt/sources.list.d/google-cloud-sdk.list \
    && curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key --keyring /usr/share/keyrings/cloud.google.gpg add - \
    && apt-get update && apt-get install -y google-cloud-cli \
    && rm -rf /var/lib/apt/lists/*

# Surfer CLI
RUN npm i -g @zen-browser/surfer

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /workspace
```

**Step 3: Verify Dockerfile syntax**

Run: `docker build --check cloudbuild/` (or just validate with `docker build --dry-run` if supported)

**Step 4: Commit**

```bash
git add -f cloudbuild/Dockerfile
git commit -m "build(gcb): add custom Docker builder image for Cloud Build"
```

**Note:** The actual image build and push to `gcr.io` is done manually:
```bash
cd cloudbuild
gcloud builds submit --tag gcr.io/$PROJECT_ID/nevoflux-builder:latest .
```

---

## Task 2: Create Helper Scripts

**Files:**
- Create: `cloudbuild/scripts/wait-for-pgo.sh`
- Create: `cloudbuild/scripts/upload-to-release.sh`

**Step 1: Write wait-for-pgo.sh**

This script polls GCS for PGO profile data uploaded by GitHub Actions (windows-latest PGO stage 2).

```bash
#!/bin/bash
set -e

VERSION="$1"
BUCKET="gs://nevoflux-builds"
MARKER="${BUCKET}/${VERSION}/done-marker"
TIMEOUT=7200  # 2 hours
INTERVAL=30

echo "Waiting for PGO profile data at ${BUCKET}/${VERSION}/ ..."
elapsed=0
while [ $elapsed -lt $TIMEOUT ]; do
  if gsutil -q stat "$MARKER" 2>/dev/null; then
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
```

**Step 2: Write upload-to-release.sh**

This script uploads build artifacts to a GitHub draft release.

```bash
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
if ! gh release view "$VERSION" --repo "$GITHUB_REPO" &>/dev/null; then
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
```

**Step 3: Make scripts executable and commit**

```bash
chmod +x cloudbuild/scripts/wait-for-pgo.sh cloudbuild/scripts/upload-to-release.sh
git add -f cloudbuild/scripts/
git commit -m "build(gcb): add helper scripts for PGO polling and release upload"
```

---

## Task 3: Create Linux Cloud Build Config

**Files:**
- Create: `cloudbuild/linux-build.yaml`

**Step 1: Write linux-build.yaml**

This config builds Linux x86_64, Linux aarch64, and AppImage packages. Builds run sequentially (each needs ~45 min on 32 vCPU, total ~150 min).

```yaml
# Linux browser builds for NevoFlux
# Triggered by version tag push (e.g., 0.1.0)
# Builds: Linux x86_64, Linux aarch64, AppImage x86_64, AppImage aarch64
# Uploads artifacts to GitHub draft release

timeout: '18000s'  # 5 hours

options:
  machineType: 'E2_HIGHCPU_32'
  diskSizeGb: 200
  logging: CLOUD_LOGGING_ONLY

availableSecrets:
  secretManager:
    - versionName: projects/$PROJECT_ID/secrets/github-deploy-token/versions/latest
      env: 'GITHUB_TOKEN'
    - versionName: projects/$PROJECT_ID/secrets/zen-safebrowsing-key/versions/latest
      env: 'ZEN_SAFEBROWSING_KEY'

substitutions:
  _RELEASE_BRANCH: 'release'
  _GITHUB_REPO: 'dorisgyl/nevoflux'

steps:
  # ── Setup: Checkout, install deps, download source ─────────────
  - id: 'setup'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    args:
      - '-c'
      - |
        set -ex
        git config --global user.email "nevoflux-bot@users.noreply.github.com"
        git config --global user.name "NevoFlux Bot"

        npm ci
        npm run surfer -- ci --brand ${_RELEASE_BRANCH} --display-version ${TAG_NAME}

        # Download Firefox source
        for attempt in 1 2 3 4 5; do
          npm run download --verbose && break
          if [ $$attempt -lt 5 ]; then
            echo "Download attempt $$attempt/5 failed. Retrying..."
            sleep $$((attempt * 15))
          else
            echo "Download failed after 5 attempts"
            exit 1
          fi
        done

        # Download language packs
        for attempt in 1 2 3; do
          sh scripts/download-language-packs.sh && break
          sleep $$((attempt * 10))
        done

  # ── Linux x86_64 build ─────────────────────────────────────────
  - id: 'linux-x86_64'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['setup']
    env:
      - 'SURFER_COMPAT=x86_64'
      - 'SURFER_PLATFORM=linux'
      - 'RUSTUP_TOOLCHAIN=1.83'
      - 'CARGO_INCREMENTAL=0'
    secretEnv: ['GITHUB_TOKEN', 'ZEN_SAFEBROWSING_KEY']
    args:
      - '-c'
      - |
        set -ex
        . "$$HOME/.cargo/env"

        npm run surfer -- ci --brand ${_RELEASE_BRANCH} --display-version ${TAG_NAME}
        npm run import -- --verbose

        # Download NevoFlux Agent
        export GH_TOKEN="$$GITHUB_TOKEN"
        ARCHIVE_NAME="nevoflux-agent-linux-x86_64.tar.gz"
        LATEST_TAG=$$(gh release view --repo dorisgyl/nevoflux-agent --json tagName --jq '.tagName' 2>/dev/null || true)
        if [ -n "$$LATEST_TAG" ]; then
          mkdir -p /tmp/agent build/AppDir/distribution/bin/
          gh release download "$$LATEST_TAG" --repo dorisgyl/nevoflux-agent \
            --pattern "$$ARCHIVE_NAME" --dir /tmp/agent/ --clobber || true
          if [ -f "/tmp/agent/$$ARCHIVE_NAME" ]; then
            tar -xzf "/tmp/agent/$$ARCHIVE_NAME" -C /tmp/agent/
            mv /tmp/agent/nevoflux-agent build/AppDir/distribution/bin/nevoflux-agent
            chmod +x build/AppDir/distribution/bin/nevoflux-agent
            [ -d "/tmp/agent/models" ] && cp -r /tmp/agent/models build/AppDir/distribution/bin/models
          fi
        fi

        # Copy soul templates
        mkdir -p build/AppDir/distribution/bin/defaults/soul
        [ -d "docs/reference/templates" ] && cp docs/reference/templates/*.md build/AppDir/distribution/bin/defaults/soul/

        # Safe Browsing key
        mkdir -p ~/.zen-keys
        echo "$$ZEN_SAFEBROWSING_KEY" > ~/.zen-keys/safebrowsing.dat

        # Bootstrap
        cd engine
        ./mach --no-interactive bootstrap --application-choice browser
        cd ..

        # Build
        rm -rf engine/obj-x86_64-pc-linux-gnu/
        bash .github/workflows/src/release-build.sh

        # Package
        export ZEN_GA_DISABLE_PGO=true
        export ZEN_RELEASE=1
        npm run package

        # Inject distribution
        OBJ_DIR="engine/obj-x86_64-pc-linux-gnu"
        STAGING_DIR="$$OBJ_DIR/dist/nevoflux"
        if [ -d "build/AppDir/distribution" ] && [ -d "$$STAGING_DIR" ]; then
          mkdir -p "$$STAGING_DIR/distribution"
          cp -r build/AppDir/distribution/* "$$STAGING_DIR/distribution/"
          rm -f dist/nevoflux-*.tar.xz
          tar -cJf "dist/nevoflux-repackaged.linux-x86_64.tar.xz" -C "$$OBJ_DIR/dist" nevoflux
        fi

        # Stage artifacts
        mkdir -p /workspace/artifacts
        mv dist/nevoflux-*.tar.xz /workspace/artifacts/nevoflux.linux-x86_64.tar.xz
        mv dist/output.mar /workspace/artifacts/linux.mar
        cp -r dist/update /workspace/linux_update_manifest_x86_64

        rm -rf ~/.zen-keys

  # ── Linux aarch64 build ────────────────────────────────────────
  - id: 'linux-aarch64'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['linux-x86_64']
    env:
      - 'SURFER_COMPAT=aarch64'
      - 'SURFER_PLATFORM=linux'
      - 'RUSTUP_TOOLCHAIN=1.83'
      - 'CARGO_INCREMENTAL=0'
    secretEnv: ['GITHUB_TOKEN', 'ZEN_SAFEBROWSING_KEY']
    args:
      - '-c'
      - |
        set -ex
        . "$$HOME/.cargo/env"

        npm run surfer -- ci --brand ${_RELEASE_BRANCH} --display-version ${TAG_NAME}
        SURFER_COMPAT=aarch64 npm run import -- --verbose

        # Download NevoFlux Agent (aarch64)
        export GH_TOKEN="$$GITHUB_TOKEN"
        ARCHIVE_NAME="nevoflux-agent-linux-aarch64.tar.gz"
        LATEST_TAG=$$(gh release view --repo dorisgyl/nevoflux-agent --json tagName --jq '.tagName' 2>/dev/null || true)
        if [ -n "$$LATEST_TAG" ]; then
          mkdir -p /tmp/agent build/AppDir/distribution/bin/
          gh release download "$$LATEST_TAG" --repo dorisgyl/nevoflux-agent \
            --pattern "$$ARCHIVE_NAME" --dir /tmp/agent/ --clobber || true
          if [ -f "/tmp/agent/$$ARCHIVE_NAME" ]; then
            tar -xzf "/tmp/agent/$$ARCHIVE_NAME" -C /tmp/agent/
            mv /tmp/agent/nevoflux-agent build/AppDir/distribution/bin/nevoflux-agent
            chmod +x build/AppDir/distribution/bin/nevoflux-agent
            [ -d "/tmp/agent/models" ] && cp -r /tmp/agent/models build/AppDir/distribution/bin/models
          fi
        fi

        # Copy soul templates
        mkdir -p build/AppDir/distribution/bin/defaults/soul
        [ -d "docs/reference/templates" ] && cp docs/reference/templates/*.md build/AppDir/distribution/bin/defaults/soul/

        # Safe Browsing key
        mkdir -p ~/.zen-keys
        echo "$$ZEN_SAFEBROWSING_KEY" > ~/.zen-keys/safebrowsing.dat

        # Bootstrap + Build + Package (same pattern as x86_64)
        cd engine
        ./mach --no-interactive bootstrap --application-choice browser
        cd ..
        rm -rf engine/obj-aarch64-pc-linux-gnu/
        bash .github/workflows/src/release-build.sh
        export ZEN_GA_DISABLE_PGO=true
        export ZEN_RELEASE=1
        npm run package

        # Inject distribution
        OBJ_DIR="engine/obj-aarch64-pc-linux-gnu"
        STAGING_DIR="$$OBJ_DIR/dist/nevoflux"
        if [ -d "build/AppDir/distribution" ] && [ -d "$$STAGING_DIR" ]; then
          mkdir -p "$$STAGING_DIR/distribution"
          cp -r build/AppDir/distribution/* "$$STAGING_DIR/distribution/"
          rm -f dist/nevoflux-*.tar.xz
          tar -cJf "dist/nevoflux-repackaged.linux-aarch64.tar.xz" -C "$$OBJ_DIR/dist" nevoflux
        fi

        # Stage artifacts
        mv dist/nevoflux-*.tar.xz /workspace/artifacts/nevoflux.linux-aarch64.tar.xz
        mv dist/output.mar /workspace/artifacts/linux-aarch64.mar
        cp -r dist/update /workspace/linux_update_manifest_aarch64

        rm -rf ~/.zen-keys

  # ── AppImage x86_64 ────────────────────────────────────────────
  - id: 'appimage-x86_64'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['linux-x86_64']
    secretEnv: ['GITHUB_TOKEN']
    args:
      - '-c'
      - |
        set -eux
        export ARCH=x86_64
        UPINFO="gh-releases-zsync|dorisgyl|desktop|latest|nevoflux-$$ARCH.AppImage.zsync"

        npm ci

        # Extract Linux build
        tar -xvf /workspace/artifacts/nevoflux.linux-x86_64.tar.xz
        rm -rf build/AppDir/.DirIcon || true
        cp configs/branding/release/logo128.png build/AppDir/usr/share/icons/hicolor/128x128/apps/zen.png
        cp configs/branding/release/logo128.png build/AppDir/zen.png && ln -s zen.png build/AppDir/.DirIcon

        APPDIR=build/AppDir
        cp -a nevoflux/* $$APPDIR/ && rm -rf nevoflux

        wget "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
        wget "https://github.com/VHSgunzo/uruntime/releases/latest/download/uruntime-appimage-squashfs-lite-$$ARCH"
        chmod +x *.AppImage ./uruntime-appimage-squashfs-lite-"$$ARCH" ./build/AppDir/AppRun
        sed -i 's|URUNTIME_MOUNT=[0-9]|URUNTIME_MOUNT=0|' ./uruntime-appimage-squashfs-lite-"$$ARCH"

        ./appimagetool-x86_64.AppImage -u "$$UPINFO" "$$APPDIR" nevoflux-"$$ARCH".AppImage --runtime-file ./uruntime-appimage-squashfs-lite-"$$ARCH"

        mv nevoflux-$$ARCH.AppImage /workspace/artifacts/
        mv nevoflux-$$ARCH.AppImage.zsync /workspace/artifacts/ || true

  # ── AppImage aarch64 ───────────────────────────────────────────
  - id: 'appimage-aarch64'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['linux-aarch64']
    secretEnv: ['GITHUB_TOKEN']
    args:
      - '-c'
      - |
        set -eux
        export ARCH=aarch64
        UPINFO="gh-releases-zsync|dorisgyl|desktop|latest|nevoflux-$$ARCH.AppImage.zsync"

        npm ci

        tar -xvf /workspace/artifacts/nevoflux.linux-aarch64.tar.xz
        rm -rf build/AppDir/.DirIcon || true
        cp configs/branding/release/logo128.png build/AppDir/usr/share/icons/hicolor/128x128/apps/zen.png
        cp configs/branding/release/logo128.png build/AppDir/zen.png && ln -s zen.png build/AppDir/.DirIcon

        APPDIR=build/AppDir
        cp -a nevoflux/* $$APPDIR/ && rm -rf nevoflux

        wget "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
        wget "https://github.com/VHSgunzo/uruntime/releases/latest/download/uruntime-appimage-squashfs-lite-$$ARCH"
        chmod +x *.AppImage ./uruntime-appimage-squashfs-lite-"$$ARCH" ./build/AppDir/AppRun
        sed -i 's|URUNTIME_MOUNT=[0-9]|URUNTIME_MOUNT=0|' ./uruntime-appimage-squashfs-lite-"$$ARCH"

        ./appimagetool-x86_64.AppImage -u "$$UPINFO" "$$APPDIR" nevoflux-"$$ARCH".AppImage --runtime-file ./uruntime-appimage-squashfs-lite-"$$ARCH"

        mv nevoflux-$$ARCH.AppImage /workspace/artifacts/
        mv nevoflux-$$ARCH.AppImage.zsync /workspace/artifacts/ || true

  # ── Upload all Linux artifacts to GitHub draft release ─────────
  - id: 'upload-linux'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['appimage-x86_64', 'appimage-aarch64']
    secretEnv: ['GITHUB_TOKEN']
    args:
      - '-c'
      - |
        set -ex
        chmod +x cloudbuild/scripts/upload-to-release.sh
        bash cloudbuild/scripts/upload-to-release.sh \
          "${TAG_NAME}" "${_GITHUB_REPO}" "$$GITHUB_TOKEN"

        # Also upload update manifests to GCS for the release job
        gsutil -m cp -r /workspace/linux_update_manifest_x86_64 gs://nevoflux-builds/${TAG_NAME}/
        gsutil -m cp -r /workspace/linux_update_manifest_aarch64 gs://nevoflux-builds/${TAG_NAME}/
```

**Step 2: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('cloudbuild/linux-build.yaml'))"`

**Step 3: Commit**

```bash
git add -f cloudbuild/linux-build.yaml
git commit -m "build(gcb): add Linux Cloud Build config"
```

---

## Task 4: Create Windows Cloud Build Config

**Files:**
- Create: `cloudbuild/windows-build.yaml`

**Step 1: Write windows-build.yaml**

This config handles all Windows builds: PGO stage 1, waiting for PGO data from GHA, PGO stage 3 (final), and aarch64. Note: Windows builds are cross-compiled from Linux using Wine + VS2022 tools. These tools are pre-installed in the Docker image.

```yaml
# Windows browser builds for NevoFlux (cross-compiled from Linux)
# Triggered by version tag push
# PGO flow: stage 1 -> upload to GCS -> wait for GHA stage 2 -> stage 3
# Uploads artifacts to GitHub draft release

timeout: '18000s'  # 5 hours

options:
  machineType: 'E2_HIGHCPU_32'
  diskSizeGb: 200
  logging: CLOUD_LOGGING_ONLY

availableSecrets:
  secretManager:
    - versionName: projects/$PROJECT_ID/secrets/github-deploy-token/versions/latest
      env: 'GITHUB_TOKEN'
    - versionName: projects/$PROJECT_ID/secrets/zen-safebrowsing-key/versions/latest
      env: 'ZEN_SAFEBROWSING_KEY'

substitutions:
  _RELEASE_BRANCH: 'release'
  _GITHUB_REPO: 'dorisgyl/nevoflux'

steps:
  # ── Setup ──────────────────────────────────────────────────────
  - id: 'setup'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    args:
      - '-c'
      - |
        set -ex
        git config --global user.email "nevoflux-bot@users.noreply.github.com"
        git config --global user.name "NevoFlux Bot"
        npm ci
        npm run surfer -- ci --brand ${_RELEASE_BRANCH} --display-version ${TAG_NAME}
        for attempt in 1 2 3 4 5; do
          npm run download --verbose && break
          sleep $$((attempt * 15))
        done
        for attempt in 1 2 3; do
          sh scripts/download-language-packs.sh && break
          sleep $$((attempt * 10))
        done

  # ── Windows Setup (Wine + VS2022) ─────────────────────────────
  - id: 'windows-setup'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['setup']
    args:
      - '-c'
      - |
        set -ex
        # Wine + VS2022 are pre-installed in the Docker image at ~/win-cross/
        # If not present, download them (fallback)
        if [ ! -d ~/win-cross/wine ] || [ ! -d ~/win-cross/vs2022 ]; then
          mkdir -p ~/win-cross
          cd engine/
          aria2c "https://firefox-ci-tc.services.mozilla.com/api/index/v1/task/gecko.cache.level-1.toolchains.v3.linux64-wine.latest/artifacts/public%2Fbuild%2Fwine.tar.zst" -o wine.tar.zst
          tar --zstd -xf wine.tar.zst -C ~/win-cross
          rm wine.tar.zst
          ./mach python --virtualenv build taskcluster/scripts/misc/get_vs.py build/vs/vs2022.yaml ~/win-cross/vs2022
          cd ..
        fi

  # ── Windows PGO Stage 1 (x86_64) ──────────────────────────────
  - id: 'windows-pgo-stage1'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['windows-setup']
    env:
      - 'SURFER_COMPAT=x86_64'
      - 'SURFER_PLATFORM=win32'
      - 'ZEN_CROSS_COMPILING=1'
      - 'ZEN_GA_GENERATE_PROFILE=1'
      - 'RUSTUP_TOOLCHAIN=1.83'
      - 'CARGO_INCREMENTAL=0'
    secretEnv: ['GITHUB_TOKEN', 'ZEN_SAFEBROWSING_KEY']
    args:
      - '-c'
      - |
        set -ex
        . "$$HOME/.cargo/env"

        npm run surfer -- ci --brand ${_RELEASE_BRANCH} --display-version ${TAG_NAME}
        dos2unix configs/windows/mozconfig
        npm run import -- --verbose

        # Download agent binary (Windows)
        export GH_TOKEN="$$GITHUB_TOKEN"
        ARCHIVE_NAME="nevoflux-agent-windows-x86_64.zip"
        LATEST_TAG=$$(gh release view --repo dorisgyl/nevoflux-agent --json tagName --jq '.tagName' 2>/dev/null || true)
        if [ -n "$$LATEST_TAG" ]; then
          mkdir -p /tmp/agent build/AppDir/distribution/bin/
          gh release download "$$LATEST_TAG" --repo dorisgyl/nevoflux-agent \
            --pattern "$$ARCHIVE_NAME" --dir /tmp/agent/ --clobber || true
          if [ -f "/tmp/agent/$$ARCHIVE_NAME" ]; then
            cd /tmp/agent && unzip -o "$$ARCHIVE_NAME" && cd -
            mv /tmp/agent/nevoflux-agent.exe build/AppDir/distribution/bin/nevoflux-agent.exe || \
              mv /tmp/agent/nevoflux-agent build/AppDir/distribution/bin/nevoflux-agent.exe || true
            [ -d "/tmp/agent/models" ] && cp -r /tmp/agent/models build/AppDir/distribution/bin/models
          fi
        fi

        mkdir -p build/AppDir/distribution/bin/defaults/soul
        [ -d "docs/reference/templates" ] && cp docs/reference/templates/*.md build/AppDir/distribution/bin/defaults/soul/

        mkdir -p ~/.zen-keys
        echo "$$ZEN_SAFEBROWSING_KEY" > ~/.zen-keys/safebrowsing.dat

        # Bootstrap
        cd engine/
        chmod -R +x "$$(echo ~)/win-cross/vs2022" || true
        cd ..
        npm run bootstrap
        cd engine/
        echo "export LIB=\"$$(cd ~/.mozbuild/clang/lib/clang/* && cd lib/windows && pwd)\"" >> ../configs/common/mozconfig
        cd ..

        # Setup Rust for Windows
        cargo install cargo-download --locked || true
        cd engine
        cargo download -x windows=0.58.0 || true
        echo "export MOZ_WINDOWS_RS_DIR=$$(pwd)/windows-0.58.0" >> ../configs/common/mozconfig
        cd ..

        # Build (PGO stage 1)
        rm -rf engine/obj-x86_64-pc-windows-msvc/
        bash .github/workflows/src/release-build.sh

        # Package
        export ZEN_GA_DISABLE_PGO=true
        export ZEN_RELEASE=1
        npm run package

        # Inject distribution + create zip
        VERSION=$$(npm run --silent surfer -- get version | xargs)
        ZIP_NAME="nevoflux-$${VERSION}.en-US.win64.zip"
        if [ -d "build/AppDir/distribution" ] && [ -f "dist/$${ZIP_NAME}" ]; then
          TOP_DIR=$$(unzip -Z1 "dist/$${ZIP_NAME}" | head -1 | cut -d/ -f1)
          STAGING=$$(mktemp -d)
          mkdir -p "$$STAGING/$$TOP_DIR/distribution"
          cp -r build/AppDir/distribution/* "$$STAGING/$$TOP_DIR/distribution/"
          cd "$$STAGING" && zip -r "$$OLDPWD/dist/$${ZIP_NAME}" "$$TOP_DIR/distribution/" && cd "$$OLDPWD"
          rm -rf "$$STAGING"
        fi
        mv "./dist/$${ZIP_NAME}" /workspace/pgo-stage1.zip

        rm -rf ~/.zen-keys

  # ── Upload PGO stage 1 to GCS ─────────────────────────────────
  - id: 'upload-pgo-stage1'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['windows-pgo-stage1']
    args:
      - '-c'
      - |
        set -ex
        gsutil cp /workspace/pgo-stage1.zip gs://nevoflux-builds/${TAG_NAME}/pgo-stage-1.zip
        echo "PGO stage 1 uploaded to GCS."

  # ── Wait for PGO stage 2 data (from GitHub Actions) ───────────
  - id: 'wait-for-pgo'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['upload-pgo-stage1']
    args:
      - '-c'
      - |
        chmod +x cloudbuild/scripts/wait-for-pgo.sh
        bash cloudbuild/scripts/wait-for-pgo.sh "${TAG_NAME}"

  # ── Windows Final Build (x86_64 with PGO data) ────────────────
  - id: 'windows-x86_64-final'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['wait-for-pgo']
    env:
      - 'SURFER_COMPAT=x86_64'
      - 'SURFER_PLATFORM=win32'
      - 'ZEN_CROSS_COMPILING=1'
      - 'RUSTUP_TOOLCHAIN=1.83'
      - 'CARGO_INCREMENTAL=0'
    secretEnv: ['GITHUB_TOKEN', 'ZEN_SAFEBROWSING_KEY']
    args:
      - '-c'
      - |
        set -ex
        . "$$HOME/.cargo/env"

        npm run surfer -- ci --brand ${_RELEASE_BRANCH} --display-version ${TAG_NAME}
        dos2unix configs/windows/mozconfig
        npm run import -- --verbose

        # Place PGO profile data
        mkdir -p ~/artifact
        cp /workspace/merged.profdata ~/artifact/merged.profdata
        cp /workspace/en-US.log ~/artifact/en-US.log
        chmod +x ~/artifact/en-US.log ~/artifact/merged.profdata

        # Agent + soul templates + API keys (same as stage 1)
        export GH_TOKEN="$$GITHUB_TOKEN"
        ARCHIVE_NAME="nevoflux-agent-windows-x86_64.zip"
        LATEST_TAG=$$(gh release view --repo dorisgyl/nevoflux-agent --json tagName --jq '.tagName' 2>/dev/null || true)
        if [ -n "$$LATEST_TAG" ]; then
          mkdir -p /tmp/agent build/AppDir/distribution/bin/
          gh release download "$$LATEST_TAG" --repo dorisgyl/nevoflux-agent \
            --pattern "$$ARCHIVE_NAME" --dir /tmp/agent/ --clobber || true
          if [ -f "/tmp/agent/$$ARCHIVE_NAME" ]; then
            cd /tmp/agent && unzip -o "$$ARCHIVE_NAME" && cd -
            mv /tmp/agent/nevoflux-agent.exe build/AppDir/distribution/bin/nevoflux-agent.exe || \
              mv /tmp/agent/nevoflux-agent build/AppDir/distribution/bin/nevoflux-agent.exe || true
            [ -d "/tmp/agent/models" ] && cp -r /tmp/agent/models build/AppDir/distribution/bin/models
          fi
        fi
        mkdir -p build/AppDir/distribution/bin/defaults/soul
        [ -d "docs/reference/templates" ] && cp docs/reference/templates/*.md build/AppDir/distribution/bin/defaults/soul/
        mkdir -p ~/.zen-keys
        echo "$$ZEN_SAFEBROWSING_KEY" > ~/.zen-keys/safebrowsing.dat

        # Re-bootstrap + setup (configs may be lost between steps)
        cd engine/
        chmod -R +x "$$(echo ~)/win-cross/vs2022" || true
        cd ..
        npm run bootstrap
        cd engine/
        echo "export LIB=\"$$(cd ~/.mozbuild/clang/lib/clang/* && cd lib/windows && pwd)\"" >> ../configs/common/mozconfig
        cargo download -x windows=0.58.0 || true
        echo "export MOZ_WINDOWS_RS_DIR=$$(pwd)/windows-0.58.0" >> ../configs/common/mozconfig
        cd ..

        # Build (final, with PGO data)
        rm -rf engine/obj-x86_64-pc-windows-msvc/
        bash .github/workflows/src/release-build.sh

        # Package
        export ZEN_GA_DISABLE_PGO=true
        export ZEN_RELEASE=1
        npm run package

        # Inject distribution + rename
        VERSION=$$(npm run --silent surfer -- get version | xargs)
        ZIP_NAME="nevoflux-$${VERSION}.en-US.win64.zip"
        if [ -d "build/AppDir/distribution" ] && [ -f "dist/$${ZIP_NAME}" ]; then
          TOP_DIR=$$(unzip -Z1 "dist/$${ZIP_NAME}" | head -1 | cut -d/ -f1)
          STAGING=$$(mktemp -d)
          mkdir -p "$$STAGING/$$TOP_DIR/distribution"
          cp -r build/AppDir/distribution/* "$$STAGING/$$TOP_DIR/distribution/"
          cd "$$STAGING" && zip -r "$$OLDPWD/dist/$${ZIP_NAME}" "$$TOP_DIR/distribution/" && cd "$$OLDPWD"
          rm -rf "$$STAGING"
        fi

        mkdir -p /workspace/artifacts
        mv "./dist/$${ZIP_NAME}" /workspace/artifacts/nevoflux.win-x86_64.zip
        mv ./dist/output.mar /workspace/artifacts/windows.mar
        mv ./dist/nevoflux.installer.exe /workspace/artifacts/nevoflux.installer.exe

        rm -rf ~/.zen-keys

  # ── Windows aarch64 build (no PGO) ────────────────────────────
  - id: 'windows-aarch64'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['windows-x86_64-final']
    env:
      - 'SURFER_COMPAT=aarch64'
      - 'SURFER_PLATFORM=win32'
      - 'ZEN_CROSS_COMPILING=1'
      - 'RUSTUP_TOOLCHAIN=1.83'
      - 'CARGO_INCREMENTAL=0'
    secretEnv: ['GITHUB_TOKEN', 'ZEN_SAFEBROWSING_KEY']
    args:
      - '-c'
      - |
        set -ex
        . "$$HOME/.cargo/env"

        npm run surfer -- ci --brand ${_RELEASE_BRANCH} --display-version ${TAG_NAME}
        dos2unix configs/windows/mozconfig
        SURFER_COMPAT=aarch64 npm run import -- --verbose

        # Agent + soul templates + API keys
        export GH_TOKEN="$$GITHUB_TOKEN"
        ARCHIVE_NAME="nevoflux-agent-windows-aarch64.zip"
        LATEST_TAG=$$(gh release view --repo dorisgyl/nevoflux-agent --json tagName --jq '.tagName' 2>/dev/null || true)
        if [ -n "$$LATEST_TAG" ]; then
          mkdir -p /tmp/agent build/AppDir/distribution/bin/
          gh release download "$$LATEST_TAG" --repo dorisgyl/nevoflux-agent \
            --pattern "$$ARCHIVE_NAME" --dir /tmp/agent/ --clobber || true
          if [ -f "/tmp/agent/$$ARCHIVE_NAME" ]; then
            cd /tmp/agent && unzip -o "$$ARCHIVE_NAME" && cd -
            mv /tmp/agent/nevoflux-agent.exe build/AppDir/distribution/bin/nevoflux-agent.exe || \
              mv /tmp/agent/nevoflux-agent build/AppDir/distribution/bin/nevoflux-agent.exe || true
            [ -d "/tmp/agent/models" ] && cp -r /tmp/agent/models build/AppDir/distribution/bin/models
          fi
        fi
        mkdir -p build/AppDir/distribution/bin/defaults/soul
        [ -d "docs/reference/templates" ] && cp docs/reference/templates/*.md build/AppDir/distribution/bin/defaults/soul/
        mkdir -p ~/.zen-keys
        echo "$$ZEN_SAFEBROWSING_KEY" > ~/.zen-keys/safebrowsing.dat

        # Bootstrap + Rust setup
        cd engine/
        chmod -R +x "$$(echo ~)/win-cross/vs2022" || true
        cd ..
        npm run bootstrap
        cd engine/
        echo "export LIB=\"$$(cd ~/.mozbuild/clang/lib/clang/* && cd lib/windows && pwd)\"" >> ../configs/common/mozconfig
        cargo download -x windows=0.58.0 || true
        echo "export MOZ_WINDOWS_RS_DIR=$$(pwd)/windows-0.58.0" >> ../configs/common/mozconfig
        cd ..

        # Build + Package
        rm -rf engine/obj-aarch64-pc-windows-msvc/
        bash .github/workflows/src/release-build.sh
        export ZEN_GA_DISABLE_PGO=true
        export ZEN_RELEASE=1
        npm run package

        # Inject distribution + rename
        VERSION=$$(npm run --silent surfer -- get version | xargs)
        ZIP_NAME="nevoflux-$${VERSION}.en-US.win64-aarch64.zip"
        if [ -d "build/AppDir/distribution" ] && [ -f "dist/$${ZIP_NAME}" ]; then
          TOP_DIR=$$(unzip -Z1 "dist/$${ZIP_NAME}" | head -1 | cut -d/ -f1)
          STAGING=$$(mktemp -d)
          mkdir -p "$$STAGING/$$TOP_DIR/distribution"
          cp -r build/AppDir/distribution/* "$$STAGING/$$TOP_DIR/distribution/"
          cd "$$STAGING" && zip -r "$$OLDPWD/dist/$${ZIP_NAME}" "$$TOP_DIR/distribution/" && cd "$$OLDPWD"
          rm -rf "$$STAGING"
        fi

        mv "./dist/$${ZIP_NAME}" /workspace/artifacts/nevoflux.win-arm64.zip
        mv ./dist/output.mar /workspace/artifacts/windows-arm64.mar
        mv ./dist/nevoflux.installer.exe /workspace/artifacts/nevoflux.installer-arm64.exe

        rm -rf ~/.zen-keys

  # ── Upload all Windows artifacts to GitHub draft release ───────
  - id: 'upload-windows'
    name: 'gcr.io/$PROJECT_ID/nevoflux-builder:latest'
    entrypoint: 'bash'
    waitFor: ['windows-x86_64-final', 'windows-aarch64']
    secretEnv: ['GITHUB_TOKEN']
    args:
      - '-c'
      - |
        set -ex
        chmod +x cloudbuild/scripts/upload-to-release.sh
        bash cloudbuild/scripts/upload-to-release.sh \
          "${TAG_NAME}" "${_GITHUB_REPO}" "$$GITHUB_TOKEN"
```

**Step 2: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('cloudbuild/windows-build.yaml'))"`

**Step 3: Commit**

```bash
git add -f cloudbuild/windows-build.yaml
git commit -m "build(gcb): add Windows Cloud Build config"
```

---

## Task 5: Create wait-for-gcb.yml Reusable Workflow

**Files:**
- Create: `.github/workflows/wait-for-gcb.yml`

**Step 1: Write the reusable workflow**

This workflow polls the GitHub API for expected draft release assets uploaded by Cloud Build.

```yaml
name: Wait for Cloud Build Artifacts

on:
  workflow_call:
    inputs:
      version:
        description: 'Release version tag to check'
        required: true
        type: string
      timeout-minutes:
        description: 'Maximum wait time in minutes'
        required: false
        type: number
        default: 180
    outputs:
      ready:
        description: 'Whether all artifacts are ready'
        value: ${{ jobs.wait.outputs.ready }}

jobs:
  wait:
    runs-on: ubuntu-latest
    outputs:
      ready: ${{ steps.poll.outputs.ready }}
    steps:
      - name: Poll for draft release assets
        id: poll
        env:
          GH_TOKEN: ${{ secrets.DEPLOY_KEY }}
          VERSION: ${{ inputs.version }}
          TIMEOUT: ${{ inputs.timeout-minutes }}
          REPO: ${{ github.repository }}
        run: |
          EXPECTED_ASSETS=(
            "nevoflux.linux-x86_64.tar.xz"
            "nevoflux.linux-aarch64.tar.xz"
            "nevoflux.win-x86_64.zip"
            "nevoflux.win-arm64.zip"
            "linux.mar"
            "linux-aarch64.mar"
            "windows.mar"
            "windows-arm64.mar"
            "nevoflux.installer.exe"
            "nevoflux.installer-arm64.exe"
            "nevoflux-x86_64.AppImage"
            "nevoflux-aarch64.AppImage"
          )

          INTERVAL=60
          ELAPSED=0
          MAX_SECONDS=$(( TIMEOUT * 60 ))

          echo "Waiting for ${#EXPECTED_ASSETS[@]} artifacts on release $VERSION ..."

          while [ $ELAPSED -lt $MAX_SECONDS ]; do
            ASSETS=$(gh release view "$VERSION" --repo "$REPO" --json assets --jq '.assets[].name' 2>/dev/null || echo "")

            MISSING=0
            for expected in "${EXPECTED_ASSETS[@]}"; do
              if ! echo "$ASSETS" | grep -qx "$expected"; then
                MISSING=$((MISSING + 1))
              fi
            done

            if [ $MISSING -eq 0 ]; then
              echo "All ${#EXPECTED_ASSETS[@]} artifacts found."
              echo "ready=true" >> $GITHUB_OUTPUT
              exit 0
            fi

            echo "[$((ELAPSED / 60))m] Missing $MISSING artifacts. Waiting..."
            sleep $INTERVAL
            ELAPSED=$((ELAPSED + INTERVAL))
          done

          echo "Timed out after ${TIMEOUT} minutes. Missing $MISSING artifacts."
          echo "ready=false" >> $GITHUB_OUTPUT
          exit 1
```

**Step 2: Commit**

```bash
git add .github/workflows/wait-for-gcb.yml
git commit -m "build(gcb): add reusable workflow to wait for Cloud Build artifacts"
```

---

## Task 6: Modify windows-profile-build.yml for GCS Artifact Exchange

**Files:**
- Modify: `.github/workflows/windows-profile-build.yml`

**Step 1: Read the current file**

Refer to: `.github/workflows/windows-profile-build.yml` (already read above)

**Step 2: Replace GHA artifact download/upload with GCS operations**

Key changes:
- Remove `actions/download-artifact@v4` step — replace with `gsutil cp` from GCS
- Remove `actions/upload-artifact@v4` step — replace with `gsutil cp` to GCS + done-marker
- Add `google-github-actions/auth` step for GCS authentication
- Add new input: `build-version` is used as the GCS path key

Replace the "Download artifact" step (lines 73-79) with:

```yaml
      - name: Authenticate to Google Cloud
        if: ${{ matrix.arch == 'x86_64' }}
        uses: google-github-actions/auth@v2
        with:
          credentials_json: ${{ secrets.GCS_SERVICE_ACCOUNT_KEY }}

      - name: Setup Google Cloud SDK
        if: ${{ matrix.arch == 'x86_64' }}
        uses: google-github-actions/setup-gcloud@v2

      - name: Download PGO stage 1 from GCS
        if: ${{ matrix.arch == 'x86_64' }}
        run: |
          mkdir -p C:\artifact
          gsutil cp gs://nevoflux-builds/${{ inputs.build-version }}/pgo-stage-1.zip C:\artifact\${{ inputs.profile-data-path-archive }}
```

Replace the "Upload artifacts" step (lines 158-167) with:

```yaml
      - name: Upload profile data to GCS
        if: ${{ matrix.arch == 'x86_64' }}
        run: |
          gsutil cp merged.profdata gs://nevoflux-builds/${{ inputs.build-version }}/merged.profdata
          gsutil cp en-US.log gs://nevoflux-builds/${{ inputs.build-version }}/en-US.log
          echo "done" > done-marker
          gsutil cp done-marker gs://nevoflux-builds/${{ inputs.build-version }}/done-marker
```

**Step 3: Commit**

```bash
git add .github/workflows/windows-profile-build.yml
git commit -m "build(gcb): switch windows-profile-build to use GCS for PGO artifact exchange"
```

---

## Task 7: Modify build.yml Main Orchestrator

**Files:**
- Modify: `.github/workflows/build.yml`

This is the largest change. Key modifications:

**Step 1: Remove jobs that moved to GCB**

Delete these job blocks from `build.yml`:
- `linux` (lines 381-393) — moved to `cloudbuild/linux-build.yaml`
- `windows-step-1` (lines 340-353) — moved to `cloudbuild/windows-build.yaml`
- `windows-step-3` (lines 365-379) — moved to `cloudbuild/windows-build.yaml`
- `appimage` (lines 419-511) — moved to `cloudbuild/linux-build.yaml`

**Step 2: Modify windows-step-2 to use GCS**

The `windows-step-2` job currently depends on `windows-step-1` (GHA artifact). Change it to:
- Remove `needs: [windows-step-1, build-data]` — change to `needs: [build-data]`
- It no longer waits for a GHA job; GCS polling happens in the job itself via `gsutil`
- Add GCS auth steps and poll for PGO stage 1 zip from GCS

```yaml
  windows-step-2:
    name: Windows build step 2 (Generate profile data)
    uses: ./.github/workflows/windows-profile-build.yml
    permissions:
      contents: write
    secrets: inherit
    needs: [build-data]
    with:
      build-version: ${{ needs.build-data.outputs.version }}
      profile-data-path-archive: nevoflux-windows-profile-data-and-jarlog.zip
      release-branch: ${{ inputs.update_branch }}
```

**Step 3: Add wait-for-gcb job and modify release job**

Add a new job before `release`:

```yaml
  wait-gcb:
    name: Wait for Cloud Build artifacts
    uses: ./.github/workflows/wait-for-gcb.yml
    needs: [build-data]
    secrets: inherit
    with:
      version: ${{ needs.build-data.outputs.version }}
      timeout-minutes: 180
```

Modify the `release` job:
- Change `needs` to: `[build-data, wait-gcb, mac-uni, source, lint]`
- Remove references to GHA artifacts for Linux/Windows (they're now on the draft release)
- The release job now:
  1. Downloads macOS artifacts from GHA artifacts
  2. Downloads update manifests from GCS
  3. Finalizes the draft release (remove draft flag, add release notes, attach remaining macOS artifacts)

**Step 4: Update release job artifact handling**

Replace the `actions/download-artifact@v4` (which downloads all GHA artifacts) with selective downloads for macOS-only artifacts, then add GCS downloads for update manifests:

```yaml
      - name: Download macOS artifacts
        uses: actions/download-artifact@v4
        with:
          pattern: 'nevoflux-*-apple-darwin*'
          merge-multiple: true

      - name: Download macOS universal DMG
        uses: actions/download-artifact@v4
        with:
          name: nevoflux.macos-universal.dmg

      - name: Download macOS MAR
        uses: actions/download-artifact@v4
        with:
          name: macos.mar

      - name: Download source tarball
        uses: actions/download-artifact@v4
        with:
          name: nevoflux.source.tar.zst

      - name: Authenticate to Google Cloud
        uses: google-github-actions/auth@v2
        with:
          credentials_json: ${{ secrets.GCS_SERVICE_ACCOUNT_KEY }}

      - name: Setup Google Cloud SDK
        uses: google-github-actions/setup-gcloud@v2

      - name: Download update manifests from GCS
        run: |
          VERSION="${{ needs.build-data.outputs.version }}"
          gsutil -m cp -r "gs://nevoflux-builds/${VERSION}/linux_update_manifest_x86_64" .
          gsutil -m cp -r "gs://nevoflux-builds/${VERSION}/linux_update_manifest_aarch64" .

      - name: Upload remaining artifacts to release
        env:
          GH_TOKEN: ${{ secrets.DEPLOY_KEY }}
        run: |
          VERSION="${{ needs.build-data.outputs.version }}"
          # Upload macOS + source artifacts (Linux/Windows already uploaded by GCB)
          gh release upload "$VERSION" \
            ./nevoflux.source.tar.zst/* \
            ./nevoflux.macos-universal.dmg/* \
            ./macos.mar/* \
            --clobber || true

      - name: Finalize release
        env:
          GH_TOKEN: ${{ secrets.DEPLOY_KEY }}
        run: |
          VERSION="${{ needs.build-data.outputs.version }}"
          # Mark release as non-draft
          gh release edit "$VERSION" \
            --draft=false \
            --title "Release build - $VERSION (${{ needs.build-data.outputs.build_date }})" \
            --notes-file release_notes.md
```

**Step 5: Commit**

```bash
git add .github/workflows/build.yml
git commit -m "build(gcb): restructure build.yml for Cloud Build integration

Remove linux, windows-step-1, windows-step-3, and appimage jobs.
Add wait-for-gcb job. Modify release to combine GCB and GHA artifacts."
```

---

## Task 8: Update test-runners.yml

**Files:**
- Modify: `.github/workflows/test-runners.yml`

**Step 1: Replace Linux runner test with GCS connectivity test**

Replace the `test-linux` job with a GCS connectivity test. Keep `test-macos` unchanged.

```yaml
name: Test CI Infrastructure
on: workflow_dispatch

jobs:
  test-gcs:
    runs-on: ubuntu-latest
    steps:
      - name: Authenticate to Google Cloud
        uses: google-github-actions/auth@v2
        with:
          credentials_json: ${{ secrets.GCS_SERVICE_ACCOUNT_KEY }}

      - name: Setup Google Cloud SDK
        uses: google-github-actions/setup-gcloud@v2

      - name: Test GCS connectivity
        run: |
          echo "=== GCS Bucket ==="
          gsutil ls gs://nevoflux-builds/ || echo "Bucket empty or not accessible"
          echo "=== Cloud Build ==="
          gcloud builds list --limit=3 --project=${{ secrets.GOOGLE_CLOUD_PROJECT }} || echo "No recent builds"

      - name: Test agent download
        run: |
          gh release list --repo dorisgyl/nevoflux-agent --limit 3 || echo "No releases yet"
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  test-macos:
    runs-on: [self-hosted, macos]
    steps:
      - name: System info
        run: |
          echo "=== OS ==="
          uname -a
          echo "=== CPU ==="
          sysctl -n hw.ncpu
          echo "=== Memory ==="
          sysctl -n hw.memsize | awk '{print $0/1024/1024/1024 " GB"}'
          echo "=== Disk ==="
          df -h /
          echo "=== Node ==="
          node --version
          echo "=== Rust ==="
          rustc --version
          echo "=== Python ==="
          python3 --version

      - name: Test code signing
        run: |
          echo "=== Signing Identities ==="
          security find-identity -v -p codesigning
          echo "=== Keychain List ==="
          security list-keychains
```

**Step 2: Commit**

```bash
git add .github/workflows/test-runners.yml
git commit -m "build(gcb): update test-runners.yml for GCS connectivity testing"
```

---

## Task 9: Delete Old Workflow Files

**Files:**
- Delete: `.github/workflows/linux-release-build.yml`
- Delete: `.github/workflows/windows-release-build.yml`

**Step 1: Delete the files**

```bash
git rm .github/workflows/linux-release-build.yml
git rm .github/workflows/windows-release-build.yml
```

**Step 2: Commit**

```bash
git commit -m "build(gcb): remove linux and windows release build workflows

These are now handled by Cloud Build configs in cloudbuild/"
```

---

## Task 10: Manual GCP Setup Checklist

These steps must be done manually in the GCP Console or via `gcloud` CLI. They cannot be automated via the repo.

**Step 1: Create GCS bucket**

```bash
gcloud storage buckets create gs://nevoflux-builds \
  --project=$PROJECT_ID \
  --location=us-central1 \
  --uniform-bucket-level-access
```

**Step 2: Store secrets in Secret Manager**

```bash
# GitHub PAT with repo + releases permissions
echo -n "$GITHUB_PAT" | gcloud secrets create github-deploy-token \
  --data-file=- --project=$PROJECT_ID

# Safe Browsing API key
echo -n "$SAFEBROWSING_KEY" | gcloud secrets create zen-safebrowsing-key \
  --data-file=- --project=$PROJECT_ID
```

**Step 3: Grant IAM roles to Cloud Build service account**

```bash
PROJECT_NUMBER=$(gcloud projects describe $PROJECT_ID --format='value(projectNumber)')
CB_SA="${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com"

# Storage access
gcloud storage buckets add-iam-policy-binding gs://nevoflux-builds \
  --member="serviceAccount:${CB_SA}" \
  --role="roles/storage.objectAdmin"

# Secret Manager access
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:${CB_SA}" \
  --role="roles/secretmanager.secretAccessor"
```

**Step 4: Create a service account for GitHub Actions**

```bash
gcloud iam service-accounts create nevoflux-gha \
  --display-name="NevoFlux GitHub Actions"

gcloud storage buckets add-iam-policy-binding gs://nevoflux-builds \
  --member="serviceAccount:nevoflux-gha@${PROJECT_ID}.iam.gserviceaccount.com" \
  --role="roles/storage.objectAdmin"

gcloud iam service-accounts keys create gha-key.json \
  --iam-account=nevoflux-gha@${PROJECT_ID}.iam.gserviceaccount.com
```

Add the contents of `gha-key.json` as `GCS_SERVICE_ACCOUNT_KEY` in GitHub Secrets.
Add the project ID as `GOOGLE_CLOUD_PROJECT` in GitHub Secrets.

**Step 5: Build and push the Docker builder image**

```bash
cd cloudbuild
gcloud builds submit --tag gcr.io/$PROJECT_ID/nevoflux-builder:latest .
```

**Step 6: Create Cloud Build triggers**

```bash
# Linux builds trigger
gcloud builds triggers create github \
  --name="nevoflux-linux-build" \
  --repo-name=nevoflux \
  --repo-owner=dorisgyl \
  --tag-pattern="^[0-9]+\.[0-9]+\.[0-9]+" \
  --build-config=cloudbuild/linux-build.yaml

# Windows builds trigger
gcloud builds triggers create github \
  --name="nevoflux-windows-build" \
  --repo-name=nevoflux \
  --repo-owner=dorisgyl \
  --tag-pattern="^[0-9]+\.[0-9]+\.[0-9]+" \
  --build-config=cloudbuild/windows-build.yaml
```

---

## Task 11: End-to-End Test

**Step 1: Trigger a test build**

Create a test tag to trigger both GCB and GHA:

```bash
git tag 0.0.1-gcb-test
git push origin 0.0.1-gcb-test
```

**Step 2: Verify GCB triggers fire**

```bash
gcloud builds list --limit=5 --project=$PROJECT_ID
```

**Step 3: Monitor builds**

- GCB: `gcloud builds log <build-id> --stream`
- GHA: Check GitHub Actions tab

**Step 4: Verify artifact flow**

1. Check GCS bucket has PGO artifacts: `gsutil ls gs://nevoflux-builds/0.0.1-gcb-test/`
2. Check GitHub draft release has all expected assets
3. Verify GHA release job can finalize the release

**Step 5: Clean up test**

```bash
git tag -d 0.0.1-gcb-test
git push origin :refs/tags/0.0.1-gcb-test
gh release delete 0.0.1-gcb-test --yes
gsutil -m rm -r gs://nevoflux-builds/0.0.1-gcb-test/
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Docker builder image | `cloudbuild/Dockerfile` |
| 2 | Helper scripts | `cloudbuild/scripts/wait-for-pgo.sh`, `upload-to-release.sh` |
| 3 | Linux Cloud Build config | `cloudbuild/linux-build.yaml` |
| 4 | Windows Cloud Build config | `cloudbuild/windows-build.yaml` |
| 5 | wait-for-gcb reusable workflow | `.github/workflows/wait-for-gcb.yml` |
| 6 | Modify windows-profile-build | `.github/workflows/windows-profile-build.yml` |
| 7 | Modify build.yml orchestrator | `.github/workflows/build.yml` |
| 8 | Update test-runners.yml | `.github/workflows/test-runners.yml` |
| 9 | Delete old workflows | Remove `linux-release-build.yml`, `windows-release-build.yml` |
| 10 | Manual GCP setup | Console/gcloud commands |
| 11 | End-to-end test | Tag push + verification |
