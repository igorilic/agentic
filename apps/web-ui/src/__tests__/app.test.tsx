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
  beforeEach(() => {
    // StartRunForm is only rendered in DEV mode — make the assumption explicit
    // so these tests survive a future global env change.
    vi.stubEnv("DEV", true);
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

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

  it("renders the StartRunForm above the EventList", async () => {
    render(<App />);
    // StartRunForm is lazy-loaded inside a Suspense boundary — use findBy* (async).
    expect(await screen.findByTestId("start-run-form")).toBeInTheDocument();
  });

  it("renders the cockpit stepper", () => {
    render(<App />);
    expect(screen.getByTestId("cockpit-stepper")).toBeInTheDocument();
  });

  it("renders the chat pane", () => {
    render(<App />);
    expect(screen.getByTestId("chat-pane")).toBeInTheDocument();
  });

  it("renders the findings table", () => {
    render(<App />);
    expect(screen.getByTestId("findings-table")).toBeInTheDocument();
  });

  it("renders the settings pane", () => {
    render(<App />);
    expect(screen.getByTestId("settings-pane")).toBeInTheDocument();
  });

  it("renders the past runs pane", () => {
    render(<App />);
    expect(screen.getByTestId("past-runs-pane")).toBeInTheDocument();
  });
});
