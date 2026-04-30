import SpecDialog from "../components/SpecDialog";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import React from "react";

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
});
