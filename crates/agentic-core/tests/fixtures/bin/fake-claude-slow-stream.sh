#!/usr/bin/env bash
# Fixture for streaming runner test.
# Emits "line1", sleeps 2 seconds, then emits "line2".
# Used to verify that run_streaming yields stdout BEFORE the subprocess exits.

echo "line1"
sleep 2
echo "line2"
