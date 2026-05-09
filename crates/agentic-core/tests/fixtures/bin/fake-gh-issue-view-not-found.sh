#!/usr/bin/env bash
# Simulates: gh issue view <ref> when issue doesn't exist.
# Exits non-zero and prints an error to stderr.
echo "GraphQL: Could not resolve to an issue with the number of 999. (notFound)" >&2
exit 1
