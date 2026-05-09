#!/usr/bin/env bash
# Simulates: gh issue view <ref> returning garbage output.
# Exits 0 but stdout is not valid JSON.
echo "this is not valid json {"
