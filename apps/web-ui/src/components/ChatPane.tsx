import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useChat } from "../hooks/useChat";
import { useMentionEvents } from "../hooks/useMentionEvents";
import { parseSlashCommand, formatSlashParseError } from "../slash/parser";
import { dispatchSlashCommand, type SlashServices } from "../slash/dispatcher";
import { parseMention, formatMentionParseError } from "../mention/parser";
import ChatColumn from "./ChatColumn";

type MentionResult = {
  run_id: string;
  agent: string;
  dispatched: boolean;
};

export type ChatPaneProps = {
  /// Called when `/plan <ticket>` successfully kicks off a real ticket run.
  onTicketRunStarted?: (runId: string) => void;
  /// Currently-active run, if any.
  activeRunId?: string | null;
  /// Wall-clock start of the active run (from the first envelope's timestamp_ms).
  activeRunStartedAtMs?: number | null;
  /// Cancels the currently-active run (best-effort SIGTERM via the `cancel_run` IPC).
  onCancelActiveRun?: () => Promise<void>;
};

export default function ChatPane({
  onTicketRunStarted,
  activeRunId = null,
  activeRunStartedAtMs = null,
  onCancelActiveRun,
}: ChatPaneProps = {}) {
  const { messages, send, sending, error } = useChat();
  const [systemMessages, setSystemMessages] = useState<string[]>([]);
  // Maps mention run_id → agent name so event envelopes can be rendered with
  // the correct per-agent identity. The EventEnvelope schema does not carry an
  // agent field; we capture it at dispatch time from the parsed mention command.
  const [mentionRunAgents, setMentionRunAgents] = useState<
    Record<string, string>
  >({});
  const mentionEvents = useMentionEvents();

  // Slash-command services wired to IPC.
  const slashServices: SlashServices = useMemo(
    () => ({
      plan: async (ticket, backend) => {
        const runId = (await invoke("start_ticket_run", {
          ticket,
          backend: backend ?? "claude-code",
          model: null,
        })) as string;
        onTicketRunStarted?.(runId);
        return runId;
      },
      status: async (_runId) => {
        throw new Error(
          "[STUB] /status is not yet wired to a real backend (Phase 11.7+)",
        );
      },
      cancel: async (_runId) => {
        throw new Error(
          "[STUB] /cancel is not yet wired to a real backend (Phase 11.7+)",
        );
      },
    }),
    [onTicketRunStarted],
  );

  // Project mention envelopes into renderable chat messages.
  // The agent name is looked up from mentionRunAgents (populated at dispatch
  // time) using the envelope's run_id. Falls back to "mention" for envelopes
  // whose run was not initiated in this session (e.g., replayed stubs).
  const mentionMessages = useMemo(
    () =>
      mentionEvents
        .map((env) => {
          const ev = env.event as { type: string; data?: { content?: string } };
          if (ev.type === "TextDelta" && typeof ev.data?.content === "string") {
            return {
              id: env.event_id,
              content: ev.data.content,
              agent: mentionRunAgents[env.run_id] ?? "mention",
            };
          }
          return null;
        })
        .filter(
          (m): m is { id: string; content: string; agent: string } =>
            m !== null,
        ),
    [mentionEvents, mentionRunAgents],
  );

  // onSend is called by ChatColumn/ChatComposer on Cmd+Enter or send-button click.
  // It handles slash commands, @mentions, and plain chat sends.
  const onSend = (text: string): void => {
    void (async () => {
      if (!text.trim() || sending) return;

      if (text.trim().startsWith("/")) {
        const parsed = parseSlashCommand(text);
        if (!parsed.ok) {
          setSystemMessages((prev) => [...prev, formatSlashParseError(parsed.error)]);
          return;
        }
        try {
          const result = await dispatchSlashCommand(parsed.command, slashServices);
          setSystemMessages((prev) => [...prev, result.message]);
        } catch (err) {
          setSystemMessages((prev) => [...prev, `Command failed: ${err}`]);
        }
        return;
      }

      if (text.trim().startsWith("@")) {
        const parsed = parseMention(text);
        if (!parsed.ok) {
          setSystemMessages((prev) => [...prev, formatMentionParseError(parsed.error)]);
          return;
        }
        try {
          const result = (await invoke("mention_agent", {
            agent: parsed.command.agent,
            body: parsed.command.body,
          })) as MentionResult;
          // Record run_id → agent so incoming EventEnvelopes on the mention
          // channel can be projected with the real agent name.
          setMentionRunAgents((prev) => ({
            ...prev,
            [result.run_id]: result.agent,
          }));
          setSystemMessages((prev) => [
            ...prev,
            `Mention dispatched to @${parsed.command.agent} (run ${result.run_id})${result.dispatched ? "" : " [STUB]"}`,
          ]);
        } catch (e) {
          setSystemMessages((prev) => [...prev, `Mention failed: ${e}`]);
        }
        return;
      }

      await send(text);
    })();
  };

  // Adapt mentionMessages for ChatColumn: map to { agent, body, t } shape.
  // The agent field now carries the real agent name (e.g., "architect") from
  // the dispatch-time run_id→agent mapping rather than the "mention" placeholder.
  // Timestamp is not surfaced in EventEnvelope for mention stubs; use empty
  // string (ChatMessage renders nothing for an empty timestamp).
  const mentionMessagesForColumn = useMemo(
    () =>
      mentionMessages.map((m) => ({
        agent: m.agent,
        body: m.content,
        t: "",
      })),
    [mentionMessages],
  );

  return (
    <section
      className="flex flex-col h-full min-h-0 flex-1"
      data-testid="chat-pane"
    >
      <ChatColumn
        messages={messages}
        systemMessages={systemMessages}
        mentionMessages={mentionMessagesForColumn}
        activeAgent={null}
        activeRunId={activeRunId}
        activeRunStartedAtMs={activeRunStartedAtMs}
        onSend={onSend}
        onCancelActiveRun={onCancelActiveRun}
        error={error}
      />
    </section>
  );
}
