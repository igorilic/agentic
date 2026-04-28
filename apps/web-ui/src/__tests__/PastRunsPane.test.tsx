import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import PastRunsPane from "../components/PastRunsPane";
import type { RunSummary } from "../types/run_summary";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function makeRun(over: Partial<RunSummary> = {}): RunSummary {
  return {
    id: "01abc1234567",
    workspace_id: "ws-test",
    status: "completed",
    backend: "claude-code",
    model: "sonnet",
    ticket_label: "fix the auth race",
    started_at: 1_700_000_000_000,
    completed_at: 1_700_000_000_500,
    duration_ms: 500,
    ...over,
  };
}

describe("PastRunsPane", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("on mount, calls list_runs and renders the empty state when none exist", async () => {
    invokeMock.mockResolvedValueOnce([]);
    render(<PastRunsPane />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("list_runs", { limit: 50 });
    });
    expect(screen.getByText(/no past runs/i)).toBeInTheDocument();
  });

  it("renders one row per run with truncated id, status badge, and ticket label", async () => {
    invokeMock.mockResolvedValueOnce([
      makeRun({ id: "01aaaaa11111", ticket_label: "alpha", status: "completed" }),
      makeRun({ id: "01bbbbb22222", ticket_label: "beta", status: "failed" }),
    ]);

    render(<PastRunsPane />);
    await waitFor(() => {
      expect(screen.getAllByTestId(/past-run-row-/)).toHaveLength(2);
    });
    expect(screen.getByTestId("past-run-row-01aaaaa11111")).toHaveTextContent(
      "alpha",
    );
    expect(screen.getByTestId("past-run-row-01aaaaa11111")).toHaveTextContent(
      "completed",
    );
    expect(screen.getByTestId("past-run-row-01bbbbb22222")).toHaveTextContent(
      "failed",
    );
  });

  it("clicking a row notifies the parent via onSelectRun with the full id", async () => {
    invokeMock.mockResolvedValueOnce([makeRun({ id: "01ccccc33333" })]);
    const onSelect = vi.fn();
    const user = userEvent.setup();

    render(<PastRunsPane onSelectRun={onSelect} />);
    await waitFor(() => {
      expect(screen.getByTestId("past-run-row-01ccccc33333")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("past-run-row-01ccccc33333"));
    expect(onSelect).toHaveBeenCalledWith("01ccccc33333");
  });

  it("surfaces a load error when list_runs rejects", async () => {
    invokeMock.mockRejectedValueOnce("db locked");
    render(<PastRunsPane />);

    await waitFor(() => {
      expect(screen.getByTestId("past-runs-error")).toBeInTheDocument();
    });
    expect(screen.getByTestId("past-runs-error")).toHaveTextContent(/db locked/);
  });

  it("refresh button re-invokes list_runs", async () => {
    invokeMock.mockResolvedValueOnce([]).mockResolvedValueOnce([]);
    const user = userEvent.setup();
    render(<PastRunsPane />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByTestId("past-runs-refresh"));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(2);
    });
  });
});
