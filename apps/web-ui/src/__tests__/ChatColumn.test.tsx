import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, afterEach, beforeEach } from "vitest";
import ChatColumn from "../components/ChatColumn";
import type { ChatColumnProps } from "../components/ChatColumn";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

afterEach(() => invokeMock.mockReset());

// Minimal prop factory — keeps each test focused on only what it varies.
function makeProps(overrides: Partial<ChatColumnProps> = {}): ChatColumnProps {
  return {
    messages: [],
    systemMessages: [],
    mentionMessages: [],
    activeAgent: null,
    onSend: vi.fn(),
    error: null,
    ...overrides,
  };
}

describe("ChatColumn", () => {
  describe("header", () => {
    it("renders the header with 'Chat with pipeline' text", () => {
      render(<ChatColumn {...makeProps()} />);
      expect(screen.getByText("Chat with pipeline")).toBeInTheDocument();
    });

    it("shows active-agent chip when activeAgent is set", () => {
      render(<ChatColumn {...makeProps({ activeAgent: "developer" })} />);
      expect(screen.getByTestId("chat-column-active-chip")).toBeInTheDocument();
      expect(screen.getByTestId("chat-column-active-chip")).toHaveTextContent(
        "developer is responding",
      );
    });

    it("hides active-agent chip when activeAgent is null", () => {
      render(<ChatColumn {...makeProps({ activeAgent: null })} />);
      expect(screen.queryByTestId("chat-column-active-chip")).toBeNull();
    });
  });

  describe("message list", () => {
    it("renders the chat-messages container", () => {
      render(<ChatColumn {...makeProps()} />);
      expect(screen.getByTestId("chat-messages")).toBeInTheDocument();
    });

    it("renders a user message via chat-message-user testid", () => {
      render(
        <ChatColumn
          {...makeProps({
            messages: [
              {
                id: "m1",
                session_id: "s1",
                run_id: null,
                role: "user",
                content: "hello from user",
                metadata: null,
                created_at: 1000,
              },
            ],
          })}
        />,
      );
      expect(screen.getByTestId("chat-message-user")).toBeInTheDocument();
      expect(screen.getByTestId("chat-message-user")).toHaveTextContent(
        "hello from user",
      );
    });

    it("renders an assistant message via chat-message-agent testid with data-agent attribute", () => {
      render(
        <ChatColumn
          {...makeProps({
            messages: [
              {
                id: "m2",
                session_id: "s1",
                run_id: null,
                role: "assistant",
                content: "reply from agent",
                metadata: null,
                created_at: 1001,
              },
            ],
          })}
        />,
      );
      const agentMsg = screen.getByTestId("chat-message-agent");
      expect(agentMsg).toBeInTheDocument();
      expect(agentMsg).toHaveAttribute("data-agent");
      expect(agentMsg).toHaveTextContent("reply from agent");
    });

    it("renders data-agent='developer' when assistant message has senderAgent='developer'", () => {
      render(
        <ChatColumn
          {...makeProps({
            messages: [
              {
                id: "m3",
                session_id: "s1",
                run_id: null,
                role: "assistant",
                senderAgent: "developer",
                content: "developer reply",
                metadata: null,
                created_at: 1002,
              },
            ],
          })}
        />,
      );
      const agentMsg = screen.getByTestId("chat-message-agent");
      expect(agentMsg).toHaveAttribute("data-agent", "developer");
    });

    it("renders system messages via chat-message-system testid", () => {
      render(
        <ChatColumn
          {...makeProps({
            systemMessages: ["System event happened"],
          })}
        />,
      );
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
      expect(screen.getByTestId("chat-message-system")).toHaveTextContent(
        "System event happened",
      );
    });

    it("renders mention messages as agent-variant with correct content", () => {
      render(
        <ChatColumn
          {...makeProps({
            mentionMessages: [
              { agent: "architect", body: "mention reply", t: "10:00" },
            ],
          })}
        />,
      );
      const agentMessages = screen.getAllByTestId("chat-message-agent");
      expect(agentMessages.length).toBeGreaterThanOrEqual(1);
      const mentionMsg = agentMessages.find((el) =>
        el.textContent?.includes("mention reply"),
      );
      expect(mentionMsg).toBeDefined();
    });
  });

  describe("form and composer testids", () => {
    it("renders chat-form testid", () => {
      render(<ChatColumn {...makeProps()} />);
      expect(screen.getByTestId("chat-form")).toBeInTheDocument();
    });

    it("renders chat-input testid (forwarded to ChatComposer textarea)", () => {
      render(<ChatColumn {...makeProps()} />);
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
    });

    it("renders chat-send testid (forwarded to ChatComposer send button)", () => {
      render(<ChatColumn {...makeProps()} />);
      expect(screen.getByTestId("chat-send")).toBeInTheDocument();
    });
  });

  describe("send interaction", () => {
    it("Cmd+Enter in the textarea calls onSend with the typed text", async () => {
      const onSend = vi.fn();
      render(<ChatColumn {...makeProps({ onSend })} />);
      const textarea = screen.getByTestId("chat-input");

      await userEvent.type(textarea, "test message");
      fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

      expect(onSend).toHaveBeenCalledWith("test message");
    });

    it("clicking send button calls onSend", async () => {
      const onSend = vi.fn();
      render(<ChatColumn {...makeProps({ onSend })} />);
      const textarea = screen.getByTestId("chat-input");
      const sendBtn = screen.getByTestId("chat-send");

      await userEvent.type(textarea, "hello");
      await userEvent.click(sendBtn);

      expect(onSend).toHaveBeenCalledWith("hello");
    });
  });

  describe("W.9.4 — New-spec affordance in ChatColumn", () => {
    beforeEach(() => localStorage.clear());

    it("renders chat-composer-new-spec button inside ChatColumn", () => {
      render(<ChatColumn {...makeProps()} />);
      expect(screen.getByTestId("chat-composer-new-spec")).toBeInTheDocument();
    });

    it("clicking new-spec button opens the SpecDialog", async () => {
      render(<ChatColumn {...makeProps()} />);
      expect(screen.queryByTestId("spec-dialog")).toBeNull();

      await userEvent.click(screen.getByTestId("chat-composer-new-spec"));

      expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();
    });

    it("typing a title and submitting calls invoke with start_ticket_run including agents", async () => {
      invokeMock.mockResolvedValueOnce({ run_id: "run-1" });
      const user = userEvent.setup();

      render(<ChatColumn {...makeProps()} />);

      await user.click(screen.getByTestId("chat-composer-new-spec"));
      await user.type(screen.getByTestId("spec-dialog-title-input"), "My spec");
      await user.click(screen.getByTestId("spec-dialog-submit"));

      await waitFor(() => {
        expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
          ticket: "My spec",
          backend: "claude-code",
          model: null,
          agents: expect.arrayContaining(["architect"]),
        });
      });
    });

    it("SpecDialog closes after successful submit", async () => {
      invokeMock.mockResolvedValueOnce({ run_id: "run-1" });
      const user = userEvent.setup();

      render(<ChatColumn {...makeProps()} />);

      await user.click(screen.getByTestId("chat-composer-new-spec"));
      await user.type(screen.getByTestId("spec-dialog-title-input"), "My spec");
      await user.click(screen.getByTestId("spec-dialog-submit"));

      await waitFor(() => {
        expect(screen.queryByTestId("spec-dialog")).toBeNull();
      });
    });

    it("SpecDialog stays open when IPC fails", async () => {
      invokeMock.mockRejectedValueOnce(new Error("network"));
      const user = userEvent.setup();

      render(<ChatColumn {...makeProps()} />);

      await user.click(screen.getByTestId("chat-composer-new-spec"));
      await user.type(screen.getByTestId("spec-dialog-title-input"), "My spec");
      await user.click(screen.getByTestId("spec-dialog-submit"));

      await waitFor(() => {
        expect(invokeMock).toHaveBeenCalledTimes(1);
      });

      expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();
    });

    it("calls onTicketRunStarted with { runId, ticketLabel, description: undefined } when body is empty", async () => {
      invokeMock.mockResolvedValueOnce("run-xyz");
      const onTicketRunStarted = vi.fn();
      const user = userEvent.setup();

      render(<ChatColumn {...makeProps({ onTicketRunStarted })} />);

      await user.click(screen.getByTestId("chat-composer-new-spec"));
      await user.type(screen.getByTestId("spec-dialog-title-input"), "My spec");
      await user.click(screen.getByTestId("spec-dialog-submit"));

      await waitFor(() => {
        expect(onTicketRunStarted).toHaveBeenCalledWith({
          runId: "run-xyz",
          ticketLabel: "My spec",
          description: undefined,
        });
      });
    });

    it("calls onTicketRunStarted with description populated when body is non-empty", async () => {
      invokeMock.mockResolvedValueOnce("run-with-desc");
      const onTicketRunStarted = vi.fn();
      const user = userEvent.setup();

      render(<ChatColumn {...makeProps({ onTicketRunStarted })} />);

      await user.click(screen.getByTestId("chat-composer-new-spec"));
      await user.type(screen.getByTestId("spec-dialog-title-input"), "My spec");
      await user.type(screen.getByTestId("spec-dialog-body-textarea"), "The body details");
      await user.click(screen.getByTestId("spec-dialog-submit"));

      await waitFor(() => {
        expect(onTicketRunStarted).toHaveBeenCalledWith({
          runId: "run-with-desc",
          ticketLabel: "My spec",
          description: "The body details",
        });
      });
    });

    it("does not throw when onTicketRunStarted is not provided", async () => {
      invokeMock.mockResolvedValueOnce("run-xyz");
      const user = userEvent.setup();

      // No onTicketRunStarted prop — existing behaviour must still work
      render(<ChatColumn {...makeProps()} />);

      await user.click(screen.getByTestId("chat-composer-new-spec"));
      await user.type(screen.getByTestId("spec-dialog-title-input"), "My spec");
      await user.click(screen.getByTestId("spec-dialog-submit"));

      await waitFor(() => {
        expect(screen.queryByTestId("spec-dialog")).toBeNull();
      });
    });
  });

  describe("error display", () => {
    it("shows error in chat-error when error prop is set", () => {
      render(
        <ChatColumn {...makeProps({ error: "Something went wrong" })} />,
      );
      expect(screen.getByTestId("chat-error")).toBeInTheDocument();
      expect(screen.getByTestId("chat-error")).toHaveTextContent(
        "Something went wrong",
      );
    });

    it("hides chat-error when error is null", () => {
      render(<ChatColumn {...makeProps({ error: null })} />);
      expect(screen.queryByTestId("chat-error")).toBeNull();
    });
  });
});
