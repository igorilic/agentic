import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import AgentPicker from "../components/AgentPicker";

describe("AgentPicker", () => {
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

  it("each visible row shows agent name and description", () => {
    render(<AgentPicker {...defaultProps} />);
    // QA agent is not excluded
    expect(screen.getByText("QA")).toBeInTheDocument();
    expect(screen.getByText("Runs tests, checks edge cases")).toBeInTheDocument();
    // Reviewer agent is not excluded
    expect(screen.getByText("Reviewer")).toBeInTheDocument();
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

  it("search matches by id substring: typing 'db' returns DB Migrator row", () => {
    render(<AgentPicker {...defaultProps} />);
    const input = screen.getByPlaceholderText("Search agents…");
    fireEvent.change(input, { target: { value: "db" } });
    const rows = screen.queryAllByTestId(/^agent-picker-row-/);
    expect(rows).toHaveLength(1);
    expect(screen.getByTestId("agent-picker-row-db")).toBeInTheDocument();
  });

  it("calls onPick with agent id when row is clicked", async () => {
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
});
