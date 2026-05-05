import SpecDialog from "../components/SpecDialog";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, beforeEach } from "vitest";
import React from "react";

// Default mock for useJiraFetch — overridden per-test where needed.
const mockFetch = vi.fn();
// isLoading reflects the hook's internal state: toggled by the in-flight test.
let mockIsLoading = false;
vi.mock("../hooks/useJiraFetch", () => ({
  useJiraFetch: () => ({
    fetch: mockFetch,
    get isLoading() {
      return mockIsLoading;
    },
    error: null,
  }),
}));

beforeEach(() => {
  mockFetch.mockReset();
  mockIsLoading = false;
});

function makeProps(overrides: Partial<React.ComponentProps<typeof SpecDialog>> = {}) {
  return {
    open: true,
    onClose: vi.fn(),
    onSubmit: vi.fn(),
    ...overrides,
  };
}

describe("SpecDialog", () => {
  describe("visibility", () => {
    it("renders nothing when open={false}", () => {
      render(<SpecDialog {...makeProps({ open: false })} />);
      expect(screen.queryByTestId("spec-dialog")).toBeNull();
      expect(screen.queryByTestId("spec-dialog-backdrop")).toBeNull();
    });

    it("renders panel + backdrop testids when open={true}", () => {
      render(<SpecDialog {...makeProps()} />);
      expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();
      expect(screen.getByTestId("spec-dialog-backdrop")).toBeInTheDocument();
      expect(screen.getByTestId("spec-dialog-title-input")).toBeInTheDocument();
      expect(screen.getByTestId("spec-dialog-body-textarea")).toBeInTheDocument();
      expect(screen.getByTestId("spec-dialog-cancel")).toBeInTheDocument();
      expect(screen.getByTestId("spec-dialog-submit")).toBeInTheDocument();
    });
  });

  describe("header text", () => {
    it("shows 'New spec' heading", () => {
      render(<SpecDialog {...makeProps()} />);
      expect(screen.getByText("New spec")).toBeInTheDocument();
    });

    it("shows helper text 'Spec will be handed to the Architect.'", () => {
      render(<SpecDialog {...makeProps()} />);
      expect(screen.getByText("Spec will be handed to the Architect.")).toBeInTheDocument();
    });
  });

  describe("submit button disabled state", () => {
    it("submit is disabled when title is empty", () => {
      render(<SpecDialog {...makeProps()} />);
      const submit = screen.getByTestId("spec-dialog-submit") as HTMLButtonElement;
      expect(submit.disabled).toBe(true);
    });

    it("submit is disabled when title is whitespace only", async () => {
      render(<SpecDialog {...makeProps()} />);
      const titleInput = screen.getByTestId("spec-dialog-title-input");
      await userEvent.type(titleInput, "   ");
      const submit = screen.getByTestId("spec-dialog-submit") as HTMLButtonElement;
      expect(submit.disabled).toBe(true);
    });

    it("submit is enabled when title has content", async () => {
      render(<SpecDialog {...makeProps()} />);
      const titleInput = screen.getByTestId("spec-dialog-title-input");
      await userEvent.type(titleInput, "Add rate limiting");
      const submit = screen.getByTestId("spec-dialog-submit") as HTMLButtonElement;
      expect(submit.disabled).toBe(false);
    });
  });

  describe("submit behavior", () => {
    it("fires onSubmit with title + body when both are filled", async () => {
      const props = makeProps();
      render(<SpecDialog {...props} />);
      await userEvent.type(screen.getByTestId("spec-dialog-title-input"), "Add rate limiting");
      await userEvent.type(screen.getByTestId("spec-dialog-body-textarea"), "needs token bucket");
      await userEvent.click(screen.getByTestId("spec-dialog-submit"));
      expect(props.onSubmit).toHaveBeenCalledWith("Add rate limiting", "needs token bucket");
    });

    it("fires onSubmit(title, '') when only title is filled", async () => {
      const props = makeProps();
      render(<SpecDialog {...props} />);
      await userEvent.type(screen.getByTestId("spec-dialog-title-input"), "Add rate limiting");
      await userEvent.click(screen.getByTestId("spec-dialog-submit"));
      expect(props.onSubmit).toHaveBeenCalledWith("Add rate limiting", "");
    });

    it("does NOT call onClose after submit (parent decides)", async () => {
      const props = makeProps();
      render(<SpecDialog {...props} />);
      await userEvent.type(screen.getByTestId("spec-dialog-title-input"), "Add rate limiting");
      await userEvent.click(screen.getByTestId("spec-dialog-submit"));
      expect(props.onClose).not.toHaveBeenCalled();
    });
  });

  describe("dismissal", () => {
    it("cancel click fires onClose", async () => {
      const props = makeProps();
      render(<SpecDialog {...props} />);
      await userEvent.click(screen.getByTestId("spec-dialog-cancel"));
      expect(props.onClose).toHaveBeenCalledTimes(1);
    });

    it("backdrop click fires onClose", () => {
      const props = makeProps();
      render(<SpecDialog {...props} />);
      fireEvent.click(screen.getByTestId("spec-dialog-backdrop"));
      expect(props.onClose).toHaveBeenCalled();
    });

    it("panel click does NOT fire onClose", () => {
      const props = makeProps();
      render(<SpecDialog {...props} />);
      fireEvent.click(screen.getByTestId("spec-dialog"));
      expect(props.onClose).not.toHaveBeenCalled();
    });

    it("Esc key fires onClose", async () => {
      const props = makeProps();
      render(<SpecDialog {...props} />);
      await userEvent.click(screen.getByTestId("spec-dialog-title-input"));
      await userEvent.keyboard("{Escape}");
      expect(props.onClose).toHaveBeenCalled();
    });
  });

  describe("accessibility", () => {
    it("title input has autoFocus when open", () => {
      render(<SpecDialog {...makeProps()} />);
      const titleInput = screen.getByTestId("spec-dialog-title-input");
      expect(document.activeElement).toBe(titleInput);
    });

    it("panel has role='dialog' and aria-modal='true'", () => {
      render(<SpecDialog {...makeProps()} />);
      const panel = screen.getByTestId("spec-dialog");
      expect(panel).toHaveAttribute("role", "dialog");
      expect(panel).toHaveAttribute("aria-modal", "true");
    });

    it("panel has aria-label 'New spec'", () => {
      render(<SpecDialog {...makeProps()} />);
      const panel = screen.getByTestId("spec-dialog");
      expect(panel).toHaveAttribute("aria-label", "New spec");
    });
  });

  describe("structure", () => {
    it("textarea has rows={8}", () => {
      render(<SpecDialog {...makeProps()} />);
      const textarea = screen.getByTestId("spec-dialog-body-textarea") as HTMLTextAreaElement;
      expect(textarea.rows).toBe(8);
    });

    it("panel className contains w-[560px]", () => {
      render(<SpecDialog {...makeProps()} />);
      const panel = screen.getByTestId("spec-dialog");
      expect(panel.className).toContain("w-[560px]");
    });
  });

  describe("jira-pull row", () => {
    it("renders the jira-pull row above the title input", () => {
      render(<SpecDialog {...makeProps()} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      const pullButton = screen.getByTestId("spec-dialog-jira-pull-button");
      const titleInput = screen.getByTestId("spec-dialog-title-input");
      expect(keyInput).toBeInTheDocument();
      expect(pullButton).toBeInTheDocument();
      // Assert DOM order: key input comes before pull button, which comes before title input
      expect(
        keyInput.compareDocumentPosition(pullButton) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
      expect(
        pullButton.compareDocumentPosition(titleInput) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    });

    it("disables the pull button when key is empty", () => {
      render(<SpecDialog {...makeProps()} />);
      const pullButton = screen.getByTestId("spec-dialog-jira-pull-button") as HTMLButtonElement;
      expect(pullButton.disabled).toBe(true);
    });

    it("disables the pull button for invalid keys", async () => {
      render(<SpecDialog {...makeProps()} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      const pullButton = screen.getByTestId("spec-dialog-jira-pull-button") as HTMLButtonElement;

      await userEvent.type(keyInput, "proj-1");
      expect(pullButton.disabled).toBe(true);

      await userEvent.clear(keyInput);
      await userEvent.type(keyInput, "-1");
      expect(pullButton.disabled).toBe(true);

      await userEvent.clear(keyInput);
      await userEvent.type(keyInput, "PROJ");
      expect(pullButton.disabled).toBe(true);
    });

    it("enables the pull button for valid keys", async () => {
      render(<SpecDialog {...makeProps()} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      const pullButton = screen.getByTestId("spec-dialog-jira-pull-button") as HTMLButtonElement;

      await userEvent.type(keyInput, "PROJ-1");
      expect(pullButton.disabled).toBe(false);
    });

    it("populates title and body on successful pull", async () => {
      const dto = {
        key: "PROJ-1",
        title: "Fix bug",
        body: "Steps:\n1. …",
        ac: "Given X, when Y, then Z",
      };
      mockFetch.mockResolvedValueOnce(dto);

      render(<SpecDialog {...makeProps()} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      await userEvent.type(keyInput, "PROJ-1");
      await userEvent.click(screen.getByTestId("spec-dialog-jira-pull-button"));

      await waitFor(() => {
        const titleInput = screen.getByTestId("spec-dialog-title-input") as HTMLInputElement;
        expect(titleInput.value).toBe("Fix bug");
      });

      const bodyTextarea = screen.getByTestId("spec-dialog-body-textarea") as HTMLTextAreaElement;
      expect(bodyTextarea.value).toBe(
        "Steps:\n1. …\n\n## Acceptance Criteria\nGiven X, when Y, then Z",
      );
    });

    it("appends only body when ac is null", async () => {
      const dto = {
        key: "PROJ-1",
        title: "Fix bug",
        body: "Steps:\n1. …",
        ac: null,
      };
      mockFetch.mockResolvedValueOnce(dto);

      render(<SpecDialog {...makeProps()} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      await userEvent.type(keyInput, "PROJ-1");
      await userEvent.click(screen.getByTestId("spec-dialog-jira-pull-button"));

      await waitFor(() => {
        const titleInput = screen.getByTestId("spec-dialog-title-input") as HTMLInputElement;
        expect(titleInput.value).toBe("Fix bug");
      });

      const bodyTextarea = screen.getByTestId("spec-dialog-body-textarea") as HTMLTextAreaElement;
      expect(bodyTextarea.value).toBe("Steps:\n1. …");
    });

    it("renders the missing-env error inline", async () => {
      const errMsg = "missing environment variables: JIRA_URL, JIRA_EMAIL";
      mockFetch.mockRejectedValueOnce(errMsg);

      render(<SpecDialog {...makeProps()} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      await userEvent.type(keyInput, "PROJ-1");
      await userEvent.click(screen.getByTestId("spec-dialog-jira-pull-button"));

      await waitFor(() => {
        expect(
          screen.getByTestId("spec-dialog-jira-pull-error"),
        ).toHaveTextContent(/missing environment variables: JIRA_URL, JIRA_EMAIL/);
      });
    });

    it("does not clear existing title/body fields on error", async () => {
      const errMsg = "missing environment variables: JIRA_URL, JIRA_EMAIL";
      mockFetch.mockRejectedValueOnce(errMsg);

      render(<SpecDialog {...makeProps()} />);
      await userEvent.type(screen.getByTestId("spec-dialog-title-input"), "Existing title");
      await userEvent.type(screen.getByTestId("spec-dialog-body-textarea"), "Existing body");

      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      await userEvent.type(keyInput, "PROJ-1");
      await userEvent.click(screen.getByTestId("spec-dialog-jira-pull-button"));

      await waitFor(() => {
        expect(screen.getByTestId("spec-dialog-jira-pull-error")).toBeInTheDocument();
      });

      const titleInput = screen.getByTestId("spec-dialog-title-input") as HTMLInputElement;
      const bodyTextarea = screen.getByTestId("spec-dialog-body-textarea") as HTMLTextAreaElement;
      expect(titleInput.value).toBe("Existing title");
      expect(bodyTextarea.value).toBe("Existing body");
    });

    it("disables the button while fetch is in flight", async () => {
      let resolvePromise!: (value: unknown) => void;
      const deferred = new Promise((resolve) => {
        resolvePromise = resolve;
      });
      mockFetch.mockReturnValueOnce(deferred);

      const props = makeProps();
      const { rerender } = render(<SpecDialog {...props} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      await userEvent.type(keyInput, "PROJ-1");

      const pullButton = screen.getByTestId("spec-dialog-jira-pull-button") as HTMLButtonElement;
      expect(pullButton.disabled).toBe(false);

      // Set isLoading=true before clicking so next render picks it up
      mockIsLoading = true;
      await userEvent.click(pullButton);
      // Force re-render so component reads updated mockIsLoading from hook mock
      rerender(<SpecDialog {...props} />);

      // While in-flight, button should be disabled (isLoading=true from hook)
      expect(pullButton.disabled).toBe(true);

      // Resolve the promise and reset isLoading
      mockIsLoading = false;
      resolvePromise({ key: "PROJ-1", title: "Done", body: "body", ac: null });
      rerender(<SpecDialog {...props} />);

      // After resolution, button should be re-enabled
      expect(pullButton.disabled).toBe(false);
    });

    it("re-pull clears prior error before fetch settles", async () => {
      // First pull: fails with "network down"
      mockFetch.mockRejectedValueOnce("network down");

      render(<SpecDialog {...makeProps()} />);
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input");
      await userEvent.type(keyInput, "PROJ-1");
      await userEvent.click(screen.getByTestId("spec-dialog-jira-pull-button"));

      await waitFor(() => {
        expect(screen.getByTestId("spec-dialog-jira-pull-error")).toHaveTextContent("network down");
      });

      // Second pull: resolves successfully
      const dto = { key: "PROJ-1", title: "Fixed title", body: "Fixed body", ac: null };
      mockFetch.mockResolvedValueOnce(dto);
      await userEvent.click(screen.getByTestId("spec-dialog-jira-pull-button"));

      await waitFor(() => {
        // Error must be gone
        expect(screen.queryByTestId("spec-dialog-jira-pull-error")).toBeNull();
      });

      // Title and body must be populated from successful pull
      const titleInput = screen.getByTestId("spec-dialog-title-input") as HTMLInputElement;
      const bodyTextarea = screen.getByTestId("spec-dialog-body-textarea") as HTMLTextAreaElement;
      expect(titleInput.value).toBe("Fixed title");
      expect(bodyTextarea.value).toBe("Fixed body");
    });

    it("close + reopen clears jira key and error", async () => {
      const onClose = vi.fn();
      const { rerender } = render(<SpecDialog {...makeProps({ onClose })} />);

      // Type a key and trigger an error
      mockFetch.mockRejectedValueOnce("server error");
      await userEvent.type(screen.getByTestId("spec-dialog-jira-key-input"), "PROJ-99");
      await userEvent.click(screen.getByTestId("spec-dialog-jira-pull-button"));

      await waitFor(() => {
        expect(screen.getByTestId("spec-dialog-jira-pull-error")).toHaveTextContent("server error");
      });

      // Close the dialog (simulate Cancel click which calls onClose, then parent sets open=false)
      await userEvent.click(screen.getByTestId("spec-dialog-cancel"));
      expect(onClose).toHaveBeenCalledTimes(1);

      // Parent re-renders with open=false
      rerender(<SpecDialog {...makeProps({ open: false, onClose })} />);

      // Reopen
      rerender(<SpecDialog {...makeProps({ open: true, onClose })} />);

      // Key input should be empty
      const keyInput = screen.getByTestId("spec-dialog-jira-key-input") as HTMLInputElement;
      expect(keyInput.value).toBe("");

      // No error visible
      expect(screen.queryByTestId("spec-dialog-jira-pull-error")).toBeNull();
    });
  });
});
