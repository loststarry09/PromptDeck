#!/usr/bin/env bash
# Sync the source tree to NTFS and build a native Windows MSVC binary.
# Usage: ./scripts/build-windows.sh [cargo args...]
# Env:   PROMPTDECK_WIN_BUILD_DIR, PROMPTDECK_PROFILE (default: release)
set -euo pipefail

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WIN_BUILD_DIR="${PROMPTDECK_WIN_BUILD_DIR:-/mnt/d/build/promptdeck}"
TARGET="x86_64-pc-windows-msvc"
PROFILE="${PROMPTDECK_PROFILE:-release}"
PS="/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe"

get_user_env() {
  "$PS" -NoProfile -Command "[Environment]::GetEnvironmentVariable('$1','User')" | tr -d '\r'
}

CARGO_HOME_WIN="$(get_user_env CARGO_HOME)"
RUSTUP_HOME_WIN="$(get_user_env RUSTUP_HOME)"
[ -n "$CARGO_HOME_WIN" ] || { echo "CARGO_HOME (User) is not set; install Windows rustup first" >&2; exit 1; }
[ -n "$RUSTUP_HOME_WIN" ] || { echo "RUSTUP_HOME (User) is not set; install Windows rustup first" >&2; exit 1; }

drive="$(printf '%s' "$CARGO_HOME_WIN" | cut -d: -f1 | tr 'A-Z' 'a-z')"
rest="$(printf '%s' "$CARGO_HOME_WIN" | cut -d: -f2- | tr '\\' '/')"
CARGO_EXE="/mnt/$drive$rest/bin/cargo.exe"
[ -x "$CARGO_EXE" ] || { echo "cargo.exe not found at $CARGO_EXE" >&2; exit 1; }

export WSLENV="${WSLENV:+$WSLENV:}CARGO_HOME:RUSTUP_HOME"
export CARGO_HOME="$CARGO_HOME_WIN"
export RUSTUP_HOME="$RUSTUP_HOME_WIN"

mkdir -p "$WIN_BUILD_DIR"
rsync -a --delete \
  --exclude target --exclude .git --exclude .scratch --exclude .agents \
  "$SRC_DIR/" "$WIN_BUILD_DIR/"

cd "$WIN_BUILD_DIR"
"$CARGO_EXE" build --profile "$PROFILE" --target "$TARGET" "$@"
echo "OK: $WIN_BUILD_DIR/target/$TARGET/$PROFILE/promptdeck.exe"
