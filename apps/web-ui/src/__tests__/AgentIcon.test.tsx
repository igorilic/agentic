import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import AgentIcon from "../components/AgentIcon";

describe("AgentIcon", () => {
  describe("basic rendering — architect (blueprint, simple path)", () => {
    it("renders an svg with data-testid agent-icon-architect", () => {
      render(<AgentIcon agent="architect" />);
      expect(screen.getByTestId("agent-icon-architect")).toBeInTheDocument();
    });

    it("svg has viewBox='0 0 20 20'", () => {
      render(<AgentIcon agent="architect" />);
      const svg = screen.getByTestId("agent-icon-architect");
      expect(svg.tagName.toLowerCase()).toBe("svg");
      expect(svg).toHaveAttribute("viewBox", "0 0 20 20");
    });

    it("svg has default width and height of 18", () => {
      render(<AgentIcon agent="architect" />);
      const svg = screen.getByTestId("agent-icon-architect");
      expect(svg).toHaveAttribute("width", "18");
      expect(svg).toHaveAttribute("height", "18");
    });

    it("svg has aria-hidden=true", () => {
      render(<AgentIcon agent="architect" />);
      const svg = screen.getByTestId("agent-icon-architect");
      expect(svg).toHaveAttribute("aria-hidden", "true");
    });

    it("renders a <path> with the blueprint glyph d attribute", () => {
      render(<AgentIcon agent="architect" />);
      const svg = screen.getByTestId("agent-icon-architect");
      const path = svg.querySelector("path");
      expect(path).not.toBeNull();
      expect(path!.getAttribute("d")).toBe("M3 4h14v12H3zM3 8h14M7 4v12M11 12h2");
    });
  });

  describe("size prop", () => {
    it("default size=18 produces width=18 and height=18", () => {
      render(<AgentIcon agent="architect" />);
      const svg = screen.getByTestId("agent-icon-architect");
      expect(svg).toHaveAttribute("width", "18");
      expect(svg).toHaveAttribute("height", "18");
    });

    it("size=14 produces width=14 and height=14", () => {
      render(<AgentIcon agent="architect" size={14} />);
      const svg = screen.getByTestId("agent-icon-architect");
      expect(svg).toHaveAttribute("width", "14");
      expect(svg).toHaveAttribute("height", "14");
    });

    it("size=24 produces width=24 and height=24", () => {
      render(<AgentIcon agent="architect" size={24} />);
      const svg = screen.getByTestId("agent-icon-architect");
      expect(svg).toHaveAttribute("width", "24");
      expect(svg).toHaveAttribute("height", "24");
    });
  });

  describe("qa agent (icon=check — simple path)", () => {
    it("renders svg with data-testid agent-icon-qa", () => {
      render(<AgentIcon agent="qa" />);
      expect(screen.getByTestId("agent-icon-qa")).toBeInTheDocument();
    });

    it("renders a <path> for the check glyph", () => {
      render(<AgentIcon agent="qa" />);
      const svg = screen.getByTestId("agent-icon-qa");
      const path = svg.querySelector("path");
      expect(path).not.toBeNull();
      expect(path!.getAttribute("d")).toBe("M3 10l2 5 4-9 5 7 3-6");
    });
  });

  describe("reviewer agent (icon=eye — group with path + circle)", () => {
    it("renders svg with data-testid agent-icon-reviewer", () => {
      render(<AgentIcon agent="reviewer" />);
      expect(screen.getByTestId("agent-icon-reviewer")).toBeInTheDocument();
    });

    it("svg contains a <g> element for the eye glyph", () => {
      render(<AgentIcon agent="reviewer" />);
      const svg = screen.getByTestId("agent-icon-reviewer");
      const g = svg.querySelector("g");
      expect(g).not.toBeNull();
    });

    it("svg contains a <circle> element for the eye pupil", () => {
      render(<AgentIcon agent="reviewer" />);
      const svg = screen.getByTestId("agent-icon-reviewer");
      const circle = svg.querySelector("circle");
      expect(circle).not.toBeNull();
    });
  });

  describe("developer agent (icon=code)", () => {
    it("renders svg with data-testid agent-icon-developer", () => {
      render(<AgentIcon agent="developer" />);
      expect(screen.getByTestId("agent-icon-developer")).toBeInTheDocument();
    });

    it("svg contains a <path> with the code glyph d attribute", () => {
      render(<AgentIcon agent="developer" />);
      const svg = screen.getByTestId("agent-icon-developer");
      const path = svg.querySelector("path");
      expect(path).not.toBeNull();
      expect(path!.getAttribute("d")).toBe(
        "M7 6l-4 4 4 4M13 6l4 4-4 4M11 4l-2 12"
      );
    });
  });

  describe("tdd-developer alias — resolves to code glyph same as developer", () => {
    it("renders svg with data-testid agent-icon-tdd-developer", () => {
      render(<AgentIcon agent="tdd-developer" />);
      expect(screen.getByTestId("agent-icon-tdd-developer")).toBeInTheDocument();
    });

    it("svg contains the same code glyph path as developer", () => {
      render(<AgentIcon agent="tdd-developer" />);
      const svg = screen.getByTestId("agent-icon-tdd-developer");
      const path = svg.querySelector("path");
      expect(path).not.toBeNull();
      expect(path!.getAttribute("d")).toBe(
        "M7 6l-4 4 4 4M13 6l4 4-4 4M11 4l-2 12"
      );
    });
  });

  describe("all 12 known agent ids + tdd-developer alias render without crash", () => {
    const agentIds = [
      "architect", "developer", "qa", "reviewer", "researcher",
      "security", "perf", "docs", "designer", "db", "devops", "a11y",
      "tdd-developer",
    ];

    for (const id of agentIds) {
      it(`renders without crash for agent="${id}"`, () => {
        expect(() => render(<AgentIcon agent={id} />)).not.toThrow();
        expect(screen.getByTestId(`agent-icon-${id}`)).toBeInTheDocument();
      });
    }
  });

  describe("unknown agent — initial fallback (div with letter)", () => {
    it("renders without crash for unknown agent", () => {
      expect(() => render(<AgentIcon agent="unknown-agent" />)).not.toThrow();
    });

    it("renders element with data-testid agent-icon-unknown-agent", () => {
      render(<AgentIcon agent="unknown-agent" />);
      expect(screen.getByTestId("agent-icon-unknown-agent")).toBeInTheDocument();
    });

    it("fallback element shows letter 'U' (first letter of unknown-agent)", () => {
      render(<AgentIcon agent="unknown-agent" />);
      const el = screen.getByTestId("agent-icon-unknown-agent");
      expect(el.textContent).toBe("U");
    });

    it("fallback element is a div not an svg", () => {
      render(<AgentIcon agent="unknown-agent" />);
      const el = screen.getByTestId("agent-icon-unknown-agent");
      expect(el.tagName.toLowerCase()).toBe("div");
    });
  });

  describe("gauge agent (icon=gauge — group with circle)", () => {
    it("renders svg with data-testid agent-icon-perf", () => {
      render(<AgentIcon agent="perf" />);
      expect(screen.getByTestId("agent-icon-perf")).toBeInTheDocument();
    });

    it("svg contains a <g> and <circle> for the gauge glyph", () => {
      render(<AgentIcon agent="perf" />);
      const svg = screen.getByTestId("agent-icon-perf");
      expect(svg.querySelector("g")).not.toBeNull();
      expect(svg.querySelector("circle")).not.toBeNull();
    });
  });

  describe("database agent (icon=database — group with ellipse)", () => {
    it("renders svg with data-testid agent-icon-db", () => {
      render(<AgentIcon agent="db" />);
      expect(screen.getByTestId("agent-icon-db")).toBeInTheDocument();
    });

    it("svg contains an <ellipse> element for the database glyph", () => {
      render(<AgentIcon agent="db" />);
      const svg = screen.getByTestId("agent-icon-db");
      expect(svg.querySelector("ellipse")).not.toBeNull();
    });
  });

  describe("a11y agent (icon=a11y — group with circle + path)", () => {
    it("renders svg with data-testid agent-icon-a11y", () => {
      render(<AgentIcon agent="a11y" />);
      expect(screen.getByTestId("agent-icon-a11y")).toBeInTheDocument();
    });

    it("svg contains a <g> and <circle> for the a11y glyph", () => {
      render(<AgentIcon agent="a11y" />);
      const svg = screen.getByTestId("agent-icon-a11y");
      expect(svg.querySelector("g")).not.toBeNull();
      expect(svg.querySelector("circle")).not.toBeNull();
    });
  });

  describe("keyword-resolved agents — component renders correct icon", () => {
    it("requirements-engineer renders the book svg (path with M4 4h5)", () => {
      render(<AgentIcon agent="requirements-engineer" />);
      const svg = screen.getByTestId("agent-icon-requirements-engineer");
      expect(svg.tagName.toLowerCase()).toBe("svg");
      const path = svg.querySelector("path");
      expect(path).not.toBeNull();
      // book glyph path starts with M4 4h5
      expect(path!.getAttribute("d")).toContain("M4 4h5");
    });

    it("system-architect renders the blueprint svg", () => {
      render(<AgentIcon agent="system-architect" />);
      const svg = screen.getByTestId("agent-icon-system-architect");
      const path = svg.querySelector("path");
      expect(path).not.toBeNull();
      expect(path!.getAttribute("d")).toContain("M3 4h14v12H3z");
    });

    it("ui-ux-designer renders svg with palette (circle children)", () => {
      render(<AgentIcon agent="ui-ux-designer" />);
      const svg = screen.getByTestId("agent-icon-ui-ux-designer");
      expect(svg.querySelector("circle")).not.toBeNull();
    });
  });

  describe("initial-fallback — unknown agent renders div not svg", () => {
    it("xyzzy renders a div with data-testid agent-icon-xyzzy", () => {
      render(<AgentIcon agent="xyzzy" />);
      expect(screen.getByTestId("agent-icon-xyzzy")).toBeInTheDocument();
    });

    it("xyzzy fallback shows letter 'X'", () => {
      render(<AgentIcon agent="xyzzy" />);
      const el = screen.getByTestId("agent-icon-xyzzy");
      expect(el.textContent).toBe("X");
    });

    it("xyzzy fallback element is a div not an svg", () => {
      render(<AgentIcon agent="xyzzy" />);
      const el = screen.getByTestId("agent-icon-xyzzy");
      expect(el.tagName.toLowerCase()).toBe("div");
    });

    it("foo-bar fallback shows letter 'F'", () => {
      render(<AgentIcon agent="foo-bar" />);
      const el = screen.getByTestId("agent-icon-foo-bar");
      expect(el.textContent).toBe("F");
    });
  });

  describe("backward-compat — existing 'unknown-agent' test still expects rect OR div", () => {
    it("renders without crash for unknown-agent", () => {
      expect(() => render(<AgentIcon agent="unknown-agent" />)).not.toThrow();
    });
  });
});
