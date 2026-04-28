import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useChat } from "../hooks/useChat";
import { useMentionEvents } from "../hooks/useMentionEvents";
import { parseSlashCommand, formatSlashParseError } from "../slash/parser";
import { dispatchSlashCommand, type SlashServices } from "../slash/dispatcher";
import { parseMention, formatMentionParseError } from "../mention/parser";

type MentionResult = {
  run_id: string;
  agent: string;
  dispatched: boolean;
};

export type ChatPaneProps = {
  /// Called when `/plan <ticket>` successfully kicks off a real ticket run.
  /// The cockpit uses this to pin its active-run id so the Stepper /
  /// EventList / FindingsTable follow the new run.
  onTicketRunStarted?: (runId: string) => void;
};

export default function ChatPane({ onTicketRunStarted }: ChatPaneProps = {}) {
  const { messages, send, sending, error } = useChat();
  const [draft, setDraft] = useState("");
  const [systemMessages, setSystemMessages] = useState<string[]>([]);
  const mentionEvents = useMentionEvents();

  // Slash-command services. Defined inside the component so `plan` can close
  // over the `onTicketRunStarted` callback (which lives in parent state).
  // `status` and `cancel` remain stubbed for now — Phase 11.7+.
  const slashServices: SlashServices = useMemo(
    () => ({
      plan: async (ticket, backend) => {
        const runId = (await invoke("start_ticket_run", {
          ticket,
          // User's `--backend=…` flag wins; fall back to claude-code.
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

  // Project mention envelopes into renderable text. Only TextDelta is shown;
  // other envelope kinds (RunStarted, RunComplete, …) are bookkeeping for the
  // future real backend and would be noise in the chat transcript.
  const mentionMessages = useMemo(
    () =>
      mentionEvents
        .map((env) => {
          const ev = env.event as { type: string; data?: { content?: string } };
          if (ev.type === "TextDelta" && typeof ev.data?.content === "string") {
            return { id: env.event_id, content: ev.data.content };
          }
          return null;
        })
        .filter((m): m is { id: string; content: string } => m !== null),
    [mentionEvents],
  );

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!draft.trim() || sending) return;
    const text = draft;
    setDraft("");

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
  };

  return (
    <section
      className="flex flex-col h-96 border border-gray-200 rounded"
      data-testid="chat-pane"
    >
      <ul
        className="flex-1 overflow-y-auto divide-y divide-gray-100"
        aria-label="Chat messages"
        data-testid="chat-messages"
      >
        {messages.map((m) => (
          <li
            key={m.id}
            data-testid={`chat-message-${m.role}`}
            className="px-3 py-2 flex gap-3"
          >
            <span
              className={`text-xs font-semibold uppercase shrink-0 ${
                m.role === "user" ? "text-blue-600" : "text-gray-500"
              }`}
            >
              {m.role}
            </span>
            <span className="text-sm text-gray-800">{m.content}</span>
          </li>
        ))}
        {systemMessages.map((msg, i) => (
          <li
            key={`sys-${i}`}
            data-testid="chat-message-system"
            className="px-3 py-2 flex gap-3"
          >
            <span className="text-xs font-semibold uppercase shrink-0 text-yellow-600">
              system
            </span>
            <span className="text-sm text-gray-800">{msg}</span>
          </li>
        ))}
        {mentionMessages.map((m) => (
          <li
            key={`mention-${m.id}`}
            data-testid="chat-message-mention"
            className="px-3 py-2 flex gap-3"
          >
            <span className="text-xs font-semibold uppercase shrink-0 text-purple-600">
              mention
            </span>
            <span className="text-sm text-gray-800 whitespace-pre-wrap">{m.content}</span>
          </li>
        ))}
        {messages.length === 0 &&
          systemMessages.length === 0 &&
          mentionMessages.length === 0 && (
            <li className="px-3 py-2 italic text-gray-400">No messages yet.</li>
          )}
      </ul>
      {error && (
        <div
          className="px-3 py-2 bg-red-50 text-sm text-red-700"
          role="alert"
          data-testid="chat-error"
        >
          {error}
        </div>
      )}
      <form
        onSubmit={onSubmit}
        className="px-3 py-2 border-t border-gray-200 flex gap-2"
        data-testid="chat-form"
      >
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Type a message..."
          className="flex-1 px-2 py-1 border border-gray-300 rounded text-sm"
          data-testid="chat-input"
          disabled={sending}
        />
        <button
          type="submit"
          disabled={!draft.trim() || sending}
          className="px-3 py-1 bg-blue-600 text-white rounded text-sm disabled:bg-gray-400"
          data-testid="chat-send"
        >
          Send
        </button>
      </form>
    </section>
  );
}
