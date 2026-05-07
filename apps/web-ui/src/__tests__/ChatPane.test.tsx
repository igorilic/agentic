import { render, screen, waitFor, fireEvent } from "@testing-library/react";
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
      // assistant role messages render as chat-message-agent in the new ChatColumn
      expect(screen.getAllByTestId("chat-message-agent")).toHaveLength(1);
    });

    expect(screen.getByTestId("chat-message-user")).toHaveTextContent("hello");
    expect(screen.getByTestId("chat-message-agent")).toHaveTextContent(
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

  it("typing a /plan command invokes start_ticket_run and notifies the parent", async () => {
    invokeMock.mockResolvedValueOnce("01abc"); // start_ticket_run returns run_id
    const onStart = vi.fn();
    const user = userEvent.setup();
    render(<ChatPane onTicketRunStarted={onStart} pipelineAgents={["architect", "tdd-developer", "qa", "reviewer"]} />);

    await user.type(screen.getByTestId("chat-input"), "/plan #42");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
        ticket: "#42",
        backend: "claude-code",
        model: null,
        agents: expect.arrayContaining(["architect"]),
      });
    });
    await waitFor(() => {
      expect(onStart).toHaveBeenCalledWith({ runId: "01abc", ticketLabel: "#42", description: undefined });
    });
    // System message confirms the run started.
    expect(screen.getByTestId("chat-message-system")).toHaveTextContent(
      /started run 01abc/i,
    );
  });

  it("/plan splits on first dot — first sentence is ticketLabel, rest is description", async () => {
    invokeMock.mockResolvedValueOnce("run-split-1");
    const onStart = vi.fn();
    const user = userEvent.setup();
    render(<ChatPane onTicketRunStarted={onStart} />);

    await user.type(
      screen.getByTestId("chat-input"),
      "/plan Add rate limiting. Pro tier issue.",
    );
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(onStart).toHaveBeenCalledWith({
        runId: "run-split-1",
        ticketLabel: "Add rate limiting",
        description: "Pro tier issue.",
      });
    });
  });

  it("/plan with no dot sends full text as ticketLabel with undefined description", async () => {
    invokeMock.mockResolvedValueOnce("run-nodot-1");
    const onStart = vi.fn();
    const user = userEvent.setup();
    render(<ChatPane onTicketRunStarted={onStart} />);

    await user.type(screen.getByTestId("chat-input"), "/plan create palindrome");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(onStart).toHaveBeenCalledWith({
        runId: "run-nodot-1",
        ticketLabel: "create palindrome",
        description: undefined,
      });
    });
  });

  it("uses backend from useBackend() in /plan dispatch when no explicit flag", async () => {
    localStorage.setItem("agentic.backend", "copilot-cli");
    invokeMock.mockResolvedValueOnce("run-hook-1");
    const user = userEvent.setup();
    render(<ChatPane pipelineAgents={["architect", "tdd-developer", "qa", "reviewer"]} />);

    await user.type(screen.getByTestId("chat-input"), "/plan #42 ticket text");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
        ticket: "#42 ticket text",
        backend: "copilot-cli",
        model: null,
        agents: expect.arrayContaining(["architect"]),
      });
    });
  });

  it("/plan --backend=copilot-cli forwards the parsed backend to the IPC", async () => {
    invokeMock.mockResolvedValueOnce("01def");
    const user = userEvent.setup();
    render(<ChatPane pipelineAgents={["architect", "tdd-developer", "qa", "reviewer"]} />);

    await user.type(
      screen.getByTestId("chat-input"),
      "/plan --backend=copilot-cli implement export",
    );
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
        ticket: "implement export",
        backend: "copilot-cli",
        model: null,
        agents: expect.arrayContaining(["architect"]),
      });
    });
    expect(screen.getByTestId("chat-message-system")).toHaveTextContent(
      /\[copilot-cli\]/,
    );
  });

  it("/plan --backend=foo shows an actionable parse error", async () => {
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "/plan --backend=foo #42");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
    });
    expect(screen.getByTestId("chat-message-system")).toHaveTextContent(/foo/);
    expect(screen.getByTestId("chat-message-system")).toHaveTextContent(/allowed/);
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

  // Responsive layout: chat pane should fill its parent column, not impose a fixed height.
  it("chat-pane fills its parent column (h-full flex-1) instead of using a fixed h-96", () => {
    render(<ChatPane />);
    const pane = screen.getByTestId("chat-pane");
    expect(pane.className).toMatch(/h-full/);
    expect(pane.className).toMatch(/flex-1/);
    expect(pane.className).not.toMatch(/h-96/);
  });

  it("chat-messages scrollback area retains overflow-y-auto after height fix", () => {
    render(<ChatPane />);
    const messages = screen.getByTestId("chat-messages");
    expect(messages.className).toMatch(/overflow-y-auto/);
  });

  it("typing / into the textarea opens the slash popover", async () => {
    render(<ChatPane />);
    const textarea = screen.getByTestId("chat-input");

    fireEvent.change(textarea, { target: { value: "/" } });

    expect(screen.getByTestId("slash-popover")).toBeInTheDocument();
  });

  it("an assistant message renders with [data-testid='chat-message-agent'] and a data-agent attribute", async () => {
    invokeMock.mockResolvedValueOnce(makeSendResult("hi"));
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "hi");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-message-agent")).toBeInTheDocument();
    });
    const agentMsg = screen.getByTestId("chat-message-agent");
    expect(agentMsg).toHaveAttribute("data-agent");
  });

  // -------------------------------------------------------------------------
  // F.1.4: Pre-flight error surfacing
  // -------------------------------------------------------------------------

  it("surfaces start_ticket_run pre-flight errors as system messages without 'Command failed:' prefix", async () => {
    const preflightError = "pre-flight: `claude` not found on PATH. Install: https://claude.ai/cli";
    invokeMock.mockRejectedValueOnce(preflightError);
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "/plan #42 do thing");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
    });
    const msg = screen.getByTestId("chat-message-system").textContent ?? "";
    expect(msg).toContain("pre-flight:");
    expect(msg).not.toMatch(/^Command failed:/);
    expect(msg).toContain("https://claude.ai/cli");
  });

  it("wraps non-pre-flight errors in 'Command failed:' prefix", async () => {
    invokeMock.mockRejectedValueOnce("Some other backend error");
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "/plan #99 do thing");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
    });
    const msg = screen.getByTestId("chat-message-system").textContent ?? "";
    expect(msg).toContain("Command failed:");
    expect(msg).toContain("Some other backend error");
  });

  // -------------------------------------------------------------------------
  // GH #67 — slash branch calls chat_record_system_message for audit trail
  // -------------------------------------------------------------------------

  // -------------------------------------------------------------------------
  // GH #67 — dispatch-success and dispatch-error branches also call
  // chat_record_system_message for audit trail (regression guard)
  // -------------------------------------------------------------------------

  it("slash dispatch-success persists audit trail via chat_record_system_message", async () => {
    // start_ticket_run resolves first, then chat_record_system_message resolves.
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "start_ticket_run") return "run-audit-ok";
      if (cmd === "chat_record_system_message") {
        const content = (args?.content as string) ?? "";
        return {
          id: `sys-${Date.now()}`,
          session_id: "s",
          run_id: null,
          role: "system",
          content,
          metadata: null,
          created_at: Date.now(),
        };
      }
      return undefined;
    });

    const user = userEvent.setup();
    render(<ChatPane pipelineAgents={["architect"]} />);

    await user.type(screen.getByTestId("chat-input"), "/plan #42 do thing");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "chat_record_system_message",
        expect.objectContaining({
          content: expect.stringMatching(/started run run-audit-ok/i),
          workspaceId: "default",
        }),
      );
    });
  });

  it("slash dispatch-error persists audit trail via chat_record_system_message", async () => {
    // start_ticket_run rejects with a non-pre-flight error so the
    // "Command failed:" prefix is prepended before recordSystem is called.
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "start_ticket_run") throw "backend exploded";
      if (cmd === "chat_record_system_message") {
        const content = (args?.content as string) ?? "";
        return {
          id: `sys-${Date.now()}`,
          session_id: "s",
          run_id: null,
          role: "system",
          content,
          metadata: null,
          created_at: Date.now(),
        };
      }
      return undefined;
    });

    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "/plan #99 do thing");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "chat_record_system_message",
        expect.objectContaining({
          content: expect.stringContaining("Command failed:"),
          workspaceId: "default",
        }),
      );
    });

    // Also assert the error content itself is present
    const recordCall = invokeMock.mock.calls.find(
      ([cmd]) => cmd === "chat_record_system_message",
    );
    expect(recordCall).toBeDefined();
    expect(recordCall![1]).toMatchObject({
      content: expect.stringContaining("backend exploded"),
    });
  });

  it("slash parse error calls chat_record_system_message with the formatted error string", async () => {
    // /plan --backend=bad-format triggers a parse error (bad backend value)
    // recordSystem mock: return a system ChatMessage so the hook appends it.
    invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "chat_record_system_message") {
        const content = (args?.content as string) ?? "";
        return {
          id: `sys-${Date.now()}`,
          session_id: "s",
          run_id: null,
          role: "system",
          content,
          metadata: null,
          created_at: Date.now(),
        };
      }
      return undefined;
    });

    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "/plan --backend=bad-format #42");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("chat_record_system_message", expect.objectContaining({
        content: expect.stringContaining("bad-format"),
        workspaceId: "default",
      }));
    });
  });
});
