import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import App from "../App";
import type { RunSummary } from "../types/run_summary";

// Mock Tauri event and core APIs to keep async hooks mount-safe.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Conditional invoke mock: list_runs returns a populated row so the
// end-to-end run-selection chain can be exercised. All other commands
// (list_findings, list_auth_accounts, useTauriEvents, …) return [] which
// is the correct empty serialisation of a Rust Vec<T>.
const invokeMock = vi.fn(async (cmd: string, _args?: unknown) => {
  if (cmd === "list_runs") {
    const run: RunSummary = {
      id: "run-abc",
      workspace_id: "default",
      status: "completed",
      backend: "claude-code",
      model: "sonnet",
      ticket_label: "Test ticket",
      started_at: 1_700_000_000_000,
      completed_at: 1_700_000_001_000,
      duration_ms: 1_000,
    };
    return [run];
  }
  return [];
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string, unknown?])),
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
    invokeMock.mockClear();
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

  it("selecting a run from the History tab triggers a findings refetch", async () => {
    render(<App />);

    // Open the settings modal.
    fireEvent.click(screen.getByTestId("header-settings"));

    // Switch to the History tab.
    fireEvent.click(screen.getByTestId("settings-tab-history"));

    // Wait for PastRunsPane to fetch list_runs and render the row.
    await waitFor(() =>
      expect(screen.getByTestId("past-run-row-run-abc")).toBeInTheDocument(),
    );

    // Click the run row — fires onSelectRun("run-abc").
    fireEvent.click(screen.getByTestId("past-run-row-run-abc"));

    // App's setFindingsRunId("run-abc") triggers useFindings which calls
    // invoke("list_findings", { runId: "run-abc" }).
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith(
        "list_findings",
        expect.objectContaining({ runId: "run-abc" }),
      ),
    );
  });
});
