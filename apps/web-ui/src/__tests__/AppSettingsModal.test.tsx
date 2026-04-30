import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import App from "../App";

// Mock Tauri event and core APIs to keep async hooks mount-safe.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
// All IPC commands (list_runs, list_findings, list_auth_accounts …) return []
// which is the correct empty serialisation of a Rust Vec<T>.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// HeaderBar → useTheme calls window.matchMedia — stub it for jsdom.
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

describe("App + SettingsModal integration (W.8.3)", () => {
  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("has no header-history button — history is only inside SettingsModal", () => {
    render(<App />);
    expect(screen.queryByTestId("header-history")).toBeNull();
  });

  it("settings click opens the modal with General tab active by default", () => {
    render(<App />);

    // Modal absent before interaction.
    expect(screen.queryByTestId("settings-modal")).toBeNull();

    fireEvent.click(screen.getByTestId("header-settings"));

    // Modal present, general tab active.
    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();
    expect(screen.getByTestId("settings-pane")).toBeInTheDocument();
    expect(screen.queryByTestId("past-runs-pane")).toBeNull();
  });

  it("switching to History tab renders PastRunsPane", async () => {
    render(<App />);
    fireEvent.click(screen.getByTestId("header-settings"));

    // Modal is open at general tab.
    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();

    // Click the History tab.
    fireEvent.click(screen.getByTestId("settings-tab-history"));

    // PastRunsPane mounts asynchronously (fetches list_runs on mount).
    await waitFor(() => {
      expect(screen.getByTestId("past-runs-pane")).toBeInTheDocument();
    });

    // General tab body is gone.
    expect(screen.queryByTestId("settings-pane")).toBeNull();
  });

  it("clicking the backdrop closes the modal", () => {
    render(<App />);
    fireEvent.click(screen.getByTestId("header-settings"));
    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("settings-modal-backdrop"));
    expect(screen.queryByTestId("settings-modal")).toBeNull();
  });

  it("clicking the close button closes the modal", () => {
    render(<App />);
    fireEvent.click(screen.getByTestId("header-settings"));
    expect(screen.getByTestId("settings-modal")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("settings-modal-close"));
    expect(screen.queryByTestId("settings-modal")).toBeNull();
  });
});
