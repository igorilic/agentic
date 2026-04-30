import { act, render, screen, waitFor, fireEvent } from "@testing-library/react";
import ChatPane from "../components/ChatPane";

const invokeMock = vi.fn();
let capturedMentionHandler: ((event: { payload: unknown }) => void) | null = null;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_channel: string, handler: (event: { payload: unknown }) => void) => {
    capturedMentionHandler = handler;
    return () => {};
  }),
}));

describe("ChatPane @mention routing", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    capturedMentionHandler = null;
  });

  it("submitting @architect hello calls invoke(mention_agent, { agent, body })", async () => {
    invokeMock.mockResolvedValueOnce({
      run_id: "01abc",
      agent: "architect",
      dispatched: false,
    });
    render(<ChatPane />);

    // Use fireEvent to set the textarea value directly (avoids mention-popover
    // keyboard interception that occurs when userEvent types character-by-character).
    const textarea = screen.getByTestId("chat-input");
    fireEvent.change(textarea, { target: { value: "@architect hello" } });

    // Trigger send via Cmd+Enter
    fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("mention_agent", {
        agent: "architect",
        body: "hello",
      });
    });
  });

  it("submitting @bad-input (no body) shows a parse error system message", async () => {
    render(<ChatPane />);

    const textarea = screen.getByTestId("chat-input");
    // Use fireEvent to bypass the mention popover keyboard handling
    fireEvent.change(textarea, { target: { value: "@bad-input" } });
    fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

    await waitFor(() => {
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
    });
    const systemMsg = screen.getByTestId("chat-message-system");
    expect(systemMsg.textContent).toMatch(/bad-input/i);
  });

  it("renders mention envelopes received on agentic://mention-event as agent-variant chat messages", async () => {
    render(<ChatPane />);

    await waitFor(() => {
      expect(capturedMentionHandler).not.toBeNull();
    });

    act(() => {
      capturedMentionHandler!({
        payload: {
          schema_version: 1,
          event_id: "m1",
          run_id: "run-1",
          step_id: null,
          timestamp_ms: 1,
          event: {
            type: "TextDelta",
            data: { content: "[STUB] @architect received: hello" },
          },
        },
      });
    });

    await waitFor(() => {
      // Mention envelopes are now rendered via ChatColumn as agent-variant
      // ChatMessage components; the old chat-message-mention testid is replaced
      // by chat-message-agent (the agent-variant's testid from ChatMessage.tsx).
      expect(screen.getByTestId("chat-message-agent")).toBeInTheDocument();
    });
    expect(screen.getByTestId("chat-message-agent")).toHaveTextContent(
      "[STUB] @architect received: hello",
    );
  });
});
