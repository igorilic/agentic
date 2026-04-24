#!/usr/bin/env bash
# Fake claude binary emitting a successful stream with token usage.
# Used for claude_backend_e2e happy-path test.

set -euo pipefail

# Consume stdin (the user_context)
_stdin=$(cat)

# Emit a minimal realistic Claude stream with usage
printf '{"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-6","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":25,"output_tokens":0,"cache_read_input_tokens":5,"cache_creation_input_tokens":0}}}\n'
printf '{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}\n'
printf '{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Task complete."}}\n'
printf '{"type":"content_block_stop","index":0}\n'
printf '{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":3}}\n'
printf '{"type":"message_stop"}\n'

exit 0
