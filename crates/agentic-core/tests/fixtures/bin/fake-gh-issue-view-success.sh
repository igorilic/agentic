#!/usr/bin/env bash
# Simulates: gh issue view <ref> --json title,body,labels,state,url,comments
# Exits 0 and prints a canned JSON response.
set -euo pipefail
cat <<'JSON'
{
  "title": "Add a feature",
  "body": "Description here.\n\n## Acceptance Criteria\n- [ ] Feature works\n- [ ] Tests pass\n\n## Notes\n\nFurther context.",
  "labels": [{"id": "L_1", "name": "bug", "description": "", "color": "d73a4a"}],
  "state": "OPEN",
  "url": "https://github.com/owner/repo/issues/42",
  "comments": [
    {
      "id": "IC_1",
      "author": {"login": "alice"},
      "body": "first comment",
      "createdAt": "2026-04-24T10:00:00Z",
      "includesCreatedEdit": false,
      "isMinimized": false,
      "minimizedReason": "",
      "reactionGroups": [],
      "url": "https://github.com/owner/repo/issues/42#issuecomment-1",
      "viewerDidAuthor": false
    }
  ]
}
JSON
