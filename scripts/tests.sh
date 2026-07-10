#!/usr/bin/env bash
set -euo pipefail

resolve_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    command -v cargo
    return
  fi
  if command -v cargo.exe >/dev/null 2>&1; then
    command -v cargo.exe
    return
  fi
  if [ -x "${HOME:-}/.cargo/bin/cargo" ]; then
    printf '%s\n' "${HOME}/.cargo/bin/cargo"
    return
  fi
  if [ -x "${HOME:-}/.cargo/bin/cargo.exe" ]; then
    printf '%s\n' "${HOME}/.cargo/bin/cargo.exe"
    return
  fi
  if command -v cygpath >/dev/null 2>&1 && [ -n "${USERPROFILE:-}" ]; then
    local cargo_home
    cargo_home="$(cygpath "$USERPROFILE")/.cargo/bin/cargo.exe"
    if [ -x "$cargo_home" ]; then
      printf '%s\n' "$cargo_home"
      return
    fi
  fi
  echo "cargo not found" >&2
  exit 127
}

CARGO_BIN="$(resolve_cargo)"

"$CARGO_BIN" test --locked
node --test "tests/node/*.test.js"
