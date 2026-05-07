import { renderHook, act } from "@testing-library/react";
import { useChat, MAX_MESSAGES } from "../hooks/useChat";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function makeSystemMessage(content: string, sessionId = "s") {
  return {
    id: `sys-${Math.random().toString(36).slice(2)}`,
    session_id: sessionId,
    run_id: null,
    role: "system",
    content,
    metadata: null,
    created_at: Date.now(),
  };
}

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

  // -------------------------------------------------------------------------
  // UC-RS1 — recordSystem invokes chat_record_system_message with correct args
  // -------------------------------------------------------------------------
  it("UC-RS1: recordSystem invokes chat_record_system_message with sessionId, workspaceId, content", async () => {
    const msg = makeSystemMessage("slash command audit");
    invokeMock.mockResolvedValueOnce(msg);

    const { result } = renderHook(() => useChat());

    await act(async () => {
      await result.current.recordSystem("slash command audit");
    });

    expect(invokeMock).toHaveBeenCalledWith("chat_record_system_message", {
      sessionId: null,
      workspaceId: "default",
      content: "slash command audit",
    });
  });

  // -------------------------------------------------------------------------
  // UC-RS2 — successful recordSystem appends to messages
  // -------------------------------------------------------------------------
  it("UC-RS2: successful recordSystem appends the returned ChatMessage to messages", async () => {
    const msg = makeSystemMessage("run started: 01abc");
    invokeMock.mockResolvedValueOnce(msg);

    const { result } = renderHook(() => useChat());

    await act(async () => {
      await result.current.recordSystem("run started: 01abc");
    });

    expect(result.current.messages).toHaveLength(1);
    expect(result.current.messages[0].role).toBe("system");
    expect(result.current.messages[0].content).toBe("run started: 01abc");
  });

  // -------------------------------------------------------------------------
  // UC-RS3 — failed recordSystem sets error but does not throw
  // -------------------------------------------------------------------------
  it("UC-RS3: failed recordSystem sets error and does not throw", async () => {
    invokeMock.mockRejectedValueOnce("IPC failure");

    const { result } = renderHook(() => useChat());

    // Must not throw.
    await act(async () => {
      await result.current.recordSystem("will fail");
    });

    expect(result.current.error).toBe("IPC failure");
    // messages must remain empty — nothing appended on failure.
    expect(result.current.messages).toHaveLength(0);
  });
});
