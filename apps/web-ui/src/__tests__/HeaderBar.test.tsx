import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import HeaderBar from "../components/HeaderBar";

const defaultProps = {
  brand: "Agentic",
  ticketSlug: null as string | null,
  runState: "idle" as const,
  theme: "light" as const,
  onThemeToggle: vi.fn(),
  onOpenSettings: vi.fn(),
  onRunPipeline: vi.fn(),
  onStopRun: vi.fn(),
  onRerun: vi.fn(),
  elapsedMs: null as number | null,
};

describe("HeaderBar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the header bar with h-12 class", () => {
    render(<HeaderBar {...defaultProps} />);
    const header = screen.getByTestId("header-bar");
    expect(header).toBeInTheDocument();
    expect(header.className).toMatch(/h-12/);
  });

  it("renders brand text", () => {
    render(<HeaderBar {...defaultProps} />);
    expect(screen.getByText("Agentic")).toBeInTheDocument();
  });

  it("does not render slug when ticketSlug is null", () => {
    render(<HeaderBar {...defaultProps} ticketSlug={null} />);
    expect(screen.queryByTestId("header-slug")).toBeNull();
  });

  it("renders 'Run pipeline' button when runState is idle", () => {
    render(<HeaderBar {...defaultProps} />);
    const btn = screen.getByTestId("header-run");
    expect(btn).toBeInTheDocument();
    expect(btn).toHaveTextContent("Run pipeline");
  });

  it("renders theme toggle with aria-pressed=false when theme is light", () => {
    render(<HeaderBar {...defaultProps} theme="light" />);
    const toggle = screen.getByTestId("header-theme-toggle");
    expect(toggle).toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-pressed", "false");
  });

  it("renders slug when ticketSlug is provided", () => {
    render(<HeaderBar {...defaultProps} ticketSlug="AGT-204" />);
    const slug = screen.getByTestId("header-slug");
    expect(slug).toBeInTheDocument();
    expect(slug).toHaveTextContent("AGT-204");
  });

  it("renders theme toggle with aria-pressed=true when theme is dark", () => {
    render(<HeaderBar {...defaultProps} theme="dark" />);
    const toggle = screen.getByTestId("header-theme-toggle");
    expect(toggle).toHaveAttribute("aria-pressed", "true");
  });

  it("calls onRunPipeline when Run pipeline button is clicked", async () => {
    const user = userEvent.setup();
    const onRunPipeline = vi.fn();
    render(<HeaderBar {...defaultProps} onRunPipeline={onRunPipeline} />);
    await user.click(screen.getByTestId("header-run"));
    expect(onRunPipeline).toHaveBeenCalledTimes(1);
  });

  it("calls onThemeToggle when theme toggle is clicked", async () => {
    const user = userEvent.setup();
    const onThemeToggle = vi.fn();
    render(<HeaderBar {...defaultProps} onThemeToggle={onThemeToggle} />);
    await user.click(screen.getByTestId("header-theme-toggle"));
    expect(onThemeToggle).toHaveBeenCalledTimes(1);
  });

  it("calls onOpenSettings when settings icon is clicked", async () => {
    const user = userEvent.setup();
    const onOpenSettings = vi.fn();
    render(<HeaderBar {...defaultProps} onOpenSettings={onOpenSettings} />);
    await user.click(screen.getByTestId("header-settings"));
    expect(onOpenSettings).toHaveBeenCalledTimes(1);
  });

  // F1 — run-state slot must have role="status" and aria-live="polite"
  it("run-state wrapper has role=status and aria-live=polite", () => {
    render(<HeaderBar {...defaultProps} />);
    const wrapper = screen.getByTestId("header-run-state");
    expect(wrapper.getAttribute("role")).toBe("status");
    expect(wrapper.getAttribute("aria-live")).toBe("polite");
  });

  // F2 — avatar div must carry role="img" so aria-label is reliably exposed
  it("avatar div has role=img", () => {
    render(<HeaderBar {...defaultProps} />);
    const avatar = screen.getByTestId("header-avatar");
    expect(avatar.getAttribute("role")).toBe("img");
  });

  // F3 — "Run pipeline" button must not appear when runState is not idle
  it("does not render Run pipeline button when runState is running", () => {
    render(<HeaderBar {...defaultProps} runState="running" elapsedMs={1000} />);
    expect(screen.queryByTestId("header-run")).toBeNull();
  });
});
