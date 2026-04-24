#!/usr/bin/env bash
# Fake claude binary emitting a successful stream with token usage.
# Used for claude_backend_e2e happy-path test.
# Emits the real Claude CLI envelope format (assistant/result envelopes).

set -euo pipefail

# Consume stdin (the user_context)
_stdin=$(cat)

# Emit a minimal realistic Claude CLI stream with usage
printf '{"type":"system","subtype":"init","session_id":"sess_test","model":"claude-sonnet-4-6","cwd":"/tmp","tools":[],"uuid":"init-001"}\n'
printf '{"type":"assistant","message":{"model":"claude-sonnet-4-6","id":"msg_01","type":"message","role":"assistant","content":[{"type":"text","text":"Task complete."}],"stop_reason":"end_turn","usage":{"input_tokens":25,"output_tokens":3,"cache_read_input_tokens":5,"cache_creation_input_tokens":0}},"parent_tool_use_id":null,"session_id":"sess_test","uuid":"msg-001"}\n'
printf '{"type":"result","subtype":"success","duration_ms":100,"session_id":"sess_test"}\n'

exit 0
