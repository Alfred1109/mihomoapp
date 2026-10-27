#!/usr/bin/env bash
# Versions and trusted release checksums are shared with the Windows entry point.
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if (( $# > 1 )); then
    echo "用法: $0 [vX.Y.Z]" >&2
    exit 1
fi
if (( $# == 1 )); then
    exec node "$SCRIPT_DIR/prepare-resources.mjs" --mihomo-version "$1"
fi
exec node "$SCRIPT_DIR/prepare-resources.mjs"
