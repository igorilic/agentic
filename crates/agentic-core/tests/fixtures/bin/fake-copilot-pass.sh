#!/usr/bin/env bash
# Fake copilot binary emitting a successful stream.
# Used for copilot_subprocess happy-path and copilot_backend_e2e happy-path tests.

cat <<'EOF'
{"type":"session.tools_updated","data":{"model":"claude-opus-4.6"},"id":"a","timestamp":"2026-04-24T00:00:00Z","parentId":"p"}
{"type":"assistant.turn_start","data":{"turnId":"0","interactionId":"ix"},"id":"b","timestamp":"2026-04-24T00:00:01Z","parentId":"p"}
{"type":"assistant.message_delta","data":{"messageId":"m1","deltaContent":"Hello"},"id":"c","timestamp":"2026-04-24T00:00:02Z","parentId":"b","ephemeral":true}
{"type":"assistant.message","data":{"messageId":"m1","content":"Hello","toolRequests":[],"interactionId":"ix","outputTokens":3,"requestId":"r"},"id":"d","timestamp":"2026-04-24T00:00:03Z","parentId":"b"}
{"type":"assistant.turn_end","data":{"turnId":"0"},"id":"e","timestamp":"2026-04-24T00:00:04Z","parentId":"d"}
{"type":"result","timestamp":"2026-04-24T00:00:05Z","sessionId":"sess","exitCode":0,"usage":{"premiumRequests":1,"totalApiDurationMs":1000,"sessionDurationMs":2000,"codeChanges":{"linesAdded":0,"linesRemoved":0,"filesModified":[]}}}
EOF
