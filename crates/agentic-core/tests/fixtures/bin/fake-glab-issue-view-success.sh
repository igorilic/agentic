#!/usr/bin/env bash
# Fake glab binary for testing GitlabTicketSource.
# Handles two subcommands:
#   issue view <iid> --repo <group/project> --output json
#   api /projects/<encoded>/issues/<iid>/notes
set -euo pipefail

case "$1" in
  issue)
    # glab issue view <iid> --repo ... --output json
    cat <<'JSON'
{
  "id": 42,
  "iid": 42,
  "title": "Add feature",
  "description": "Body.\n\n## Acceptance Criteria\n- [ ] Works\n\n## Notes\nMore.",
  "state": "opened",
  "web_url": "https://gitlab.com/group/repo/-/issues/42",
  "author": {"id": 1, "username": "alice", "name": "Alice"},
  "labels": [{"name": "bug"}]
}
JSON
    ;;
  api)
    # glab api /projects/<encoded>/issues/<iid>/notes
    cat <<'JSON'
[
  {
    "id": 1,
    "body": "first note",
    "author": {"id": 2, "username": "bob", "name": "Bob"},
    "created_at": "2026-04-24T10:00:00.000Z",
    "system": false
  }
]
JSON
    ;;
  *)
    echo "unknown glab subcommand: $1" >&2
    exit 2
    ;;
esac
