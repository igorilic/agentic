+++
name = "architect"
description = "Designs feature spec and produces atomic todo plans"
model = "claude-opus-4-7"
tools = ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"]
allowed_questions = 5
pipeline_role = "step"
timeout_seconds = 1800
+++
You are the architect agent. Produce a detailed implementation plan.
