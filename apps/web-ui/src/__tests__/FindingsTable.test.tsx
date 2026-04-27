import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import FindingsTable from "../components/FindingsTable";
import type { Finding } from "../types/finding";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function makeFinding(overrides: Partial<Finding> = {}): Finding {
  return {
    id: "f1",
    run_id: "run1",
    step_id: "step1",
    severity: "warning",
    file_path: "src/main.rs",
    line: 42,
    message: "missing-error-handling",
    suggestion: null,
    triage: null,
    triaged_at: null,
    created_at: 200,
    ...overrides,
  };
}

describe("FindingsTable", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("renders an empty-state row when there are no findings", () => {
    render(<FindingsTable findings={[]} />);
    expect(screen.getByTestId("findings-table")).toBeInTheDocument();
    expect(screen.getByText(/no findings/i)).toBeInTheDocument();
  });

  it("renders one row per finding with severity, message, and file:line", () => {
    render(
      <FindingsTable
        findings={[
          makeFinding({ id: "f1", severity: "warning", message: "alpha", line: 10 }),
          makeFinding({
            id: "f2",
            severity: "error",
            message: "beta",
            file_path: "src/lib.rs",
            line: 20,
          }),
        ]}
      />,
    );

    const rows = screen.getAllByTestId(/finding-row-/);
    expect(rows).toHaveLength(2);
    expect(screen.getByTestId("finding-row-f1")).toHaveTextContent("alpha");
    expect(screen.getByTestId("finding-row-f1")).toHaveTextContent("warning");
    expect(screen.getByTestId("finding-row-f2")).toHaveTextContent("beta");
    expect(screen.getByTestId("finding-row-f2")).toHaveTextContent("src/lib.rs:20");
  });

  it("clicking [Tech-debt] invokes triage_finding with triage='tech-debt'", async () => {
    const user = userEvent.setup();
    render(<FindingsTable findings={[makeFinding({ id: "f1" })]} />);

    await user.click(screen.getByTestId("triage-tech-debt-f1"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("triage_finding", {
        findingId: "f1",
        triage: "tech-debt",
      });
    });
  });

  it("clicking [Fix] invokes triage_finding with triage='fix'", async () => {
    const user = userEvent.setup();
    render(<FindingsTable findings={[makeFinding({ id: "f1" })]} />);

    await user.click(screen.getByTestId("triage-fix-f1"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("triage_finding", {
        findingId: "f1",
        triage: "fix",
      });
    });
  });

  it("clicking [Ignore] invokes triage_finding with triage='ignore'", async () => {
    const user = userEvent.setup();
    render(<FindingsTable findings={[makeFinding({ id: "f1" })]} />);

    await user.click(screen.getByTestId("triage-ignore-f1"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("triage_finding", {
        findingId: "f1",
        triage: "ignore",
      });
    });
  });

  it("after a successful triage, the row reflects the new triage state", async () => {
    const user = userEvent.setup();
    render(<FindingsTable findings={[makeFinding({ id: "f1", triage: null })]} />);

    await user.click(screen.getByTestId("triage-fix-f1"));

    await waitFor(() => {
      expect(screen.getByTestId("finding-row-f1")).toHaveTextContent(/fix/i);
    });
  });

  it("renders triage-error and re-enables buttons when invoke rejects", async () => {
    invokeMock.mockRejectedValueOnce("finding not found: f1");

    const user = userEvent.setup();
    render(<FindingsTable findings={[makeFinding({ id: "f1" })]} />);

    await user.click(screen.getByTestId("triage-fix-f1"));

    await waitFor(() => {
      expect(screen.getByTestId("triage-error-f1")).toBeInTheDocument();
    });
    expect(screen.getByTestId("triage-error-f1")).toHaveTextContent(/finding not found/i);
    // After failure the buttons must be enabled again so the user can retry.
    expect(screen.getByTestId("triage-fix-f1")).not.toBeDisabled();
    // No optimistic override should have been applied for the failed call.
    expect(screen.queryByTestId("triage-badge-f1")).toBeNull();
  });

  it("disables triage buttons while invoke is in flight", async () => {
    let resolveInvoke: (() => void) | undefined;
    invokeMock.mockImplementationOnce(
      () => new Promise<void>((resolve) => { resolveInvoke = resolve; }),
    );

    const user = userEvent.setup();
    render(<FindingsTable findings={[makeFinding({ id: "f1" })]} />);

    await user.click(screen.getByTestId("triage-fix-f1"));

    // While pending, all triage buttons for this row should be disabled.
    await waitFor(() => {
      expect(screen.getByTestId("triage-fix-f1")).toBeDisabled();
      expect(screen.getByTestId("triage-tech-debt-f1")).toBeDisabled();
      expect(screen.getByTestId("triage-ignore-f1")).toBeDisabled();
    });

    resolveInvoke!();
  });
});
