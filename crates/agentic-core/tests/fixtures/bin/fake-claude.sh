#!/usr/bin/env bash
# Fake claude binary for subprocess runner tests.
# Reads all stdin, echoes it back in a typed JSON envelope, then emits a
# realistic (but synthetic) Claude stream-json sequence.

set -euo pipefail

# Consume stdin
stdin_content=$(cat)

# Echo stdin roundtrip so the test can verify piping worked
printf '{"type":"echo_stdin","content":"%s"}\n' "$stdin_content"

# Emit a minimal realistic Claude stream
printf '{"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","content":[],"model":"claude-opus-4-5","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}\n'
printf '{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello from fake claude"}}\n'
printf '{"type":"message_stop"}\n'

exit 0
