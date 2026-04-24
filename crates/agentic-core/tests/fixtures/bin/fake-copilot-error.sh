#!/usr/bin/env bash
# Fake copilot binary emitting a non-zero exit.
# Used for the error/nonzero-exit tests.

cat <<'EOF'
{"type":"session.tools_updated","data":{"model":"claude-opus-4.6"},"id":"a","timestamp":"2026-04-24T00:00:00Z","parentId":"p"}
{"type":"result","timestamp":"2026-04-24T00:00:01Z","sessionId":"sess","exitCode":2,"usage":{"premiumRequests":0,"totalApiDurationMs":0,"sessionDurationMs":0,"codeChanges":{"linesAdded":0,"linesRemoved":0,"filesModified":[]}}}
EOF
exit 2
