export type ChatMessage = {
  id: string;
  session_id: string;
  run_id: string | null;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  metadata: string | null;
  created_at: number;
};

export type ChatSendResult = {
  user_message: ChatMessage;
  reply: ChatMessage;
};
