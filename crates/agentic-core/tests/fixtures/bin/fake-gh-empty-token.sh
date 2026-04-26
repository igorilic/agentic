#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "auth status")
    exit 0
    ;;
  "auth token")
    echo ""
    exit 0
    ;;
  *)
    exit 2
    ;;
esac
