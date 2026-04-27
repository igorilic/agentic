import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage, ChatSendResult } from "../types/chat";

export const MAX_MESSAGES = 200;

const DEFAULT_WORKSPACE_ID = "default";

export type UseChatResult = {
  messages: ChatMessage[];
  /** Submit a message. Persists user msg + reply, appends both to local state. */
  send: (content: string) => Promise<void>;
  /** True while a send is in-flight. */
  sending: boolean;
  /** Last error from a failed send, or null. */
  error: string | null;
};

export function useChat(): UseChatResult {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const send = async (content: string) => {
    if (!content.trim()) return;
    setSending(true);
    setError(null);
    try {
      const result = (await invoke("chat_send_message", {
        sessionId,
        workspaceId: DEFAULT_WORKSPACE_ID,
        content: content.trim(),
      })) as ChatSendResult;
      if (!sessionId) setSessionId(result.user_message.session_id);
      setMessages((prev) => {
        const next = [...prev, result.user_message, result.reply];
        return next.length > MAX_MESSAGES ? next.slice(-MAX_MESSAGES) : next;
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setSending(false);
    }
  };

  return { messages, send, sending, error };
}
