#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

require_command() {
  local cmd="$1"
  local hint="$2"

  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    echo "Install hint: $hint" >&2
    exit 1
  fi
}

check_dependencies() {
  require_command python3 "Install Python 3 and rerun the NevoFlux bootstrap if needed."
  require_command npm "Install Node.js/npm."
  require_command node "Install Node.js."
  require_command cargo "Install Rust via rustup."
  require_command rustc "Install Rust via rustup."
  require_command trunk "Install Trunk with: cargo install trunk"
  require_command zip "Install zip."
  require_command unzip "Install unzip."

  if [[ ! -x engine/mach ]]; then
    echo "Missing engine/mach. Run the repository setup/bootstrap first." >&2
    exit 1
  fi

  if [[ ! -f native/nevoflux-agent/Cargo.toml ]]; then
    echo "Missing native/nevoflux-agent/Cargo.toml. The native agent is expected in the monorepo." >&2
    exit 1
  fi
}

find_obj_dir() {
  find engine -maxdepth 1 -type d -name 'obj-*' | sort | head -n 1
}

find_dist_dir() {
  local obj_dir="$1"
  local app_bundle

  if [[ -d "$obj_dir/dist/bin" ]]; then
    echo "$obj_dir/dist/bin"
    return 0
  fi

  app_bundle="$(find "$obj_dir/dist" -maxdepth 1 -type d -name '*.app' 2>/dev/null | head -n 1)"
  if [[ -n "$app_bundle" ]]; then
    echo "$app_bundle/Contents/Resources"
    return 0
  fi

  return 1
}

write_native_host_manifest() {
  local agent_path="$1"
  local host_dir="$HOME/.mozilla/native-messaging-hosts"

  mkdir -p "$host_dir"

  for host in com.nevoflux.agent com.nevoflux.agent.mcp; do
    local description="NevoFlux AI Agent"
    if [[ "$host" == "com.nevoflux.agent.mcp" ]]; then
      description="NevoFlux MCP Agent"
    fi

    printf '{"name":"%s","description":"%s","path":"%s","type":"stdio","allowed_extensions":["agent@nevoflux.com"]}\n' \
      "$host" "$description" "$agent_path" > "$host_dir/$host.json"
  done
}

check_dependencies

echo "Building NevoFlux browser from current source..."
(cd engine && python3 ./mach build)

OBJ_DIR="$(find_obj_dir)"
if [[ -z "$OBJ_DIR" ]]; then
  echo "No engine/obj-* build directory found after mach build." >&2
  exit 1
fi

DIST_DIR="$(find_dist_dir "$OBJ_DIR")"
APP_BIN="$DIST_DIR/nevoflux"
if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]]; then
  APP_BIN="$DIST_DIR/nevoflux.exe"
fi

if [[ ! -x "$APP_BIN" ]]; then
  echo "Current build binary is missing or not executable: $APP_BIN" >&2
  exit 1
fi

echo "Building NevoFlux native agent from monorepo source..."
cargo build --release --manifest-path native/nevoflux-agent/Cargo.toml --bin nevoflux-agent

AGENT_NAME="nevoflux-agent"
if [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]]; then
  AGENT_NAME="nevoflux-agent.exe"
fi
AGENT_BUILD="native/nevoflux-agent/target/release/$AGENT_NAME"

if [[ ! -x "$AGENT_BUILD" ]]; then
  echo "Native agent build did not produce executable: $AGENT_BUILD" >&2
  exit 1
fi

echo "Building latest agent panel UI..."
(cd src/nevoflux/extensions/nevoflux-agent && env -u NO_COLOR npm run build:chat)

echo "Packaging latest agent panel extension..."
bash scripts/package-extension.sh

DIST_BUNDLE="$DIST_DIR/distribution"
mkdir -p "$DIST_BUNDLE/bin" "$DIST_BUNDLE/extensions"

if [[ -d build/AppDir/distribution ]]; then
  cp -R build/AppDir/distribution/. "$DIST_BUNDLE/"
fi

if [[ ! -f "$DIST_BUNDLE/extensions/agent@nevoflux.com.xpi" ]]; then
  echo "Packaged agent extension was not staged to $DIST_BUNDLE/extensions/agent@nevoflux.com.xpi" >&2
  exit 1
fi

install -m 0755 "$AGENT_BUILD" "$DIST_BUNDLE/bin/$AGENT_NAME"
write_native_host_manifest "$DIST_BUNDLE/bin/$AGENT_NAME"

export LANG="${LANG:-en_US.UTF-8}"
export LANGUAGE="${LANGUAGE:-en_US:en}"
export GDK_BACKEND="${GDK_BACKEND:-x11}"
export MOZ_ENABLE_WAYLAND="${MOZ_ENABLE_WAYLAND:-0}"
export MOZ_WEBRENDER_SOFTWARE="${MOZ_WEBRENDER_SOFTWARE:-1}"
export MOZ_DISABLE_GFX_SANITY_TEST="${MOZ_DISABLE_GFX_SANITY_TEST:-1}"
export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"

echo "Using build: $APP_BIN"
echo "Staged extension: $DIST_BUNDLE/extensions/agent@nevoflux.com.xpi"
echo "Staged native agent: $DIST_BUNDLE/bin/$AGENT_NAME"

exec npm run start -- \
  --new-instance \
  --temp-profile \
  --setpref intl.locale.requested=en-US \
  --setpref intl.locale.matchOS=false \
  --setpref app.update.disabledForTesting=true \
  --setpref app.update.auto=false \
  --setpref app.update.enabled=false \
  --setpref app.update.background.enabled=false \
  "$@"
