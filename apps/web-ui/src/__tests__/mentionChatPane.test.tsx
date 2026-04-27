import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "@architect hello");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("mention_agent", {
        agent: "architect",
        body: "hello",
      });
    });
  });

  it("submitting @bad-input (no body) shows a parse error system message", async () => {
    const user = userEvent.setup();
    render(<ChatPane />);

    await user.type(screen.getByTestId("chat-input"), "@bad-input");
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
    });
    const systemMsg = screen.getByTestId("chat-message-system");
    expect(systemMsg.textContent).toMatch(/bad-input/i);
  });

  it("renders mention envelopes received on agentic://mention-event as chat messages", async () => {
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
      expect(screen.getByTestId("chat-message-mention")).toBeInTheDocument();
    });
    expect(screen.getByTestId("chat-message-mention")).toHaveTextContent(
      "[STUB] @architect received: hello",
    );
  });
});
