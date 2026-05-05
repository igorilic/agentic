import { useState } from "react";
import ChatComposer from "./ChatComposer";
import ChatMessageComp from "./ChatMessage";
import SpecDialog from "./SpecDialog";
import { createSpec } from "../utils/createSpec";
import { useBackend } from "../hooks/useBackend";
import type { ChatMessage } from "../types/chat";

const DEFAULT_PIPELINE_AGENTS = ["architect", "tdd-developer", "qa", "reviewer"];

export type ChatColumnProps = {
  messages: ChatMessage[];
  systemMessages: string[];
  mentionMessages?: Array<{ agent: string; body: string; t: string }>;
  activeAgent: string | null;
  onSend: (text: string) => void;
  error?: string | null;
  onTicketRunStarted?: (info: { runId: string; ticketLabel: string; description?: string }) => void;
  pipelineAgents?: string[];
};

/**
 * ChatColumn composes the spec §3.4 Chat column layout:
 *   - Header with "Chat with pipeline" title + active-agent chip
 *   - Scrollable message list using ChatMessage variants
 *   - Sticky ChatComposer at the bottom
 *
 * NOTE: The ChatMessage type in types/chat.ts uses role: "user" | "assistant" |
 * "system" | "tool" and carries no senderAgent field. Assistant messages are
 * rendered via the ChatMessage agent variant with agent="assistant" as a
 * placeholder. A proper senderAgent field would require a chat.ts schema change
 * (deliberate decision deferred — see GH issue for Phase 8+ tracking).
 *
 * NOTE: ActiveRunIndicator was removed in W.8.5. The run-state pill now lives
 * in HeaderBar (spec §3.4 line 250). HeaderBar.onStopRun → App.cancelActiveRun
 * is the one wiring path.
 */
export default function ChatColumn({
  messages,
  systemMessages,
  mentionMessages = [],
  activeAgent,
  onSend,
  error,
  onTicketRunStarted,
  pipelineAgents = DEFAULT_PIPELINE_AGENTS,
}: ChatColumnProps) {
  const [specOpen, setSpecOpen] = useState(false);
  const { backend } = useBackend();

  const handleSpecSubmit = async (title: string, body: string) => {
    console.log("[ChatColumn] handleSpecSubmit", { title, backend, bodyLen: body.length });
    try {
      const runId = await createSpec(title, backend, pipelineAgents);
      console.log("[ChatColumn] createSpec returned", { runId });
      if (runId !== undefined) {
        const description = body.trim().length > 0 ? body.trim() : undefined;
        onTicketRunStarted?.({ runId, ticketLabel: title, description });
      } else {
        console.warn("[ChatColumn] createSpec returned undefined — run not started; closing dialog silently");
      }
      setSpecOpen(false);
    } catch (err) {
      // Surface IPC errors via console so the user can diagnose. Dialog stays
      // open on failure. TODO: lift into a visible error slot once App.tsx
      // has one (cross-references IssueColumn's identical catch).
      console.error("createSpec failed:", err);
    }
  };

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
          onCreateSpec={() => setSpecOpen(true)}
        />
      </form>
      <SpecDialog
        open={specOpen}
        onClose={() => setSpecOpen(false)}
        onSubmit={handleSpecSubmit}
      />
    </div>
  );
}
