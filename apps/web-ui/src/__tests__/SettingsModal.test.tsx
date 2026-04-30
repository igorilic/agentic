import { render, screen, fireEvent } from "@testing-library/react";
import { vi } from "vitest";
import React from "react";
import SettingsModal from "../components/SettingsModal";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

function makeProps(
  overrides: Partial<React.ComponentProps<typeof SettingsModal>> = {}
) {
  return {
    open: true,
    onClose: vi.fn(),
    ...overrides,
  };
}

describe("SettingsModal", () => {
  describe("visibility", () => {
    it("renders nothing when open={false}", () => {
      render(<SettingsModal {...makeProps({ open: false })} />);
      expect(screen.queryByTestId("settings-modal")).toBeNull();
    });

    it("renders modal in document when open={true}", () => {
      render(<SettingsModal {...makeProps()} />);
      const modal = screen.getByTestId("settings-modal");
      expect(modal).toBeInTheDocument();
      expect(modal).toHaveAttribute("role", "dialog");
      expect(modal).toHaveAttribute("aria-modal", "true");
      expect(modal).toHaveAttribute("aria-label", "Settings");
    });
  });

  describe("tab strip", () => {
    it("renders both tabs with general active by default", () => {
      render(<SettingsModal {...makeProps({ initialTab: "general" })} />);
      const generalTab = screen.getByTestId("settings-tab-general");
      const historyTab = screen.getByTestId("settings-tab-history");
      expect(generalTab).toBeInTheDocument();
      expect(historyTab).toBeInTheDocument();
      expect(generalTab).toHaveAttribute("aria-selected", "true");
      expect(historyTab).toHaveAttribute("aria-selected", "false");
    });

    it("tab strip has role='tablist'", () => {
      render(<SettingsModal {...makeProps()} />);
      expect(screen.getByRole("tablist")).toBeInTheDocument();
    });
  });

  describe("tab bodies", () => {
    it("shows settings-pane and hides past-runs-pane on general tab", () => {
      render(<SettingsModal {...makeProps({ initialTab: "general" })} />);
      expect(screen.getByTestId("settings-pane")).toBeInTheDocument();
      expect(screen.queryByTestId("past-runs-pane")).toBeNull();
    });

    it("switches to history tab on click", () => {
      render(<SettingsModal {...makeProps({ initialTab: "general" })} />);
      fireEvent.click(screen.getByTestId("settings-tab-history"));
      expect(screen.getByTestId("settings-tab-history")).toHaveAttribute(
        "aria-selected",
        "true"
      );
      expect(screen.getByTestId("settings-tab-general")).toHaveAttribute(
        "aria-selected",
        "false"
      );
      expect(screen.getByTestId("past-runs-pane")).toBeInTheDocument();
      expect(screen.queryByTestId("settings-pane")).toBeNull();
    });

    it("shows past-runs-pane and hides settings-pane when initialTab='history'", () => {
      render(<SettingsModal {...makeProps({ initialTab: "history" })} />);
      expect(screen.getByTestId("past-runs-pane")).toBeInTheDocument();
      expect(screen.queryByTestId("settings-pane")).toBeNull();
      expect(screen.getByTestId("settings-tab-history")).toHaveAttribute(
        "aria-selected",
        "true"
      );
    });
  });

  describe("dismissal", () => {
    it("close button click fires onClose", () => {
      const props = makeProps();
      render(<SettingsModal {...props} />);
      fireEvent.click(screen.getByTestId("settings-modal-close"));
      expect(props.onClose).toHaveBeenCalledTimes(1);
    });

    it("backdrop click fires onClose", () => {
      const props = makeProps();
      render(<SettingsModal {...props} />);
      fireEvent.click(screen.getByTestId("settings-modal-backdrop"));
      expect(props.onClose).toHaveBeenCalledTimes(1);
    });

    it("Esc key fires onClose", () => {
      const props = makeProps();
      render(<SettingsModal {...props} />);
      fireEvent.keyDown(screen.getByTestId("settings-modal"), {
        key: "Escape",
        code: "Escape",
      });
      expect(props.onClose).toHaveBeenCalledTimes(1);
    });
  });

  describe("onSelectRun pass-through", () => {
    it("renders history tab with onSelectRun prop without throwing", () => {
      const onSelectRun = vi.fn();
      expect(() => {
        render(
          <SettingsModal
            {...makeProps({ initialTab: "history", onSelectRun })}
          />
        );
      }).not.toThrow();
      expect(screen.getByTestId("past-runs-pane")).toBeInTheDocument();
    });
  });
});
