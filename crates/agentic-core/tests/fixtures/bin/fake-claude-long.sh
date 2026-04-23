#!/usr/bin/env bash
# Fake claude binary that sleeps indefinitely — used to test SIGTERM cancellation.
# It does NOT trap signals, so SIGTERM will kill it immediately.

sleep 30
echo '{"type":"message_stop"}'
exit 0
