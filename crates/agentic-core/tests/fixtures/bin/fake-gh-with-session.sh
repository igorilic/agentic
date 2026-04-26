#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  "auth status")
    echo "Logged in to github.com as testuser"
    exit 0
    ;;
  "auth token")
    echo "ghp_faketoken_for_test_xyz"
    exit 0
    ;;
  *)
    echo "unknown gh subcommand: $*" >&2
    exit 2
    ;;
esac
