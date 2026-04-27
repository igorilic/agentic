import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import StartRunForm from "../components/StartRunForm";
import type { EventEnvelope } from "../types/event";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

/**
 * Wrapper that provides lifted state for StartRunForm — mirrors App.tsx usage.
 */
function ControlledStartRunForm({
  events = [],
  initialRunId,
}: {
  events?: EventEnvelope[];
  initialRunId?: string;
}) {
  const [activeRunId, setActiveRunId] = useState<string | undefined>(initialRunId);
  return (
    <StartRunForm
      events={events}
      activeRunId={activeRunId}
      onActiveRunIdChange={setActiveRunId}
    />
  );
}

describe("StartRunForm", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders inputs and buttons", () => {
    render(<ControlledStartRunForm />);
    expect(screen.getByTestId("script-path-input")).toBeInTheDocument();
    expect(screen.getByTestId("delay-ms-input")).toBeInTheDocument();
    expect(screen.getByTestId("start-button")).not.toBeDisabled();
    expect(screen.getByTestId("cancel-button")).toBeDisabled();
  });

  it("blocks Start when script path is empty", async () => {
    const user = userEvent.setup();
    render(<ControlledStartRunForm />);
    await user.click(screen.getByTestId("start-button"));
    expect(invokeMock).not.toHaveBeenCalled();
    expect(screen.getByTestId("error-message")).toHaveTextContent(/required/i);
  });

  it("calls start_scripted_run with the entered path and delay", async () => {
    invokeMock.mockResolvedValueOnce("01abcdef-test-run-id");
    const user = userEvent.setup();
    render(<ControlledStartRunForm />);

    await user.type(screen.getByTestId("script-path-input"), "/tmp/script.json");
    await user.clear(screen.getByTestId("delay-ms-input"));
    await user.type(screen.getByTestId("delay-ms-input"), "50");
    await user.click(screen.getByTestId("start-button"));

    expect(invokeMock).toHaveBeenCalledWith("start_scripted_run", {
      scriptPath: "/tmp/script.json",
      delayMs: 50,
    });
    // After successful start, activeRunId should appear and Start should disable.
    expect(await screen.findByTestId("active-run-id")).toHaveTextContent(/01abcdef/);
    expect(screen.getByTestId("start-button")).toBeDisabled();
    expect(screen.getByTestId("cancel-button")).not.toBeDisabled();
  });

  it("displays error when invoke rejects", async () => {
    invokeMock.mockRejectedValueOnce("script path outside allowed scope: /etc/passwd");
    const user = userEvent.setup();
    render(<ControlledStartRunForm />);
    await user.type(screen.getByTestId("script-path-input"), "/etc/passwd");
    await user.click(screen.getByTestId("start-button"));

    expect(await screen.findByTestId("error-message")).toHaveTextContent(/outside allowed scope/);
    // No active run on failure.
    expect(screen.queryByTestId("active-run-id")).not.toBeInTheDocument();
  });

  it("calls cancel_run with the active run_id when Cancel is clicked", async () => {
    invokeMock.mockResolvedValueOnce("run-123");
    invokeMock.mockResolvedValueOnce(true);
    const user = userEvent.setup();
    render(<ControlledStartRunForm />);

    await user.type(screen.getByTestId("script-path-input"), "/x/script.json");
    await user.click(screen.getByTestId("start-button"));
    expect(await screen.findByTestId("active-run-id")).toHaveTextContent(/run-123/);

    await user.click(screen.getByTestId("cancel-button"));
    expect(invokeMock).toHaveBeenLastCalledWith("cancel_run", { runId: "run-123" });
    // After cancel, active-run-id clears and Start becomes enabled again.
    expect(screen.queryByTestId("active-run-id")).not.toBeInTheDocument();
    expect(screen.getByTestId("start-button")).not.toBeDisabled();
  });

  it("clears activeRunId when RunComplete event arrives for it", async () => {
    invokeMock.mockResolvedValueOnce("run-xyz");
    const user = userEvent.setup();
    const { rerender } = render(<ControlledStartRunForm events={[]} />);

    await user.type(screen.getByTestId("script-path-input"), "/p");
    await user.click(screen.getByTestId("start-button"));
    expect(await screen.findByTestId("active-run-id")).toHaveTextContent("run-xyz");

    // Rerender via the uncontrolled wrapper is not possible here since we need
    // the wrapper to pick up the events. Render the form directly with the
    // controlled props instead for this test.
    rerender(
      <StartRunForm
        events={[{
          schema_version: 1,
          event_id: "ev-1",
          run_id: "run-xyz",
          step_id: null,
          timestamp_ms: 1700000000000,
          event: { type: "RunComplete", data: {} },
        } satisfies EventEnvelope]}
        activeRunId="run-xyz"
        onActiveRunIdChange={() => {}}
      />,
    );

    // After RunComplete, the form calls onActiveRunIdChange(undefined) — but since
    // we passed a no-op here, we assert the callback was invoked conceptually.
    // The form should render without active-run-id if it were controlling itself.
    // Instead test the F1 effect via the controlled wrapper: render it with
    // activeRunId set and confirm RunComplete clears via the state wrapper.
    // We re-render back to controlled wrapper with the RunComplete event.
    const onActiveRunIdChange = vi.fn();
    rerender(
      <StartRunForm
        events={[{
          schema_version: 1,
          event_id: "ev-1",
          run_id: "run-xyz",
          step_id: null,
          timestamp_ms: 1700000000000,
          event: { type: "RunComplete", data: {} },
        } satisfies EventEnvelope]}
        activeRunId="run-xyz"
        onActiveRunIdChange={onActiveRunIdChange}
      />,
    );
    expect(onActiveRunIdChange).toHaveBeenCalledWith(undefined);
  });

  it("ignores RunComplete for a different run_id", async () => {
    const onActiveRunIdChange = vi.fn();
    render(
      <StartRunForm
        events={[{
          schema_version: 1,
          event_id: "ev-1",
          run_id: "OTHER-RUN",
          step_id: null,
          timestamp_ms: 1,
          event: { type: "RunComplete", data: {} },
        } satisfies EventEnvelope]}
        activeRunId="run-xyz"
        onActiveRunIdChange={onActiveRunIdChange}
      />,
    );

    // RunComplete for a different run should not call onActiveRunIdChange.
    expect(onActiveRunIdChange).not.toHaveBeenCalled();
    expect(screen.getByTestId("active-run-id")).toHaveTextContent("run-xyz");
  });

  it("disables both buttons while start is in-flight", async () => {
    let resolveInvoke: ((value: string) => void) | undefined;
    invokeMock.mockImplementationOnce(
      () => new Promise<string>((resolve) => { resolveInvoke = resolve; }),
    );
    const user = userEvent.setup();
    render(<ControlledStartRunForm />);

    await user.type(screen.getByTestId("script-path-input"), "/p");
    await user.click(screen.getByTestId("start-button"));

    expect(screen.getByTestId("start-button")).toBeDisabled();
    expect(screen.getByTestId("cancel-button")).toBeDisabled();

    resolveInvoke!("run-1");
    await screen.findByTestId("active-run-id");
    expect(screen.getByTestId("cancel-button")).not.toBeDisabled();
  });

  it("clamps negative delayMs to 0 before invoking", async () => {
    invokeMock.mockResolvedValueOnce("run-1");
    const user = userEvent.setup();
    render(<ControlledStartRunForm />);

    await user.type(screen.getByTestId("script-path-input"), "/p");
    await user.clear(screen.getByTestId("delay-ms-input"));
    await user.type(screen.getByTestId("delay-ms-input"), "-500");
    await user.click(screen.getByTestId("start-button"));

    expect(invokeMock).toHaveBeenCalledWith("start_scripted_run", {
      scriptPath: "/p",
      delayMs: 0,
    });
  });

  it("shows error when invoke returns a non-string", async () => {
    invokeMock.mockResolvedValueOnce({ unexpected: "shape" });
    const user = userEvent.setup();
    render(<ControlledStartRunForm />);

    await user.type(screen.getByTestId("script-path-input"), "/p");
    await user.click(screen.getByTestId("start-button"));

    expect(await screen.findByTestId("error-message")).toHaveTextContent(/unexpected return/i);
    expect(screen.queryByTestId("active-run-id")).not.toBeInTheDocument();
  });
});
