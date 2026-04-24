#!/usr/bin/env bash
# Fake copilot binary that traps SIGTERM so the runner must escalate to SIGKILL.
# Used for cancel/signal escalation tests.

trap 'sleep 3600 &' TERM

cat <<'EOF'
{"type":"session.tools_updated","data":{"model":"claude-opus-4.6"},"id":"a","timestamp":"2026-04-24T00:00:00Z","parentId":"p"}
EOF

while true; do sleep 1; done
