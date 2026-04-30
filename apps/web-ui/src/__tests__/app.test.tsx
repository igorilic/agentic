import { render, screen } from "@testing-library/react";
import App from "../App";

// Mock the Tauri APIs since they're not available in jsdom.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
// Default invoke mock returns [] for all IPC commands that fetch lists.
// Commands like list_runs, list_auth_accounts, and list_findings all return
// Vec<T> on the backend which serialises to [] (never null/undefined).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// HeaderBar uses useTheme which calls window.matchMedia — stub it for jsdom.
function stubMatchMedia() {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

describe("App", () => {
  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("renders the app shell header", () => {
    render(<App />);
    expect(screen.getByTestId("app-shell-header")).toBeInTheDocument();
  });

  it("renders the app shell pipeline", () => {
    render(<App />);
    expect(screen.getByTestId("app-shell-pipeline")).toBeInTheDocument();
  });

  it("renders the chat pane", () => {
    render(<App />);
    expect(screen.getByTestId("chat-pane")).toBeInTheDocument();
  });

  it("renders the event-list (inside ActivityColumn)", () => {
    render(<App />);
    expect(screen.getByTestId("event-list")).toBeInTheDocument();
  });

  it("renders the issue column", () => {
    render(<App />);
    expect(screen.getByTestId("issue-column")).toBeInTheDocument();
  });

  it("does NOT render the standalone cockpit stepper", () => {
    render(<App />);
    expect(screen.queryByTestId("cockpit-stepper")).toBeNull();
  });

  it("does NOT render a top-level findings-table", () => {
    render(<App />);
    expect(screen.queryByTestId("findings-table")).toBeNull();
  });

  it("renders the Agentic brand", () => {
    render(<App />);
    expect(screen.getByText("Agentic")).toBeInTheDocument();
  });
});
