export type ChatMessage = {
  id: string;
  session_id: string;
  run_id: string | null;
  role: "user" | "assistant" | "system" | "tool";
  /**
   * The agent that produced an `assistant`/`tool` message. Optional —
   * not yet populated by the backend; tracked in GH #90 for the
   * agentic-core schema change.
   */
  senderAgent?: string;
  content: string;
  metadata: string | null;
  created_at: number;
};

export type ChatSendResult = {
  user_message: ChatMessage;
  reply: ChatMessage;
};
