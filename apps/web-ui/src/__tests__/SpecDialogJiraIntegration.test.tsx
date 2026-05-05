/**
 * F.2.5 — SpecDialog → IPC integration smoke test (Vitest / jsdom).
 *
 * Exercises the full Jira-pull flow without a real Tauri backend:
 *   user types key → clicks Pull → useJiraFetch calls invoke → DTO populates fields.
 *
 * Approach:
 * - Mock `invoke` from `@tauri-apps/api/core` at the IPC boundary.
 * - Do NOT mock `useJiraFetch` — let it call through to the mocked invoke.
 *   This is the integration: IPC mock → hook → dialog state → rendered fields.
 * - This catches wiring bugs that the unit tests for the hook and dialog miss.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, type Mock } from "vitest";
import React from "react";

// ---------------------------------------------------------------------------
// Module-level mocks — hoisted before imports
// ---------------------------------------------------------------------------

// Mock invoke at the Tauri IPC boundary.
// useJiraFetch is NOT mocked — it runs through its real implementation and
// calls this mocked invoke internally.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

// ---------------------------------------------------------------------------
// Named imports (resolved after vi.mock hoisting)
// ---------------------------------------------------------------------------
import { invoke } from "@tauri-apps/api/core";
import SpecDialog from "../components/SpecDialog";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeProps(overrides: Partial<React.ComponentProps<typeof SpecDialog>> = {}) {
  return {
    open: true,
    onClose: vi.fn(),
    onSubmit: vi.fn(),
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("SpecDialog → IPC integration smoke test", () => {
  const mockInvoke = vi.mocked(invoke) as Mock;

  beforeEach(() => {
    mockInvoke.mockReset();
  });

  // --------------------------------------------------------------------------
  // Test 1: end-to-end happy path — user pulls from Jira, fields populate
  // --------------------------------------------------------------------------

  it("end-to-end: user pulls from Jira, edits, submits", async () => {
    const user = userEvent.setup();

    // The Rust backend (F.2.1) already appends AC to body in the DTO.
    // So the IPC mock returns body WITHOUT the AC section (ac is separate).
    // SpecDialog appends the AC section in handlePull.
    mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "fetch_jira_ticket") {
        expect((args as { key: string }).key).toBe("PROJ-42");
        return {
          key: "PROJ-42",
          title: "Refactor X",
          body: "Why\nThis needs doing",
          ac: "AC text",
        };
      }
      return null;
    });

    const props = makeProps();
    render(<SpecDialog {...props} />);

    // Type a valid Jira key
    const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
    await user.type(keyInput, "PROJ-42");

    // Click Pull from Jira
    const pullButton = screen.getByTestId("spec-dialog-jira-pull-button");
    await user.click(pullButton);

    // Wait for title to be populated
    await waitFor(() => {
      const titleInput = screen.getByTestId("spec-dialog-title-input") as HTMLInputElement;
      expect(titleInput.value).toBe("Refactor X");
    });

    // Body must include the AC section appended by SpecDialog
    const bodyTextarea = screen.getByTestId("spec-dialog-body-textarea") as HTMLTextAreaElement;
    expect(bodyTextarea.value).toBe(
      "Why\nThis needs doing\n\n## Acceptance Criteria\nAC text",
    );

    // Append extra text to body
    await user.type(bodyTextarea, "\nExtra note");

    // Click Create & run
    await user.click(screen.getByTestId("spec-dialog-submit"));

    // onSubmit must be called with the populated title and augmented body
    expect(props.onSubmit).toHaveBeenCalledTimes(1);
    expect(props.onSubmit).toHaveBeenCalledWith(
      "Refactor X",
      "Why\nThis needs doing\n\n## Acceptance Criteria\nAC text\nExtra note",
    );

    // invoke was called exactly once (the Jira pull)
    expect(mockInvoke).toHaveBeenCalledWith("fetch_jira_ticket", { key: "PROJ-42" });
  });

  // --------------------------------------------------------------------------
  // Test 2: pull error does not block manual entry
  // --------------------------------------------------------------------------

  it("end-to-end: pull error does not block manual entry", async () => {
    const user = userEvent.setup();

    mockInvoke.mockRejectedValue(
      "missing environment variables: JIRA_URL, JIRA_USER_EMAIL, JIRA_API_TOKEN",
    );

    const props = makeProps();
    render(<SpecDialog {...props} />);

    // Type a valid key and attempt a pull
    await user.type(screen.getByTestId("spec-dialog-jira-key-input"), "PROJ-42");
    await user.click(screen.getByTestId("spec-dialog-jira-pull-button"));

    // Error must render
    await waitFor(() => {
      expect(screen.getByTestId("spec-dialog-jira-pull-error")).toHaveTextContent(
        "missing environment variables: JIRA_URL, JIRA_USER_EMAIL, JIRA_API_TOKEN",
      );
    });

    // Title and body must NOT be populated (pull failed)
    const titleInput = screen.getByTestId("spec-dialog-title-input") as HTMLInputElement;
    const bodyTextarea = screen.getByTestId("spec-dialog-body-textarea") as HTMLTextAreaElement;
    expect(titleInput.value).toBe("");
    expect(bodyTextarea.value).toBe("");

    // User can still type a title manually and submit
    await user.type(titleInput, "Manually typed title");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    expect(props.onSubmit).toHaveBeenCalledTimes(1);
    expect(props.onSubmit).toHaveBeenCalledWith("Manually typed title", "");
  });

  // --------------------------------------------------------------------------
  // Test 3: invalid key keeps pull button disabled, invoke not called
  // --------------------------------------------------------------------------

  it("invalid key keeps pull button disabled and invoke is never called", async () => {
    const user = userEvent.setup();

    const props = makeProps();
    render(<SpecDialog {...props} />);

    const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
    const pullButton = screen.getByTestId("spec-dialog-jira-pull-button") as HTMLButtonElement;

    // Lowercase key is invalid
    await user.type(keyInput, "lowercase-key");
    expect(pullButton.disabled).toBe(true);

    // Attempt a click (should be a no-op since disabled)
    await user.click(pullButton);

    // invoke must never have been called
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});
