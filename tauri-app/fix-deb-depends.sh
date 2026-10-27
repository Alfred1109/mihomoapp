#!/usr/bin/env bash
# Compatibility entry point: inspection only; packages are never rewritten.
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$SCRIPT_DIR/../scripts/check-deb.py" "$@"
