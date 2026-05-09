import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import ChatComposer from "../components/ChatComposer";
import type { AgentInfoDto } from "../types/agents";

// AgentPicker now calls useDiscoverableAgents. Mock it here so ChatComposer
// tests do not require Tauri IPC infrastructure.
vi.mock("../hooks/useDiscoverableAgents", () => ({
  useDiscoverableAgents: vi.fn(),
}));
import { useDiscoverableAgents } from "../hooks/useDiscoverableAgents";

const CHAT_COMPOSER_AGENTS: AgentInfoDto[] = [
  { name: "architect",  description: "Designs system & breaks down work",  source: "project" },
  { name: "developer",  description: "Writes code & tests",                source: "project" },
  { name: "qa",         description: "Runs tests, checks edge cases",      source: "project" },
  { name: "reviewer",   description: "Code review & feedback",             source: "project" },
  { name: "researcher", description: "Gathers context, reads docs",        source: "project" },
  { name: "security",   description: "Audits for vulnerabilities",         source: "project" },
  { name: "perf",       description: "Profiles & optimises hot paths",     source: "project" },
  { name: "docs",       description: "Updates README, API docs",           source: "project" },
  { name: "designer",   description: "UX & visual review",                 source: "project" },
  { name: "db",         description: "Schema migrations & data",           source: "project" },
  { name: "devops",     description: "CI/CD & deploy config",              source: "project" },
  { name: "a11y",       description: "WCAG compliance pass",               source: "project" },
];

describe("ChatComposer", () => {
  beforeEach(() => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: CHAT_COMPOSER_AGENTS,
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
  });

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

  describe("R2 — trimmed value on send", () => {
    it("chip click then immediate send fires onSend with trimmed value (no trailing space)", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const chip = screen.getByTestId("chat-composer-chip-plan");
      const sendBtn = screen.getByTestId("chat-composer-send");

      await userEvent.click(chip);
      await userEvent.click(sendBtn);

      expect(onSend).toHaveBeenCalledWith("/plan");
    });
  });

  describe("C — whitespace-only input guard", () => {
    it("typing only spaces and clicking send does NOT fire onSend", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");
      const sendBtn = screen.getByTestId("chat-composer-send");

      fireEvent.change(textarea, { target: { value: "   " } });
      await userEvent.click(sendBtn);

      expect(onSend).not.toHaveBeenCalled();
    });
  });

  describe("testid overrides", () => {
    it("uses inputTestId override when provided", () => {
      render(<ChatComposer onSend={vi.fn()} inputTestId="chat-input" />);
      expect(screen.getByTestId("chat-input")).toBeInTheDocument();
      expect(screen.queryByTestId("chat-composer-textarea")).toBeNull();
    });

    it("uses sendTestId override when provided", () => {
      render(<ChatComposer onSend={vi.fn()} sendTestId="chat-send" />);
      expect(screen.getByTestId("chat-send")).toBeInTheDocument();
      expect(screen.queryByTestId("chat-composer-send")).toBeNull();
    });
  });

  describe("W.9.3 — layout polish", () => {
    it("chip placement: textarea appears before chip in document order", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");
      const chip = screen.getByTestId("chat-composer-chip-plan");
      // Node.DOCUMENT_POSITION_FOLLOWING (4) is set when chip comes after textarea
      const position = textarea.compareDocumentPosition(chip);
      expect(position & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it("placeholder text matches handoff verbatim (U+2026 ellipsis)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea") as HTMLTextAreaElement;
      expect(textarea.placeholder).toBe("Ask a question, or use /plan, /develop, /@agent…");
    });

    it("send button contains paper-plane SVG path; up-arrow path is absent", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const sendBtn = screen.getByTestId("chat-composer-send");
      const svg = sendBtn.querySelector("svg");
      expect(svg).not.toBeNull();
      const path = svg!.querySelector("path");
      expect(path).not.toBeNull();
      expect(path!.getAttribute("d")).toBe("M3 10l14-7-3 16-4-7-7-2z");
      // Old up-arrow path must not be in the document
      const allPaths = document.querySelectorAll("path");
      allPaths.forEach((p) => {
        expect(p.getAttribute("d")).not.toBe("M8 14V2 M3 7l5-5 5 5");
      });
    });

    it("send button bg is bg-bg-surface-2 when draft empty, bg-[#18181b] after typing", async () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");
      const sendBtn = screen.getByTestId("chat-composer-send");

      // Empty draft
      expect(sendBtn.className).toContain("bg-bg-surface-2");

      // Type content
      await userEvent.type(textarea, "hello");
      expect(sendBtn.className).toContain("bg-[#18181b]");
      expect(sendBtn.className).toContain("text-white");
    });

    it("chat-composer-input-wrapper wraps textarea and send button, has border + rounded-xl", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");
      const sendBtn = screen.getByTestId("chat-composer-send");

      const wrapper = textarea.closest('[data-testid="chat-composer-input-wrapper"]');
      expect(wrapper).not.toBeNull();
      expect(wrapper).toContainElement(sendBtn);
      expect(wrapper!.className).toContain("border");
      expect(wrapper!.className).toContain("rounded-xl");
    });
  });

  describe("R1 — chip border token", () => {
    it("Plan chip uses border-border token, not border-border-strong", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const chip = screen.getByTestId("chat-composer-chip-plan");

      expect(chip.className).not.toContain("border-border-strong");
      expect(chip.className).toContain("border-border");
    });
  });

  describe("slash popover", () => {
    it("typing / opens the slash popover", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/" } });

      expect(screen.getByTestId("slash-popover")).toBeInTheDocument();
    });

    it("typing /pl shows only the plan row", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/pl" } });

      expect(screen.getByTestId("slash-popover-row-plan")).toBeInTheDocument();
      expect(screen.queryByTestId("slash-popover-row-brainstorm")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-develop")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-spec")).toBeNull();
    });

    it("typing /p shows only plan (only command starting with p)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/p" } });

      expect(screen.getByTestId("slash-popover-row-plan")).toBeInTheDocument();
      expect(screen.queryByTestId("slash-popover-row-brainstorm")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-develop")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-spec")).toBeNull();
    });

    it("typing /b shows only brainstorm", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/b" } });

      expect(screen.getByTestId("slash-popover-row-brainstorm")).toBeInTheDocument();
      expect(screen.queryByTestId("slash-popover-row-plan")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-develop")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-spec")).toBeNull();
    });

    it("typing /d shows only develop", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/d" } });

      expect(screen.getByTestId("slash-popover-row-develop")).toBeInTheDocument();
      expect(screen.queryByTestId("slash-popover-row-plan")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-brainstorm")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-spec")).toBeNull();
    });

    it("typing /s shows only spec", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/s" } });

      expect(screen.getByTestId("slash-popover-row-spec")).toBeInTheDocument();
      expect(screen.queryByTestId("slash-popover-row-plan")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-brainstorm")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-develop")).toBeNull();
    });

    it("typing /x shows popover with no rows (empty match)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/x" } });

      expect(screen.getByTestId("slash-popover")).toBeInTheDocument();
      expect(screen.queryByTestId("slash-popover-row-plan")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-brainstorm")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-develop")).toBeNull();
      expect(screen.queryByTestId("slash-popover-row-spec")).toBeNull();
    });

    it("ArrowDown then Enter selects the second command (brainstorm)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/" } });
      fireEvent.keyDown(textarea, { key: "ArrowDown" });
      fireEvent.keyDown(textarea, { key: "Enter" });

      expect(textarea).toHaveValue("/brainstorm ");
    });

    it("Enter with no ArrowDown selects the first command (plan)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/" } });
      fireEvent.keyDown(textarea, { key: "Enter" });

      expect(textarea).toHaveValue("/plan ");
    });

    it("Esc dismisses the popover", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/" } });
      expect(screen.getByTestId("slash-popover")).toBeInTheDocument();

      fireEvent.keyDown(textarea, { key: "Escape" });

      expect(screen.queryByTestId("slash-popover")).toBeNull();
    });

    it("Esc dismisses popover but preserves textarea text", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/pl" } });
      fireEvent.keyDown(textarea, { key: "Escape" });

      expect(screen.queryByTestId("slash-popover")).toBeNull();
      expect(textarea).toHaveValue("/pl");
    });

    it("ArrowUp from index 0 wraps to last command (help)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/" } });
      fireEvent.keyDown(textarea, { key: "ArrowUp" });
      fireEvent.keyDown(textarea, { key: "Enter" });

      expect(textarea).toHaveValue("/help ");
    });

    it("popover does NOT open when input contains a space (/plan AGT-99)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/plan AGT-99" } });

      expect(screen.queryByTestId("slash-popover")).toBeNull();
    });

    it("popover does NOT open when slash is not at the start (hello /)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "hello /" } });

      expect(screen.queryByTestId("slash-popover")).toBeNull();
    });

    it("Cmd+Enter when popover is open sends the current value and closes popover", () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/" } });
      expect(screen.getByTestId("slash-popover")).toBeInTheDocument();

      fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

      expect(onSend).toHaveBeenCalledWith("/");
      expect(screen.queryByTestId("slash-popover")).toBeNull();
    });

    it("reopens popover when value changes after Esc", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      // type "/pl" — popover opens
      fireEvent.change(textarea, { target: { value: "/pl" } });
      expect(screen.getByTestId("slash-popover")).toBeInTheDocument();

      // press Escape — popover closes for the value "/pl"
      fireEvent.keyDown(textarea, { key: "Escape" });
      expect(screen.queryByTestId("slash-popover")).toBeNull();

      // type one more char so value becomes "/pla"
      fireEvent.change(textarea, { target: { value: "/pla" } });

      // popover reopens because escClosedForValue ("/pl") !== current value ("/pla")
      expect(screen.getByTestId("slash-popover")).toBeInTheDocument();
    });

    it("popover has aria-label for screen readers", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/" } });

      expect(screen.getByTestId("slash-popover").getAttribute("aria-label")).toBe("Slash commands");
    });
  });

  describe("W.9.4 — New-spec button", () => {
    it("renders chat-composer-new-spec button when onCreateSpec is provided", () => {
      render(<ChatComposer onSend={vi.fn()} onCreateSpec={vi.fn()} />);
      expect(screen.getByTestId("chat-composer-new-spec")).toBeInTheDocument();
    });

    it("new-spec button is inside chat-composer-input-wrapper", () => {
      render(<ChatComposer onSend={vi.fn()} onCreateSpec={vi.fn()} />);
      const btn = screen.getByTestId("chat-composer-new-spec");
      const wrapper = btn.closest('[data-testid="chat-composer-input-wrapper"]');
      expect(wrapper).not.toBeNull();
    });

    it("new-spec button appears BEFORE send button in document order", () => {
      render(<ChatComposer onSend={vi.fn()} onCreateSpec={vi.fn()} />);
      const newSpec = screen.getByTestId("chat-composer-new-spec");
      const send = screen.getByTestId("chat-composer-send");
      // DOCUMENT_POSITION_FOLLOWING (4) means send comes after newSpec
      const position = newSpec.compareDocumentPosition(send);
      expect(position & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it("new-spec button has aria-label='Create spec'", () => {
      render(<ChatComposer onSend={vi.fn()} onCreateSpec={vi.fn()} />);
      const btn = screen.getByTestId("chat-composer-new-spec");
      expect(btn).toHaveAttribute("aria-label", "Create spec");
    });

    it("new-spec button inner SVG path d attribute matches doc-icon glyph", () => {
      render(<ChatComposer onSend={vi.fn()} onCreateSpec={vi.fn()} />);
      const btn = screen.getByTestId("chat-composer-new-spec");
      const path = btn.querySelector("path");
      expect(path).not.toBeNull();
      expect(path!.getAttribute("d")).toBe(
        "M5 3h7l3 3v11H5zM12 3v3h3M7 9h6M7 12h6M7 15h4",
      );
    });

    it("clicking new-spec button calls onCreateSpec once with no args", async () => {
      const onCreateSpec = vi.fn();
      render(<ChatComposer onSend={vi.fn()} onCreateSpec={onCreateSpec} />);
      const btn = screen.getByTestId("chat-composer-new-spec");

      await userEvent.click(btn);

      expect(onCreateSpec).toHaveBeenCalledTimes(1);
    });

    it("does NOT render new-spec button when onCreateSpec is omitted", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      expect(screen.queryByTestId("chat-composer-new-spec")).toBeNull();
    });
  });

  describe("mention popover", () => {
    it("typing @ alone opens the mention popover", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "@" } });

      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();
    });

    it("typing @arc opens the mention popover", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "@arc" } });

      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();
    });

    it("typing @arc filters to show only architect row", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "@arc" } });

      expect(screen.getByTestId("agent-picker-row-architect")).toBeInTheDocument();
      // other agents should be filtered out
      expect(screen.queryByTestId("agent-picker-row-developer")).toBeNull();
      expect(screen.queryByTestId("agent-picker-row-qa")).toBeNull();
    });

    it("typing 'hi @arc' opens mention popover with architect filtered", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "hi @arc" } });

      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();
      expect(screen.getByTestId("agent-picker-row-architect")).toBeInTheDocument();
      expect(screen.queryByTestId("agent-picker-row-developer")).toBeNull();
    });

    it("typing 'hi@arc' (no space before @) does NOT open mention popover", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "hi@arc" } });

      expect(screen.queryByTestId("mention-popover")).toBeNull();
    });

    it("typing 'hi @arc done' (space after query) does NOT open mention popover", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "hi @arc done" } });

      expect(screen.queryByTestId("mention-popover")).toBeNull();
    });

    it("typing 'hi @' shows all 12 agents", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "hi @" } });

      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();
      const rows = screen.queryAllByTestId(/^agent-picker-row-/);
      expect(rows).toHaveLength(12);
    });

    it("clicking architect row when value is '@' sets textarea to '@architect '", async () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "@" } });
      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();

      await userEvent.click(screen.getByTestId("agent-picker-row-architect"));

      expect(textarea).toHaveValue("@architect ");
      expect(screen.queryByTestId("mention-popover")).toBeNull();
    });

    it("clicking architect row when value is 'hi @arc' sets textarea to 'hi @architect '", async () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "hi @arc" } });
      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();

      await userEvent.click(screen.getByTestId("agent-picker-row-architect"));

      expect(textarea).toHaveValue("hi @architect ");
      expect(screen.queryByTestId("mention-popover")).toBeNull();
    });

    it("pressing Escape closes the mention popover and preserves input", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "@" } });
      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();

      // AgentPicker's internal keyboard handler fires onClose on Escape
      // which sets mentionEscClosedForValue
      const agentPicker = screen.getByTestId("agent-picker");
      fireEvent.keyDown(agentPicker, { key: "Escape" });

      expect(screen.queryByTestId("mention-popover")).toBeNull();
      expect(textarea).toHaveValue("@");
    });

    it("typing past the trigger reopens mention popover after Esc", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      // open at "@ar"
      fireEvent.change(textarea, { target: { value: "@ar" } });
      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();

      // Escape closes it for "@ar"
      const agentPicker = screen.getByTestId("agent-picker");
      fireEvent.keyDown(agentPicker, { key: "Escape" });
      expect(screen.queryByTestId("mention-popover")).toBeNull();

      // type one more char — value is now "@arc", different from "@ar"
      fireEvent.change(textarea, { target: { value: "@arc" } });

      // popover reopens
      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();
    });

    it("'/plan @arc' — slash popover does NOT open (no slash match), mention popover DOES open", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "/plan @arc" } });

      // slash popover does NOT open (value doesn't match /^\/[a-z]*$/)
      expect(screen.queryByTestId("slash-popover")).toBeNull();
      // mention popover DOES open (space before @ and no space in query)
      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();
    });

    it("the mention popover's AgentPicker has width class w-60 (240 px narrow)", () => {
      render(<ChatComposer onSend={vi.fn()} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      fireEvent.change(textarea, { target: { value: "@" } });

      const agentPicker = screen.getByTestId("agent-picker");
      expect(agentPicker.className).toContain("w-60");
      expect(agentPicker.className).not.toContain("w-80");
    });

    it("Cmd+Enter with '@architect hello' calls onSend with the raw value", async () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");

      // type a value that would invoke parseMention on submission
      fireEvent.change(textarea, { target: { value: "@architect hello" } });
      // close popover first (space after — trigger ended)
      expect(screen.queryByTestId("mention-popover")).toBeNull();

      fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });

      expect(onSend).toHaveBeenCalledWith("@architect hello");
    });

    it("filters picker rows as the user types past the initial trigger", () => {
      const onSend = vi.fn();
      render(<ChatComposer onSend={onSend} />);
      const textarea = screen.getByTestId("chat-composer-textarea");
      // First trigger: "@a" — query "a", 6 agents match (architect, qa, researcher, perf, db, a11y)
      fireEvent.change(textarea, { target: { value: "@a" } });
      expect(screen.getByTestId("mention-popover")).toBeInTheDocument();
      const rowsAfterA = screen.queryAllByTestId(/^agent-picker-row-/);
      expect(rowsAfterA.length).toBeGreaterThan(2);
      // Continue typing: "@archi" — only architect matches
      fireEvent.change(textarea, { target: { value: "@archi" } });
      const rows = screen.queryAllByTestId(/^agent-picker-row-/);
      expect(rows).toHaveLength(1);
      expect(screen.getByTestId("agent-picker-row-architect")).toBeInTheDocument();
    });
  });
});
