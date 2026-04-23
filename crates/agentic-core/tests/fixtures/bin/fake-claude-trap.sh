#!/usr/bin/env bash
# Fake claude binary that traps SIGTERM and sleeps indefinitely.
# Used to test SIGKILL escalation after SIGTERM grace period.

# Trap SIGTERM and do nothing (ignore it) to force SIGKILL escalation
trap '' TERM

sleep 3600 &
SLEEP_PID=$!

# Wait in a loop so that even if the subshell sleep is killed, we stay alive
while true; do
    sleep 1
done
