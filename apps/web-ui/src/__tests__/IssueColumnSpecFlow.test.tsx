import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import IssueColumn from "../components/IssueColumn";
import type { ActionItem, IssueTicket } from "../types/pipeline";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const ticket: IssueTicket = {
  id: "AGT-204",
  title: "Add multi-tenant rate limiting",
  labels: [],
  body: [],
  acceptance: [],
};

const actionItems: ActionItem[] = [
  { id: "a1", kind: "issue", title: "Doc the change", fromAgent: "docs" },
];

afterEach(() => invokeMock.mockReset());

describe("IssueColumn — Create spec flow (W.6.6)", () => {
  beforeEach(() => localStorage.clear());

  it("Create spec button opens the SpecDialog", () => {
    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
      />
    );
    expect(screen.queryByTestId("spec-dialog")).toBeNull();

    fireEvent.click(screen.getByTestId("issue-create-spec"));

    expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();
  });

  it("Submit invokes start_ticket_run with correct args", async () => {
    invokeMock.mockResolvedValueOnce({ run_id: "run-123" });
    const user = userEvent.setup();

    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));

    await user.type(screen.getByTestId("spec-dialog-title-input"), "New spec");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
        ticket: "New spec",
        backend: "claude-code",
        model: null,
      });
    });
  });

  it("Dialog closes after successful submit", async () => {
    invokeMock.mockResolvedValueOnce({ run_id: "run-123" });
    const user = userEvent.setup();

    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));
    await user.type(screen.getByTestId("spec-dialog-title-input"), "New spec");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    await waitFor(() => {
      expect(screen.queryByTestId("spec-dialog")).toBeNull();
    });
  });

  it("Cancel closes dialog without invoking", () => {
    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));
    expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("spec-dialog-cancel"));

    expect(screen.queryByTestId("spec-dialog")).toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("Backdrop close doesn't invoke and closes dialog", () => {
    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));
    expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("spec-dialog-backdrop"));

    expect(screen.queryByTestId("spec-dialog")).toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("Esc close doesn't invoke and closes dialog", async () => {
    const user = userEvent.setup();

    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));
    expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();

    // Focus the dialog element and press Escape
    const dialog = screen.getByTestId("spec-dialog");
    dialog.focus();
    await user.keyboard("{Escape}");

    await waitFor(() => {
      expect(screen.queryByTestId("spec-dialog")).toBeNull();
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("Failed IPC keeps dialog open with title intact", async () => {
    invokeMock.mockRejectedValueOnce(new Error("network"));
    const user = userEvent.setup();

    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));
    await user.type(screen.getByTestId("spec-dialog-title-input"), "New spec");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });

    // Dialog remains open
    expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();
    // Title input retains value
    expect(screen.getByTestId("spec-dialog-title-input")).toHaveValue("New spec");
  });

  it("calls onTicketRunStarted with { runId, ticketLabel, description: undefined } when body is empty", async () => {
    invokeMock.mockResolvedValueOnce("run-abc");
    const onTicketRunStarted = vi.fn();
    const user = userEvent.setup();

    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
        onTicketRunStarted={onTicketRunStarted}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));
    await user.type(screen.getByTestId("spec-dialog-title-input"), "New spec");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    await waitFor(() => {
      expect(onTicketRunStarted).toHaveBeenCalledWith({
        runId: "run-abc",
        ticketLabel: "New spec",
        description: undefined,
      });
    });
  });

  it("calls onTicketRunStarted with description populated when body is non-empty", async () => {
    invokeMock.mockResolvedValueOnce("run-with-body");
    const onTicketRunStarted = vi.fn();
    const user = userEvent.setup();

    render(
      <IssueColumn
        ticket={ticket}
        runState="completed"
        actionItems={actionItems}
        onTicketRunStarted={onTicketRunStarted}
      />
    );

    fireEvent.click(screen.getByTestId("issue-create-spec"));
    await user.type(screen.getByTestId("spec-dialog-title-input"), "New spec");
    await user.type(screen.getByTestId("spec-dialog-body-textarea"), "Some description text");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    await waitFor(() => {
      expect(onTicketRunStarted).toHaveBeenCalledWith({
        runId: "run-with-body",
        ticketLabel: "New spec",
        description: "Some description text",
      });
    });
  });
});
