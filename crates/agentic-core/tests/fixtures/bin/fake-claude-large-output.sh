#!/usr/bin/env bash
# Read all stdin first (prevent deadlock: drain stdin before producing stdout).
cat > /dev/null
# Then emit ~100KB of output (well past stdout pipe buffer).
yes "this is a long line that fills up the pipe buffer" | head -2000
