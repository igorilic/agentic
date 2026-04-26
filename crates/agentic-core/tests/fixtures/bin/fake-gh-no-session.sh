#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "auth status")
    echo "You are not logged into any GitHub hosts." >&2
    exit 1
    ;;
  *)
    exit 1
    ;;
esac
