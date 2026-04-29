import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import ChatMessage from "../components/ChatMessage";

describe("ChatMessage", () => {
  describe("user variant", () => {
    it("renders with data-testid chat-message-user", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="hello world" />
      );
      expect(screen.getByTestId("chat-message-user")).toBeInTheDocument();
    });

    it("renders userName text", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="hello world" />
      );
      expect(screen.getByText("Erica")).toBeInTheDocument();
    });

    it("renders timestamp text", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="hello world" />
      );
      expect(screen.getByText("14:02")).toBeInTheDocument();
    });

    it("renders body text", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="hello world" />
      );
      expect(screen.getByText("hello world")).toBeInTheDocument();
    });

    it("renders avatar placeholder element with stable testid", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="hello world" />
      );
      expect(screen.getByTestId("chat-message-user-avatar")).toBeInTheDocument();
    });
  });

  describe("system variant", () => {
    it("renders with data-testid chat-message-system", () => {
      render(
        <ChatMessage kind="system" body="── Architect handed off to Developer ──" />
      );
      expect(screen.getByTestId("chat-message-system")).toBeInTheDocument();
    });

    it("renders the system message body text matching the hand-off pattern", () => {
      render(
        <ChatMessage kind="system" body="── Architect handed off to Developer ──" />
      );
      expect(
        screen.getByText(/── Architect handed off to Developer ──/)
      ).toBeInTheDocument();
    });

    it("does NOT render an avatar element", () => {
      render(
        <ChatMessage kind="system" body="── Architect handed off to Developer ──" />
      );
      expect(screen.queryByTestId("chat-message-user-avatar")).toBeNull();
      expect(screen.queryByTestId("chat-message-agent-avatar")).toBeNull();
    });

    it("system message element className contains text-center", () => {
      render(
        <ChatMessage kind="system" body="── Architect handed off to Developer ──" />
      );
      const el = screen.getByTestId("chat-message-system");
      expect(el.className).toContain("text-center");
    });
  });

  describe("agent variant — architect", () => {
    it("renders with data-testid chat-message-agent and data-agent architect", () => {
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      const el = screen.getByTestId("chat-message-agent");
      expect(el).toBeInTheDocument();
      expect(el).toHaveAttribute("data-agent", "architect");
    });

    it("renders agent name text", () => {
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      expect(screen.getByText("architect")).toBeInTheDocument();
    });

    it("renders timestamp text", () => {
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      expect(screen.getByText("14:03")).toBeInTheDocument();
    });

    it("renders body text", () => {
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      expect(screen.getByText("speccing now")).toBeInTheDocument();
    });

    it("agent name element has class containing text-agent-architect", () => {
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      const nameEl = screen.getByTestId("chat-message-agent-name");
      expect(nameEl.className).toContain("text-agent-architect");
    });

    it("body bubble element exists with data-testid chat-message-agent-bubble", () => {
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      expect(screen.getByTestId("chat-message-agent-bubble")).toBeInTheDocument();
    });

    it("bubble borderLeftColor references --agent-architect CSS variable", () => {
      // jsdom does not compute CSS variables; we assert the raw string is present
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      expect(bubble.style.borderLeftColor).toContain("--agent-architect");
    });

    it("bubble backgroundColor for architect contains the blue tint rgb values", () => {
      // AGENT_TINT_RGBA architect = "rgb(59 130 246 / 0.06)" — assert component parts
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      expect(bubble.style.backgroundColor).toContain("59");
      expect(bubble.style.backgroundColor).toContain("130");
      expect(bubble.style.backgroundColor).toContain("246");
    });
  });

  describe("agent variant — different agents", () => {
    it("developer variant: agent name has class text-agent-developer", () => {
      render(
        <ChatMessage kind="agent" agent="developer" timestamp="14:04" body="coding" />
      );
      const nameEl = screen.getByTestId("chat-message-agent-name");
      expect(nameEl.className).toContain("text-agent-developer");
    });

    it("developer variant: bubble borderLeftColor references --agent-developer", () => {
      render(
        <ChatMessage kind="agent" agent="developer" timestamp="14:04" body="coding" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      expect(bubble.style.borderLeftColor).toContain("--agent-developer");
    });

    it("qa variant: agent name has class text-agent-qa", () => {
      render(
        <ChatMessage kind="agent" agent="qa" timestamp="14:05" body="testing" />
      );
      const nameEl = screen.getByTestId("chat-message-agent-name");
      expect(nameEl.className).toContain("text-agent-qa");
    });

    it("qa variant: bubble borderLeftColor references --agent-qa", () => {
      render(
        <ChatMessage kind="agent" agent="qa" timestamp="14:05" body="testing" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      expect(bubble.style.borderLeftColor).toContain("--agent-qa");
    });
  });

  describe("agent variant — unknown agent fallback", () => {
    it("does NOT crash when rendering an unknown agent", () => {
      expect(() =>
        render(
          <ChatMessage kind="agent" agent="researcher" timestamp="14:06" body="digging" />
        )
      ).not.toThrow();
    });

    it("unknown agent: agent name falls back to text-fg class (no per-agent class)", () => {
      render(
        <ChatMessage kind="agent" agent="researcher" timestamp="14:06" body="digging" />
      );
      const nameEl = screen.getByTestId("chat-message-agent-name");
      // Unknown agents use the text-fg fallback class
      expect(nameEl.className).toContain("text-fg");
      expect(nameEl.className).not.toContain("text-agent-researcher");
    });

    it("unknown agent: bubble borderLeftColor uses fg-muted fallback", () => {
      render(
        <ChatMessage kind="agent" agent="researcher" timestamp="14:06" body="digging" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      // Fallback: var(--fg-muted)
      expect(bubble.style.borderLeftColor).toContain("--fg-muted");
    });
  });
});
