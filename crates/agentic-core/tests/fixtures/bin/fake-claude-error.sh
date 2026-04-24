#!/usr/bin/env bash
# Fake claude binary that emits a rate_limit_event followed by an error result.
# Used for claude_backend_e2e error-path test.
# Emits the real Claude CLI envelope format.

set -euo pipefail

# Consume stdin
_stdin=$(cat)

# Emit a session init then a rate_limit_event (non-recoverable via process exit 1)
printf '{"type":"system","subtype":"init","session_id":"sess_test","model":"claude-sonnet-4-6","cwd":"/tmp","tools":[],"uuid":"init-001"}\n'
printf '{"type":"rate_limit_event","message":"invalid api key","retry_after":0}\n'

exit 1
