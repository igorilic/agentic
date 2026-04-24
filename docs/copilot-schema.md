# Copilot CLI stream-json schema (observed 2026-04-24)

Captured from `copilot --version` `GitHub Copilot CLI 1.0.34` on macOS with
`--output-format json --allow-all-tools -p "<prompt>"`.

## Envelope

All lines are JSON objects with common fields:
- `type`: string (dispatches event kind)
- `id`: uuid — unique event id
- `timestamp`: ISO-8601 string
- `parentId`: uuid — causal parent
- `ephemeral`: optional bool — if true, UI-only transient event (we'll ignore these)
- `data`: event-specific payload

## Event types observed

### session.*  (ignore)
- `session.mcp_server_status_changed` — MCP server connection status
- `session.mcp_servers_loaded` — MCP server summary
- `session.skills_loaded` — available skills
- `session.tools_updated` — model/tool info. `data.model` e.g. `"claude-opus-4.6"`
- `session.background_tasks_changed` — background task status

**Mapping for core `Event`**: drop all. Noise for the pipeline-core use case.

### user.message  (ignore)
Echo of the prompt (with `transformedContent` injected by Copilot).

**Mapping**: drop.

### assistant.turn_start / assistant.turn_end  (ignore)
Turn markers, `data.turnId`.

**Mapping**: drop. The `StepStarted`/`StepComplete` lifecycle is synthesized at the backend level (Step 6.7 pattern).

### assistant.message_delta  (→ `Event::TextDelta`)
Streaming content:
```json
{"type":"assistant.message_delta","data":{"messageId":"...","deltaContent":"..."}, ...}
```

**Mapping**: emit `Event::TextDelta { content: data.deltaContent }`.

### assistant.message  (→ `Event::ToolUseStart` per tool request; emit nothing for text — already delivered via deltas)
```json
{"type":"assistant.message","data":{
  "messageId":"...","content":"...",
  "toolRequests":[{"toolCallId":"...","name":"bash","arguments":{...},"type":"function"}],
  "outputTokens":125
}}
```

**Mapping**:
- For each `toolRequests[]`: `Event::ToolUseStart { tool_call_id: toolCallId, tool_name: name, input: arguments }`
- Accumulate `outputTokens` into `ParseOutcome.token_usage.output_tokens`

### tool.execution_start  (ignore — already have ToolUseStart from assistant.message)
```json
{"type":"tool.execution_start","data":{"toolCallId":"...","toolName":"...","arguments":{...}}}
```

Copilot emits this AFTER assistant.message.toolRequests and right before execution. Redundant for our purposes.

### tool.execution_complete  (→ `Event::ToolUseEnd`)
```json
{"type":"tool.execution_complete","data":{
  "toolCallId":"...",
  "success": true,
  "result": {"content": "...", "detailedContent": "..."}
}}
```

**Mapping**: emit `Event::ToolUseEnd { tool_call_id: toolCallId, exit_code: if success 0 else 1, duration_ms: 0 }`. Copilot doesn't expose a per-tool duration.

### result  (→ not emitted as Event; final outcome)
```json
{"type":"result","timestamp":"...","sessionId":"...","exitCode":0,"usage":{
  "premiumRequests":3,"totalApiDurationMs":9186,"sessionDurationMs":14395,
  "codeChanges":{"linesAdded":0,"linesRemoved":0,"filesModified":[]}
}}
```

**Mapping**: backend synthesizes `ExecuteOutcome` from this (exit code, duration). Parser may note `codeChanges.filesModified` for observer hinting but primary file tracking remains via Edit/Write tool-use events.

## Known differences from Claude CLI

| | Claude CLI | Copilot CLI |
|---|---|---|
| Envelope | `{"type":"assistant","message":{...}}` | `{"type":"assistant.message","data":{...}}` |
| Streaming | emits whole `assistant` envelope per turn | emits `assistant.message_delta` incrementally |
| Tool call origin | `assistant.message.content[].tool_use` block | `assistant.message.toolRequests[]` |
| Tool result | `user.message.content[].tool_result` | `tool.execution_complete.data.result.content` |
| Token usage | `assistant.message.usage.{input,output,cache_*}` | `assistant.message.outputTokens` + `result.usage.premiumRequests` |
| Rate limit | `rate_limit_event` envelope | not observed in samples |
| Error | `error` or non-zero `exitCode` | non-zero `exitCode` in `result` |

Schema is compatible with our core `Event` enum — no ADR needed.

## Unknowns / TODO for Step 7.2

- **Error path**: what does Copilot emit on auth/rate errors? Captured samples were all happy path. Step 7.2 parser should log unknown types and continue (defensive).
- **Streaming tool result**: `tool.execution_complete.data.result.content` is whole-text. No streaming of tool output.
