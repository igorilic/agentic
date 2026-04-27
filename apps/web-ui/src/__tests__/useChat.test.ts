import { renderHook, act } from "@testing-library/react";
import { useChat, MAX_MESSAGES } from "../hooks/useChat";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("useChat", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("caps the messages array at MAX_MESSAGES", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "chat_send_message") {
        const id = Math.random().toString(36).slice(2);
        return {
          user_message: {
            id: `u-${id}`,
            session_id: "s",
            run_id: null,
            role: "user",
            content: "x",
            metadata: null,
            created_at: Date.now(),
          },
          reply: {
            id: `a-${id}`,
            session_id: "s",
            run_id: null,
            role: "assistant",
            content: "Echo: x",
            metadata: null,
            created_at: Date.now() + 1,
          },
        };
      }
    });

    const { result } = renderHook(() => useChat());

    // Each send adds 2 messages. Send enough to overflow MAX_MESSAGES.
    const overflow = Math.ceil(MAX_MESSAGES / 2) + 10;
    for (let i = 0; i < overflow; i++) {
      await act(async () => {
        await result.current.send(`msg-${i}`);
      });
    }

    expect(result.current.messages.length).toBe(MAX_MESSAGES);
  });
});
