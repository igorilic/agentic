import { useState } from "react";
import { useChat } from "../hooks/useChat";

export default function ChatPane() {
  const { messages, send, sending, error } = useChat();
  const [draft, setDraft] = useState("");

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!draft.trim() || sending) return;
    const text = draft;
    setDraft("");
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
        {messages.length === 0 && (
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
