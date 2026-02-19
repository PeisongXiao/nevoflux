#!/bin/bash
# Detect the engine obj-* directory for the current platform
# Usage: source scripts/lib/detect-objdir.sh
#   Provides: OBJ_DIR (e.g., engine/obj-x86_64-pc-linux-gnu)

_detect_objdir() {
  local project_root="${1:-.}"
  local engine_dir="$project_root/engine"

  # Try to find existing obj-* directory
  local found
  found=$(find "$engine_dir" -maxdepth 1 -name 'obj-*' -type d 2>/dev/null | head -1)
  if [ -n "$found" ]; then
    echo "$found"
    return 0
  fi

  # Fallback: construct from platform detection
  local arch
  local os_triple
  arch="$(uname -m)"
  case "$(uname -s)" in
    Linux*)
      os_triple="${arch}-pc-linux-gnu"
      ;;
    Darwin*)
      [ "$arch" = "arm64" ] && arch="aarch64"
      os_triple="${arch}-apple-darwin"
      ;;
    MINGW*|MSYS*|CYGWIN*)
      os_triple="${arch}-pc-windows-msvc"
      ;;
    *)
      echo "ERROR: Unsupported platform: $(uname -s)" >&2
      return 1
      ;;
  esac

  echo "$engine_dir/obj-${os_triple}"
}

# Auto-set OBJ_DIR if PROJECT_ROOT is available
if [ -n "$PROJECT_ROOT" ]; then
  OBJ_DIR="$(_detect_objdir "$PROJECT_ROOT")"
fi
