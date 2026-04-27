import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ChatPane from "../components/ChatPane";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

function makeSendResult(content: string, sessionId = "sess-1") {
  return {
    user_message: {
      id: "msg-user-1",
      session_id: sessionId,
      run_id: null,
      role: "user",
      content,
      metadata: null,
      created_at: 1000,
    },
    reply: {
      id: "msg-asst-1",
      session_id: sessionId,
      run_id: null,
      role: "assistant",
      content: `Echo: ${content}`,
      metadata: null,
      created_at: 1001,
    },
  };
}

describe("ChatPane", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders empty state when no messages", () => {
    render(<ChatPane />);
    expect(screen.getByTestId("chat-pane")).toBeInTheDocument();
    expect(screen.getByTestId("chat-messages")).toBeInTheDocument();
    expect(screen.getByText(/no messages yet/i)).toBeInTheDocument();
    expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    expect(screen.getByTestId("chat-send")).toBeInTheDocument();
  });

  it("submitting input invokes chat_send_message with the body", async () => {
    invokeMock.mockResolvedValueOnce(makeSendResult("hello"));
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "hello");
    await user.click(screen.getByTestId("chat-send"));

    expect(invokeMock).toHaveBeenCalledWith("chat_send_message", {
      sessionId: null,
      workspaceId: "default",
      content: "hello",
    });
  });

  it("appends user message and reply to the message list after send", async () => {
    invokeMock.mockResolvedValueOnce(makeSendResult("hello"));
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "hello");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getAllByTestId("chat-message-user")).toHaveLength(1);
      expect(screen.getAllByTestId("chat-message-assistant")).toHaveLength(1);
    });

    expect(screen.getByTestId("chat-message-user")).toHaveTextContent("hello");
    expect(screen.getByTestId("chat-message-assistant")).toHaveTextContent(
      "Echo: hello",
    );
  });

  it("clears the input after successful send", async () => {
    invokeMock.mockResolvedValueOnce(makeSendResult("hello"));
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "hello");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-input")).toHaveValue("");
    });
  });

  it("typing a /plan command shows a [STUB] system message", async () => {
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "/plan #42");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
    });
    expect(screen.getByTestId("chat-message-system")).toHaveTextContent("[STUB]");
  });

  it("displays error when invoke rejects", async () => {
    invokeMock.mockRejectedValueOnce("content is empty");
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "   bad   ");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-error")).toBeInTheDocument();
    });
    expect(screen.getByTestId("chat-error")).toHaveTextContent(/content is empty/i);
  });
});
