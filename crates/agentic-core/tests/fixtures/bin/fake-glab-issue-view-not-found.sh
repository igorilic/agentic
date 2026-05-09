#!/usr/bin/env bash
# Fake glab binary that simulates a 404 (issue not found).
case "$1" in
  issue)
    echo "ERROR 404 Not Found." >&2
    exit 1
    ;;
  api)
    echo "ERROR 404 Not Found." >&2
    exit 1
    ;;
  *)
    echo "unknown glab subcommand: $1" >&2
    exit 2
    ;;
esac
