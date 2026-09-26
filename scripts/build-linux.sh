#!/usr/bin/env bash
# Build Open Stego for Linux: CLI + .deb (+ AppImage)
# Usage (from repo root, on a Linux host):
#   ./scripts/build-linux.sh
#   ./scripts/build-linux.sh --cli-only
#   ./scripts/build-linux.sh --deb-only
#   ./scripts/build-linux.sh --gui-only

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CLI_ONLY=0
GUI_ONLY=0
DEB_ONLY=0

for arg in "$@"; do
  case "$arg" in
    --cli-only) CLI_ONLY=1 ;;
    --gui-only) GUI_ONLY=1 ;;
    --deb-only) DEB_ONLY=1; GUI_ONLY=0; CLI_ONLY=0 ;;
    -h|--help)
      sed -n '2,8p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

echo "==> Repo: $ROOT"

if [[ "$GUI_ONLY" -eq 0 && "$DEB_ONLY" -eq 0 ]]; then
  echo "==> Building CLI (release)..."
  cargo build -p stego-cli --release
  CLI_BIN="$ROOT/target/release/stego"
  test -f "$CLI_BIN"
  echo "    OK: $CLI_BIN"
fi

if [[ "$CLI_ONLY" -eq 0 ]]; then
  echo "==> Installing npm deps + building Linux packages..."
  cd "$ROOT/apps/desktop"
  if [[ ! -d node_modules ]]; then
    npm install
  fi
  if [[ "$DEB_ONLY" -eq 1 ]]; then
    npm run build:deb
  else
    npm run build:linux
  fi
  cd "$ROOT"

  echo "==> Looking for packages..."
  for dir in \
    "$ROOT/apps/desktop/src-tauri/target/release/bundle" \
    "$ROOT/target/release/bundle"
  do
    if [[ -d "$dir" ]]; then
      find "$dir" \( -name '*.deb' -o -name '*.AppImage' -o -name '*.rpm' \) -print \
        | while read -r f; do echo "    $f"; done
    fi
  done
fi

echo "==> Done."
