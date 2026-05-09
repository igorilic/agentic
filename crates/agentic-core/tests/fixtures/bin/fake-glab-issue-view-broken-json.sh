#!/usr/bin/env bash
# Fake glab binary that returns garbage JSON from the issue subcommand.
case "$1" in
  issue)
    echo "this is not valid json {"
    exit 0
    ;;
  api)
    echo "[]"
    ;;
  *)
    echo "unknown glab subcommand: $1" >&2
    exit 2
    ;;
esac
