#!/usr/bin/env bash
# Emit ~200KB of stderr to test 64KB cap.
yes "this is a stderr line that should be truncated by the 64KB buffer cap" | head -3000 1>&2
exit 1
