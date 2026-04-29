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
      // AGENT_TINT_RGBA architect = "rgb(59 130 246 / 0.04)" — assert component parts
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      expect(bubble.style.backgroundColor).toContain("59");
      expect(bubble.style.backgroundColor).toContain("130");
      expect(bubble.style.backgroundColor).toContain("246");
    });

    it("bubble backgroundColor alpha for architect is 0.04 per spec §3.4", () => {
      // Spec §3.4 line 219 — agent bubble: rgba(<accent>, 0.04)
      render(
        <ChatMessage kind="agent" agent="architect" timestamp="14:03" body="speccing now" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      expect(bubble.style.backgroundColor).toContain("0.04");
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

    it("reviewer variant: agent name has class text-agent-reviewer", () => {
      render(
        <ChatMessage kind="agent" agent="reviewer" timestamp="14:06" body="reviewing" />
      );
      const nameEl = screen.getByTestId("chat-message-agent-name");
      expect(nameEl.className).toContain("text-agent-reviewer");
    });

    it("reviewer variant: bubble borderLeftColor references --agent-reviewer", () => {
      render(
        <ChatMessage kind="agent" agent="reviewer" timestamp="14:06" body="reviewing" />
      );
      const bubble = screen.getByTestId("chat-message-agent-bubble");
      expect(bubble.style.borderLeftColor).toContain("--agent-reviewer");
    });
  });

  describe("inline token highlighter — user variant", () => {
    it("renders 2 chat-token elements for a body with one slash command and one mention", () => {
      render(
        <ChatMessage
          kind="user"
          userName="Erica"
          timestamp="14:02"
          body="/develop AGT-204 @architect please"
        />
      );
      expect(screen.getAllByTestId("chat-token")).toHaveLength(2);
    });

    it("first token text is /develop and second token text is @architect", () => {
      render(
        <ChatMessage
          kind="user"
          userName="Erica"
          timestamp="14:02"
          body="/develop AGT-204 @architect please"
        />
      );
      const tokens = screen.getAllByTestId("chat-token");
      expect(tokens[0].textContent).toBe("/develop");
      expect(tokens[1].textContent).toBe("@architect");
    });

    it("message textContent contains the plain text segments ' AGT-204 ' and ' please'", () => {
      render(
        <ChatMessage
          kind="user"
          userName="Erica"
          timestamp="14:02"
          body="/develop AGT-204 @architect please"
        />
      );
      const el = screen.getByTestId("chat-message-user");
      expect(el.textContent).toContain(" AGT-204 ");
      expect(el.textContent).toContain(" please");
    });

    it("body with no slash or mention tokens renders zero chat-token testids", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="hello world" />
      );
      expect(screen.queryAllByTestId("chat-token")).toHaveLength(0);
    });

    it("token at the START of the body is highlighted — body '/plan AGT-99'", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="/plan AGT-99" />
      );
      const tokens = screen.getAllByTestId("chat-token");
      expect(tokens).toHaveLength(1);
      expect(tokens[0].textContent).toBe("/plan");
    });

    it("token at the END of the body is highlighted — body 'please /run'", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="please /run" />
      );
      const tokens = screen.getAllByTestId("chat-token");
      expect(tokens).toHaveLength(1);
      expect(tokens[0].textContent).toBe("/run");
    });

    it("adjacent tokens '/dev @qa' produce 2 tokens with the space between as plain text", () => {
      render(
        <ChatMessage kind="user" userName="Erica" timestamp="14:02" body="/dev @qa" />
      );
      const tokens = screen.getAllByTestId("chat-token");
      expect(tokens).toHaveLength(2);
      expect(tokens[0].textContent).toBe("/dev");
      expect(tokens[1].textContent).toBe("@qa");
      const el = screen.getByTestId("chat-message-user");
      expect(el.textContent).toContain(" ");
    });

    it("each chat-token element has className containing bg-[rgba(253,230,138,0.4)] and rounded-sm", () => {
      render(
        <ChatMessage
          kind="user"
          userName="Erica"
          timestamp="14:02"
          body="/develop AGT-204 @architect please"
        />
      );
      const tokens = screen.getAllByTestId("chat-token");
      for (const token of tokens) {
        expect(token.className).toContain("bg-[rgba(253,230,138,0.4)]");
        expect(token.className).toContain("rounded-sm");
      }
    });
  });

  describe("inline token highlighter — agent variant", () => {
    it("agent variant highlights a single @mention token", () => {
      render(
        <ChatMessage kind="agent" agent="developer" timestamp="14:05" body="ok @qa over to you" />
      );
      const tokens = screen.getAllByTestId("chat-token");
      expect(tokens).toHaveLength(1);
      expect(tokens[0].textContent).toBe("@qa");
    });
  });

  describe("inline token highlighter — system variant", () => {
    it("system messages do NOT highlight tokens even when body contains a slash command", () => {
      render(
        <ChatMessage kind="system" body="── /handoff to architect ──" />
      );
      expect(screen.queryAllByTestId("chat-token")).toHaveLength(0);
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
