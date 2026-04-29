import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import ChatComposer from "../components/ChatComposer";

describe("ChatComposer", () => {
  describe("rendering", () => {
    it("renders the composer root element", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      expect(screen.getByTestId("chat-composer")).toBeInTheDocument();
    });

    it("renders 4 quick-pick chips with correct labels", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const plan = screen.getByTestId("chat-composer-chip-plan");
      const brainstorm = screen.getByTestId("chat-composer-chip-brainstorm");
      const develop = screen.getByTestId("chat-composer-chip-develop");
      const spec = screen.getByTestId("chat-composer-chip-spec");

      expect(plan).toHaveTextContent("Plan");
      expect(brainstorm).toHaveTextContent("Brainstorm");
      expect(develop).toHaveTextContent("Develop");
      expect(spec).toHaveTextContent("Spec");
    });

    it("renders textarea", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      expect(screen.getByTestId("chat-composer-textarea")).toBeInTheDocument();
    });

    it("renders send button", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      expect(screen.getByTestId("chat-composer-send")).toBeInTheDocument();
    });
  });

  describe("chip interaction", () => {
    it("clicking Plan chip sets textarea to /plan  and focuses it", async () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const chip = screen.getByTestId("chat-composer-chip-plan");
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.click(chip);

      expect(textarea).toHaveValue("/plan ");
      expect(document.activeElement).toBe(textarea);
    });

    it("clicking Brainstorm chip sets textarea to /brainstorm  and focuses it", async () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const chip = screen.getByTestId("chat-composer-chip-brainstorm");
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.click(chip);

      expect(textarea).toHaveValue("/brainstorm ");
      expect(document.activeElement).toBe(textarea);
    });

    it("clicking Develop chip sets textarea to /develop  and focuses it", async () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const chip = screen.getByTestId("chat-composer-chip-develop");
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.click(chip);

      expect(textarea).toHaveValue("/develop ");
      expect(document.activeElement).toBe(textarea);
    });

    it("clicking Spec chip sets textarea to /spec  and focuses it", async () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const chip = screen.getByTestId("chat-composer-chip-spec");
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.click(chip);

      expect(textarea).toHaveValue("/spec ");
      expect(document.activeElement).toBe(textarea);
    });
  });

  describe("send via button", () => {
    it("typing after chip click then clicking send fires onSend and clears textarea", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const chip = screen.getByTestId("chat-composer-chip-plan");
      const textarea = screen.getByTestId("chat-composer-textarea");
      const sendBtn = screen.getByTestId("chat-composer-send");

      await userEvent.click(chip);
      await userEvent.type(textarea, "hello");
      await userEvent.click(sendBtn);

      expect(onSend).toHaveBeenCalledWith("/plan hello");
      expect(textarea).toHaveValue("");
    });
  });

  describe("keyboard shortcuts", () => {
    it("Cmd+Enter sends message", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.type(textarea, "hi");
      fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

      expect(onSend).toHaveBeenCalledWith("hi");
    });

    it("Ctrl+Enter sends message", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.type(textarea, "hi");
      fireEvent.keyDown(textarea, { key: "Enter", ctrlKey: true });

      expect(onSend).toHaveBeenCalledWith("hi");
    });

    it("Enter alone does NOT send", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.type(textarea, "hi");
      fireEvent.keyDown(textarea, { key: "Enter" });

      expect(onSend).not.toHaveBeenCalled();
    });

    it("Enter alone inserts a newline in the textarea", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.type(textarea, "hi");
      // Simulate browser newline insertion: keydown does not send,
      // then change event reflects the newline the browser would insert.
      fireEvent.keyDown(textarea, { key: "Enter" });
      fireEvent.change(textarea, { target: { value: "hi\nworld" } });

      expect(textarea).toHaveValue("hi\nworld");
      expect(onSend).not.toHaveBeenCalled();
    });

    it("Cmd+Enter with empty textarea does not fire onSend", () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

      expect(onSend).not.toHaveBeenCalled();
    });
  });

  describe("send after clear", () => {
    it("textarea is empty after send via Cmd+Enter", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      await userEvent.type(textarea, "hello");
      fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

      expect(textarea).toHaveValue("");
    });
  });
});
