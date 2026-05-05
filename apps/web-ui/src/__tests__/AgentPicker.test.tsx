import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import AgentPicker from "../components/AgentPicker";
import { useDiscoverableAgents } from "../hooks/useDiscoverableAgents";
import type { AgentInfoDto } from "../types/agents";

vi.mock("../hooks/useDiscoverableAgents", () => ({
  useDiscoverableAgents: vi.fn(),
}));

// Minimal stand-in for the full AGENT_LIBRARY using slug names.
// The `name` field in AgentInfoDto is the agent slug (e.g. "architect"),
// matching what PipelineBar and ChatComposer pass in excludeIds.
const FULL_MOCK_AGENTS: AgentInfoDto[] = [
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

describe("AgentPicker", () => {
  beforeEach(() => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: FULL_MOCK_AGENTS,
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
  });

  const defaultProps = {
    excludeIds: ["architect", "developer"],
    onPick: vi.fn(),
    onClose: vi.fn(),
  };

  beforeEach(() => {
    defaultProps.onPick.mockClear();
    defaultProps.onClose.mockClear();
  });

  it("renders search input with correct placeholder", () => {
    render(<AgentPicker {...defaultProps} />);
    expect(screen.getByPlaceholderText("Search agents…")).toBeInTheDocument();
  });

  it("renders picker container with data-testid agent-picker", () => {
    render(<AgentPicker {...defaultProps} />);
    expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
  });

  it("shows 10 rows when excludeIds has 2 agents", () => {
    render(<AgentPicker {...defaultProps} />);
    const rows = screen.queryAllByTestId(/^agent-picker-row-/);
    expect(rows).toHaveLength(10);
  });

  it("excluded agents are not in the list", () => {
    render(<AgentPicker {...defaultProps} />);
    expect(screen.queryByTestId("agent-picker-row-architect")).toBeNull();
    expect(screen.queryByTestId("agent-picker-row-developer")).toBeNull();
  });

  it("each visible row shows agent description", () => {
    render(<AgentPicker {...defaultProps} />);
    // QA agent is not excluded
    expect(screen.getByText("qa")).toBeInTheDocument();
    expect(screen.getByText("Runs tests, checks edge cases")).toBeInTheDocument();
    // Reviewer agent is not excluded
    expect(screen.getByText("reviewer")).toBeInTheDocument();
    expect(screen.getByText("Code review & feedback")).toBeInTheDocument();
  });

  it("filters to only QA row when typing 'qa'", async () => {
    render(<AgentPicker {...defaultProps} />);
    const input = screen.getByPlaceholderText("Search agents…");
    fireEvent.change(input, { target: { value: "qa" } });
    const rows = screen.queryAllByTestId(/^agent-picker-row-/);
    expect(rows).toHaveLength(1);
    expect(screen.getByTestId("agent-picker-row-qa")).toBeInTheDocument();
  });

  it("search is case-insensitive: typing Reviewer returns reviewer row", () => {
    render(<AgentPicker {...defaultProps} />);
    const input = screen.getByPlaceholderText("Search agents…");
    fireEvent.change(input, { target: { value: "Reviewer" } });
    const rows = screen.queryAllByTestId(/^agent-picker-row-/);
    expect(rows).toHaveLength(1);
    expect(screen.getByTestId("agent-picker-row-reviewer")).toBeInTheDocument();
  });

  it("search matches by name substring: typing 'db' returns DB row", () => {
    render(<AgentPicker {...defaultProps} />);
    const input = screen.getByPlaceholderText("Search agents…");
    fireEvent.change(input, { target: { value: "db" } });
    const rows = screen.queryAllByTestId(/^agent-picker-row-/);
    expect(rows).toHaveLength(1);
    expect(screen.getByTestId("agent-picker-row-db")).toBeInTheDocument();
  });

  it("calls onPick with agent name when row is clicked", async () => {
    const user = userEvent.setup();
    render(<AgentPicker {...defaultProps} />);
    await user.click(screen.getByTestId("agent-picker-row-qa"));
    expect(defaultProps.onPick).toHaveBeenCalledWith("qa");
  });

  it("calls onClose when Escape is pressed", async () => {
    const user = userEvent.setup();
    render(<AgentPicker {...defaultProps} />);
    const input = screen.getByPlaceholderText("Search agents…");
    await user.click(input);
    await user.keyboard("{Escape}");
    expect(defaultProps.onClose).toHaveBeenCalledTimes(1);
  });

  it("renders all 12 agents when excludeIds is empty", () => {
    render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);
    const rows = screen.queryAllByTestId(/^agent-picker-row-/);
    expect(rows).toHaveLength(12);
  });

  describe("W.9.2 — AgentIcon SVG in each row's leading avatar", () => {
    it("each visible row contains an AgentIcon SVG", () => {
      render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);
      // architect row should contain an SVG icon
      const architectRow = screen.getByTestId("agent-picker-row-architect");
      const svg = architectRow.querySelector("svg");
      expect(svg).not.toBeNull();
      expect(svg!.getAttribute("data-testid")).toBe("agent-icon-architect");
    });

    it("architect row avatar does NOT contain the old bg-bg-surface-2 placeholder", () => {
      render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);
      const architectRow = screen.getByTestId("agent-picker-row-architect");
      const placeholder = architectRow.querySelector(".bg-bg-surface-2");
      expect(placeholder).toBeNull();
    });

    it("uses shadow-popover class (not shadow-modal) on the picker container", () => {
      render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);
      const picker = screen.getByTestId("agent-picker");
      expect(picker.className).toContain("shadow-popover");
      expect(picker.className).not.toContain("shadow-modal");
    });
  });

  // ---------------------------------------------------------------------------
  // I.8 — live discovery tests
  // ---------------------------------------------------------------------------

  it("shows discoverable agents with their description and source chip", () => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: [
        { name: "architect", description: "plan", source: "project" },
        { name: "qa", description: "verify", source: "home" },
      ],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });

    render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);

    // architect row has description and source chip
    const architectRow = screen.getByTestId("agent-picker-row-architect");
    expect(architectRow).toHaveTextContent("plan");
    const architectChip = screen.getByTestId("agent-source-chip-architect");
    expect(architectChip).toHaveTextContent("project");

    // qa row has home chip
    const qaChip = screen.getByTestId("agent-source-chip-qa");
    expect(qaChip).toHaveTextContent("home");
  });

  it("filters out agents already in excludeIds (pipelineAgents)", () => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: [
        { name: "architect", description: "plan", source: "project" },
        { name: "qa", description: "verify", source: "home" },
      ],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });

    render(
      <AgentPicker excludeIds={["architect"]} onPick={vi.fn()} onClose={vi.fn()} />
    );

    expect(screen.queryByTestId("agent-picker-row-architect")).not.toBeInTheDocument();
    expect(screen.getByTestId("agent-picker-row-qa")).toBeInTheDocument();
  });

  it("renders a loading state while the hook is fetching", () => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: [],
      isLoading: true,
      error: null,
      refetch: vi.fn(),
    });

    render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);

    expect(screen.getByTestId("agent-picker-loading")).toBeInTheDocument();
  });

  it("renders an empty state when no agents are discoverable", () => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: [],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });

    render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);

    const empty = screen.getByTestId("agent-picker-empty");
    expect(empty).toBeInTheDocument();
    expect(empty).toHaveTextContent(
      "No agents discovered. Run `agentic-cli init` or `agentic-cli init --copilot`."
    );
  });

  it("renders an error message when the hook returns an error", () => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: [],
      isLoading: false,
      error: "permission denied",
      refetch: vi.fn(),
    });

    render(<AgentPicker excludeIds={[]} onPick={vi.fn()} onClose={vi.fn()} />);

    expect(screen.getByTestId("agent-picker-error")).toBeInTheDocument();
    expect(screen.getByTestId("agent-picker-error")).toHaveTextContent(
      "Failed to list agents: permission denied"
    );
  });
});
