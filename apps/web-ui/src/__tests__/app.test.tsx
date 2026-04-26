import { render, screen } from "@testing-library/react";
import App from "../App";

// Mock the Tauri APIs since they're not available in jsdom.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe("App", () => {
  it("renders the Agentic heading", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { level: 1, name: /agentic/i }),
    ).toBeInTheDocument();
  });

  it("renders the empty event-list state when no events", () => {
    render(<App />);
    expect(screen.getByText(/no events yet/i)).toBeInTheDocument();
  });
});
