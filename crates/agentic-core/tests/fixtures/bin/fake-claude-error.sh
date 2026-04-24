#!/usr/bin/env bash
# Fake claude binary that emits an upstream error event (non-recoverable).
# Used for claude_backend_e2e error-path test.

set -euo pipefail

# Consume stdin
_stdin=$(cat)

# Emit a message_start then an authentication error
printf '{"type":"message_start","message":{"id":"msg_02","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-6","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":5,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}\n'
printf '{"type":"error","error":{"type":"authentication_error","message":"invalid api key"}}\n'

exit 1
