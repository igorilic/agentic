import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Connector from "../components/Connector";

describe("Connector", () => {
  describe("data attributes", () => {
    it("renders with data-testid='connector' and data-active='false' when active={false}", () => {
      render(<Connector active={false} />);
      const connector = screen.getByTestId("connector");
      expect(connector).toBeInTheDocument();
      expect(connector).toHaveAttribute("data-active", "false");
    });

    it("renders with data-testid='connector' and data-active='true' when active={true}", () => {
      render(<Connector active={true} />);
      const connector = screen.getByTestId("connector");
      expect(connector).toBeInTheDocument();
      expect(connector).toHaveAttribute("data-active", "true");
    });
  });

  describe("active={true} — animated dashed line", () => {
    it("a child element has a class containing 'dashed' when active", () => {
      render(<Connector active={true} />);
      const connector = screen.getByTestId("connector");
      const hasDashed = [...connector.querySelectorAll("*")].some((el) =>
        (el.getAttribute("class") ?? "").includes("dashed")
      );
      expect(hasDashed).toBe(true);
    });

    it("a child element has a class containing 'animate-' when active", () => {
      render(<Connector active={true} />);
      const connector = screen.getByTestId("connector");
      const hasAnimate = [...connector.querySelectorAll("*")].some((el) =>
        (el.getAttribute("class") ?? "").includes("animate-")
      );
      expect(hasAnimate).toBe(true);
    });
  });

  describe("active={false} — static solid line", () => {
    it("no child has a 'dashed' class when not active", () => {
      render(<Connector active={false} />);
      const connector = screen.getByTestId("connector");
      const hasDashed = [...connector.querySelectorAll("*")].some((el) =>
        (el.getAttribute("class") ?? "").includes("dashed")
      );
      expect(hasDashed).toBe(false);
    });

    it("no child has an 'animate-' class when not active", () => {
      render(<Connector active={false} />);
      const connector = screen.getByTestId("connector");
      const hasAnimate = [...connector.querySelectorAll("*")].some((el) =>
        (el.getAttribute("class") ?? "").includes("animate-")
      );
      expect(hasAnimate).toBe(false);
    });
  });

  describe("chevron", () => {
    it("renders a child element with data-testid='connector-chevron'", () => {
      render(<Connector active={false} />);
      const chevron = screen.getByTestId("connector-chevron");
      expect(chevron).toBeInTheDocument();
    });

    it("chevron is present when active={true} as well", () => {
      render(<Connector active={true} />);
      const chevron = screen.getByTestId("connector-chevron");
      expect(chevron).toBeInTheDocument();
    });
  });
});
