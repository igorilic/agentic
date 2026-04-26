import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import StartRunForm from "../components/StartRunForm";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("StartRunForm", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders inputs and buttons", () => {
    render(<StartRunForm />);
    expect(screen.getByTestId("script-path-input")).toBeInTheDocument();
    expect(screen.getByTestId("delay-ms-input")).toBeInTheDocument();
    expect(screen.getByTestId("start-button")).not.toBeDisabled();
    expect(screen.getByTestId("cancel-button")).toBeDisabled();
  });

  it("blocks Start when script path is empty", async () => {
    const user = userEvent.setup();
    render(<StartRunForm />);
    await user.click(screen.getByTestId("start-button"));
    expect(invokeMock).not.toHaveBeenCalled();
    expect(screen.getByTestId("error-message")).toHaveTextContent(/required/i);
  });

  it("calls start_scripted_run with the entered path and delay", async () => {
    invokeMock.mockResolvedValueOnce("01abcdef-test-run-id");
    const user = userEvent.setup();
    render(<StartRunForm />);

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
    render(<StartRunForm />);
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
    render(<StartRunForm />);

    await user.type(screen.getByTestId("script-path-input"), "/x/script.json");
    await user.click(screen.getByTestId("start-button"));
    expect(await screen.findByTestId("active-run-id")).toHaveTextContent(/run-123/);

    await user.click(screen.getByTestId("cancel-button"));
    expect(invokeMock).toHaveBeenLastCalledWith("cancel_run", { runId: "run-123" });
    // After cancel, active-run-id clears and Start becomes enabled again.
    expect(screen.queryByTestId("active-run-id")).not.toBeInTheDocument();
    expect(screen.getByTestId("start-button")).not.toBeDisabled();
  });
});
