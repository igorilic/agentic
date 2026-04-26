#!/usr/bin/env sh
# Simulates a slow gh CLI that hangs for 30 seconds.
# Used to test the 10-second subprocess timeout in GhDelegate.
sleep 30
echo "ghp_should_never_reach_here"
