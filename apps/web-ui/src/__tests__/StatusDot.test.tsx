import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import StatusDot from "../components/StatusDot";

describe("StatusDot", () => {
  describe("queued variant", () => {
    it("renders label 'Queued'", () => {
      render(<StatusDot status="queued" />);
      expect(screen.getByText("Queued")).toBeInTheDocument();
    });

    it("pill has bg-zinc-100 and text-zinc-500 classes", () => {
      render(<StatusDot status="queued" />);
      const pill = screen.getByTestId("status-dot");
      expect(pill.className).toContain("bg-zinc-100");
      expect(pill.className).toContain("text-zinc-500");
    });

    it("dot has bg-zinc-400 and does NOT have animate-pulse", () => {
      render(<StatusDot status="queued" />);
      const dot = screen.getByTestId("status-dot-marker");
      expect(dot.className).toContain("bg-zinc-400");
      expect(dot.className).not.toContain("animate-pulse");
    });

    it("data-status attribute is 'queued'", () => {
      render(<StatusDot status="queued" />);
      expect(screen.getByTestId("status-dot")).toHaveAttribute("data-status", "queued");
    });
  });

  describe("active variant", () => {
    it("renders label 'Running'", () => {
      render(<StatusDot status="active" />);
      expect(screen.getByText("Running")).toBeInTheDocument();
    });

    it("pill has bg-blue-100 and text-blue-700 classes", () => {
      render(<StatusDot status="active" />);
      const pill = screen.getByTestId("status-dot");
      expect(pill.className).toContain("bg-blue-100");
      expect(pill.className).toContain("text-blue-700");
    });

    it("dot has bg-blue-600 and animate-pulse", () => {
      render(<StatusDot status="active" />);
      const dot = screen.getByTestId("status-dot-marker");
      expect(dot.className).toContain("bg-blue-600");
      expect(dot.className).toContain("animate-pulse");
    });

    it("data-status attribute is 'active'", () => {
      render(<StatusDot status="active" />);
      expect(screen.getByTestId("status-dot")).toHaveAttribute("data-status", "active");
    });
  });

  describe("done variant", () => {
    it("renders label 'Done'", () => {
      render(<StatusDot status="done" />);
      expect(screen.getByText("Done")).toBeInTheDocument();
    });

    it("pill has bg-green-100 and text-green-700 classes", () => {
      render(<StatusDot status="done" />);
      const pill = screen.getByTestId("status-dot");
      expect(pill.className).toContain("bg-green-100");
      expect(pill.className).toContain("text-green-700");
    });

    it("dot has bg-green-600 and does NOT have animate-pulse", () => {
      render(<StatusDot status="done" />);
      const dot = screen.getByTestId("status-dot-marker");
      expect(dot.className).toContain("bg-green-600");
      expect(dot.className).not.toContain("animate-pulse");
    });
  });

  describe("failed variant", () => {
    it("renders label 'Failed'", () => {
      render(<StatusDot status="failed" />);
      expect(screen.getByText("Failed")).toBeInTheDocument();
    });

    it("pill has bg-red-100 and text-red-700 classes", () => {
      render(<StatusDot status="failed" />);
      const pill = screen.getByTestId("status-dot");
      expect(pill.className).toContain("bg-red-100");
      expect(pill.className).toContain("text-red-700");
    });

    it("dot has bg-red-600", () => {
      render(<StatusDot status="failed" />);
      const dot = screen.getByTestId("status-dot-marker");
      expect(dot.className).toContain("bg-red-600");
    });
  });

  describe("errored variant", () => {
    it("renders label 'Errored'", () => {
      render(<StatusDot status="errored" />);
      expect(screen.getByText("Errored")).toBeInTheDocument();
    });

    it("pill has bg-red-100 and text-red-700 classes (same as failed)", () => {
      render(<StatusDot status="errored" />);
      const pill = screen.getByTestId("status-dot");
      expect(pill.className).toContain("bg-red-100");
      expect(pill.className).toContain("text-red-700");
    });

    it("dot has bg-red-600", () => {
      render(<StatusDot status="errored" />);
      const dot = screen.getByTestId("status-dot-marker");
      expect(dot.className).toContain("bg-red-600");
    });

    it("data-status attribute is 'errored'", () => {
      render(<StatusDot status="errored" />);
      expect(screen.getByTestId("status-dot")).toHaveAttribute("data-status", "errored");
    });
  });

  describe("skipped variant", () => {
    it("renders label 'Skipped'", () => {
      render(<StatusDot status="skipped" />);
      expect(screen.getByText("Skipped")).toBeInTheDocument();
    });

    it("pill has bg-zinc-100 and text-zinc-400 classes", () => {
      render(<StatusDot status="skipped" />);
      const pill = screen.getByTestId("status-dot");
      expect(pill.className).toContain("bg-zinc-100");
      expect(pill.className).toContain("text-zinc-400");
    });

    it("dot has opacity-50", () => {
      render(<StatusDot status="skipped" />);
      const dot = screen.getByTestId("status-dot-marker");
      expect(dot.className).toContain("opacity-50");
    });

    it("dot does NOT have animate-pulse", () => {
      render(<StatusDot status="skipped" />);
      const dot = screen.getByTestId("status-dot-marker");
      expect(dot.className).not.toContain("animate-pulse");
    });
  });

  describe("structural / a11y assertions", () => {
    it("outer element has data-testid='status-dot'", () => {
      render(<StatusDot status="queued" />);
      expect(screen.getByTestId("status-dot")).toBeInTheDocument();
    });

    it("dot marker has data-testid='status-dot-marker'", () => {
      render(<StatusDot status="queued" />);
      expect(screen.getByTestId("status-dot-marker")).toBeInTheDocument();
    });

    it("dot marker has aria-hidden='true'", () => {
      render(<StatusDot status="queued" />);
      expect(screen.getByTestId("status-dot-marker")).toHaveAttribute("aria-hidden", "true");
    });

    it("pill is inline-flex with items-center and rounded-full classes", () => {
      render(<StatusDot status="queued" />);
      const pill = screen.getByTestId("status-dot");
      expect(pill.className).toContain("inline-flex");
      expect(pill.className).toContain("items-center");
      expect(pill.className).toContain("rounded-full");
    });
  });
});
