#!/usr/bin/env bash
# Build the native Windows release binary, then package it into an Inno Setup installer.
# Usage: ./scripts/build-windows-installer.sh [extra cargo args...]
# Env:   PROMPTDECK_WIN_BUILD_DIR (default: /mnt/d/build/promptdeck), PROMPTDECK_PROFILE (default: release),
#        ISCC (default: /mnt/c/Program Files (x86)/Inno Setup 6/ISCC.exe)
set -euo pipefail

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WIN_BUILD_DIR="${PROMPTDECK_WIN_BUILD_DIR:-/mnt/d/build/promptdeck}"

find_iscc() {
  if [ -n "${ISCC:-}" ]; then printf '%s' "$ISCC"; return; fi
  local c
  for c in \
    "/mnt/c/Program Files (x86)/Inno Setup 6/ISCC.exe" \
    "/mnt/c/Program Files/Inno Setup 6/ISCC.exe" \
    "/mnt/d/Inno Setup 6/ISCC.exe" \
    "/mnt/e/Inno Setup 6/ISCC.exe"; do
    [ -x "$c" ] && { printf '%s' "$c"; return; }
  done
}
ISCC="$(find_iscc)"

[ -n "$ISCC" ] && [ -x "$ISCC" ] || { echo "ISCC.exe not found; install Inno Setup 6 (winget install JRSoftware.InnoSetup) or set ISCC." >&2; exit 1; }

# 1. Build the binary (this also rsyncs the source tree to the mirror).
"$SRC_DIR/scripts/build-windows.sh" "$@"

# 2. Convert the WSL mirror path to a Windows path for ISCC.
to_win() {
  local p="$1"
  local drive rest
  drive="$(printf '%s' "$p" | sed -n 's|^/mnt/\([a-zA-Z]\)/.*|\1|p')"
  [ -n "$drive" ] || { echo "$p" >&2; return; }
  drive="$(printf '%s' "$drive" | tr 'a-z' 'A-Z')"
  rest="$(printf '%s' "$p" | sed 's|^/mnt/[a-zA-Z]/||; s|/|\\|g')"
  printf '%s:\\%s' "$drive" "$rest"
}

ISS_WIN="$(to_win "$WIN_BUILD_DIR/packaging/windows/promptdeck.iss")"
echo "Packaging: $ISS_WIN"
"$ISCC" "$ISS_WIN"

DIST_DIR="$WIN_BUILD_DIR/dist"
SETUP="$DIST_DIR/PromptDeck-0.1.0-preview.1-windows-x64-setup.exe"
[ -f "$SETUP" ] || { echo "installer not produced: $SETUP" >&2; exit 1; }

# 3. Emit a SHA256 checksum next to the installer.
( cd "$DIST_DIR" && sha256sum "$(basename "$SETUP")" > "$(basename "$SETUP").sha256" )

echo "OK: $SETUP"
echo "OK: $SETUP.sha256"
