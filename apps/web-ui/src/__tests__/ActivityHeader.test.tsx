import ActivityHeader from "../components/ActivityHeader";
import { render, screen, fireEvent } from "@testing-library/react";
import { vi } from "vitest";

const defaultProps = {
  counts: { all: 12, tool: 3, perm: 1, error: 0 },
  filter: "all" as const,
  onFilterChange: vi.fn(),
};

describe("ActivityHeader", () => {
  describe("rendering", () => {
    it('renders title "Activity"', () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByText("Activity")).toBeInTheDocument();
    });

    it("outer container has data-testid activity-header", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByTestId("activity-header")).toBeInTheDocument();
    });

    it("has a tablist", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByRole("tablist")).toBeInTheDocument();
    });

    it("renders 4 tabs with role=tab", () => {
      render(<ActivityHeader {...defaultProps} />);
      const tabs = screen.getAllByRole("tab");
      expect(tabs).toHaveLength(4);
    });

    it("renders 4 tabs with stable testids", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByTestId("activity-tab-all")).toBeInTheDocument();
      expect(screen.getByTestId("activity-tab-tool")).toBeInTheDocument();
      expect(screen.getByTestId("activity-tab-perm")).toBeInTheDocument();
      expect(screen.getByTestId("activity-tab-error")).toBeInTheDocument();
    });

    it("renders visible labels All, Tool calls, Permissions, Errors", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByText("All")).toBeInTheDocument();
      expect(screen.getByText("Tool calls")).toBeInTheDocument();
      expect(screen.getByText("Permissions")).toBeInTheDocument();
      expect(screen.getByText("Errors")).toBeInTheDocument();
    });

    it("renders count chips with stable testids showing matching counts", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByTestId("activity-tab-all-count")).toHaveTextContent("12");
      expect(screen.getByTestId("activity-tab-tool-count")).toHaveTextContent("3");
      expect(screen.getByTestId("activity-tab-perm-count")).toHaveTextContent("1");
      expect(screen.getByTestId("activity-tab-error-count")).toHaveTextContent("0");
    });
  });

  describe("aria-selected", () => {
    it("active tab (filter=all) has aria-selected=true", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByTestId("activity-tab-all")).toHaveAttribute("aria-selected", "true");
    });

    it("inactive tabs have aria-selected=false when filter=all", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByTestId("activity-tab-tool")).toHaveAttribute("aria-selected", "false");
      expect(screen.getByTestId("activity-tab-perm")).toHaveAttribute("aria-selected", "false");
      expect(screen.getByTestId("activity-tab-error")).toHaveAttribute("aria-selected", "false");
    });

    it("when filter=tool, tool tab has aria-selected=true and others false", () => {
      render(<ActivityHeader {...defaultProps} filter="tool" />);
      expect(screen.getByTestId("activity-tab-tool")).toHaveAttribute("aria-selected", "true");
      expect(screen.getByTestId("activity-tab-all")).toHaveAttribute("aria-selected", "false");
      expect(screen.getByTestId("activity-tab-perm")).toHaveAttribute("aria-selected", "false");
      expect(screen.getByTestId("activity-tab-error")).toHaveAttribute("aria-selected", "false");
    });
  });

  describe("click interactions", () => {
    it("click activity-tab-tool calls onFilterChange('tool') once", () => {
      const onFilterChange = vi.fn();
      render(<ActivityHeader {...defaultProps} onFilterChange={onFilterChange} />);
      fireEvent.click(screen.getByTestId("activity-tab-tool"));
      expect(onFilterChange).toHaveBeenCalledTimes(1);
      expect(onFilterChange).toHaveBeenCalledWith("tool");
    });

    it("click activity-tab-perm calls onFilterChange('perm') once", () => {
      const onFilterChange = vi.fn();
      render(<ActivityHeader {...defaultProps} onFilterChange={onFilterChange} />);
      fireEvent.click(screen.getByTestId("activity-tab-perm"));
      expect(onFilterChange).toHaveBeenCalledTimes(1);
      expect(onFilterChange).toHaveBeenCalledWith("perm");
    });

    it("click activity-tab-error calls onFilterChange('error') once", () => {
      const onFilterChange = vi.fn();
      render(<ActivityHeader {...defaultProps} onFilterChange={onFilterChange} />);
      fireEvent.click(screen.getByTestId("activity-tab-error"));
      expect(onFilterChange).toHaveBeenCalledTimes(1);
      expect(onFilterChange).toHaveBeenCalledWith("error");
    });
  });

  describe("active tab styling", () => {
    it("active tab className contains border-b-2", () => {
      render(<ActivityHeader {...defaultProps} />);
      const activeTab = screen.getByTestId("activity-tab-all");
      expect(activeTab.className).toContain("border-b-2");
    });

    it("inactive tabs className contains text-fg-muted", () => {
      render(<ActivityHeader {...defaultProps} />);
      expect(screen.getByTestId("activity-tab-tool").className).toContain("text-fg-muted");
      expect(screen.getByTestId("activity-tab-perm").className).toContain("text-fg-muted");
      expect(screen.getByTestId("activity-tab-error").className).toContain("text-fg-muted");
    });
  });
});
