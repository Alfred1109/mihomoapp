#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUNDLE_DIR="${1:-$SCRIPT_DIR/backend/target/release/bundle/deb}"
DESIRED_DEPENDS="libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37, libayatana-appindicator3-1, libgtk-3-0"

mapfile -t DEB_FILES < <(find "$BUNDLE_DIR" -maxdepth 1 -type f -name "*.deb" | sort)

if [ "${#DEB_FILES[@]}" -eq 0 ]; then
  echo "No .deb packages found under $BUNDLE_DIR" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

for DEB_FILE in "${DEB_FILES[@]}"; do
  PKG_DIR="$TMP_DIR/$(basename "${DEB_FILE%.deb}")"
  CONTROL_FILE="$PKG_DIR/DEBIAN/control"
  REBUILT_DEB="$TMP_DIR/$(basename "$DEB_FILE")"

  rm -rf "$PKG_DIR"
  dpkg-deb -R "$DEB_FILE" "$PKG_DIR" >/dev/null

  if ! rg -q '^Depends:' "$CONTROL_FILE"; then
    echo "Missing Depends field in $CONTROL_FILE" >&2
    exit 1
  fi

  sed -i "s/^Depends:.*/Depends: $DESIRED_DEPENDS/" "$CONTROL_FILE"
  dpkg-deb -b "$PKG_DIR" "$REBUILT_DEB" >/dev/null
  mv "$REBUILT_DEB" "$DEB_FILE"

  echo "Patched Debian dependencies in $(basename "$DEB_FILE")"
done
