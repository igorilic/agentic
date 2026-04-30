import ChatComposer from "./ChatComposer";
import ChatMessageComp from "./ChatMessage";
import ActiveRunIndicator from "./ActiveRunIndicator";
import type { ChatMessage } from "../types/chat";

export type ChatColumnProps = {
  messages: ChatMessage[];
  systemMessages: string[];
  mentionMessages?: Array<{ agent: string; body: string; t: string }>;
  activeAgent: string | null;
  activeRunId: string | null;
  activeRunStartedAtMs: number | null;
  onSend: (text: string) => void;
  onCancelActiveRun?: () => Promise<void>;
  error?: string | null;
};

/**
 * ChatColumn composes the spec §3.4 Chat column layout:
 *   - Header with "Chat with pipeline" title + active-agent chip
 *   - Scrollable message list using ChatMessage variants
 *   - Optional ActiveRunIndicator
 *   - Sticky ChatComposer at the bottom
 *
 * NOTE: The ChatMessage type in types/chat.ts uses role: "user" | "assistant" |
 * "system" | "tool" and carries no senderAgent field. Assistant messages are
 * rendered via the ChatMessage agent variant with agent="assistant" as a
 * placeholder. A proper senderAgent field would require a chat.ts schema change
 * (deliberate decision deferred — see GH issue for Phase 8+ tracking).
 */
export default function ChatColumn({
  messages,
  systemMessages,
  mentionMessages = [],
  activeAgent,
  activeRunId,
  activeRunStartedAtMs,
  onSend,
  onCancelActiveRun,
  error,
}: ChatColumnProps) {
  return (
    <div
      data-testid="chat-column"
      className="flex flex-col h-full bg-bg-surface border-r border-border-soft"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border-soft">
        <h2 className="text-[13px] font-semibold text-fg">Chat with pipeline</h2>
        {activeAgent !== null && (
          <span
            data-testid="chat-column-active-chip"
            className="text-[11px] text-fg-muted"
          >
            {activeAgent} is responding
          </span>
        )}
      </div>

      {/* Scrollable messages */}
      <div
        data-testid="chat-messages"
        className="flex-1 overflow-y-auto px-4 py-3 flex flex-col gap-4 min-h-0"
      >
        {messages.map((msg) => {
          if (msg.role === "user") {
            return (
              <ChatMessageComp
                key={msg.id}
                kind="user"
                userName="You"
                timestamp={new Date(msg.created_at).toLocaleTimeString()}
                body={msg.content}
              />
            );
          }
          if (msg.role === "assistant" || msg.role === "tool") {
            return (
              <ChatMessageComp
                key={msg.id}
                kind="agent"
                agent={msg.senderAgent ?? "assistant"}
                timestamp={new Date(msg.created_at).toLocaleTimeString()}
                body={msg.content}
              />
            );
          }
          // role === "system"
          return (
            <ChatMessageComp
              key={msg.id}
              kind="system"
              body={msg.content}
            />
          );
        })}

        {systemMessages.map((text, i) => (
          <ChatMessageComp key={`sys-${i}`} kind="system" body={text} />
        ))}

        {mentionMessages.map((m, i) => (
          <ChatMessageComp
            key={`mention-${i}`}
            kind="agent"
            agent={m.agent}
            timestamp={m.t}
            body={m.body}
          />
        ))}

        {messages.length === 0 &&
          systemMessages.length === 0 &&
          mentionMessages.length === 0 && (
            <span className="px-1 italic text-gray-400">No messages yet.</span>
          )}
      </div>

      {/* Active run indicator — only render when both activeRunId AND
          onCancelActiveRun are provided; a cancel handler without a run id
          (or a run id without a handler) produces a dead affordance. */}
      {activeRunId !== null && onCancelActiveRun !== undefined && (
        <div className="px-3 py-1 border-t border-gray-200 bg-gray-50">
          <ActiveRunIndicator
            runId={activeRunId}
            startedAtMs={activeRunStartedAtMs}
            onCancel={onCancelActiveRun}
          />
        </div>
      )}

      {/* Error */}
      {error != null && (
        <div
          data-testid="chat-error"
          className="px-4 py-2 bg-red-50 text-red-700 text-xs"
          role="alert"
        >
          {error}
        </div>
      )}

      {/* Composer */}
      <form
        data-testid="chat-form"
        onSubmit={(e) => e.preventDefault()}
        className="border-t border-border-soft"
      >
        <ChatComposer
          onSend={onSend}
          inputTestId="chat-input"
          sendTestId="chat-send"
        />
      </form>
    </div>
  );
}
