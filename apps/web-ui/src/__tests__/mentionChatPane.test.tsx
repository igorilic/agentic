import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ChatPane from "../components/ChatPane";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("ChatPane @mention routing", () => {
  beforeEach(() => {
    invokeMock.mockReset();
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
});
