import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import HeaderBar, { formatMmSs } from "../components/HeaderBar";

function stubMatchMedia(matches: boolean) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: query === "(prefers-color-scheme: dark)" ? matches : false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

const defaultProps = {
  brand: "Agentic",
  ticketSlug: null as string | null,
  runState: "idle" as const,
  onOpenSettings: vi.fn(),
  onRunPipeline: vi.fn(),
  onStopRun: vi.fn(),
  onRerun: vi.fn(),
  elapsedMs: null as number | null,
};

describe("HeaderBar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    stubMatchMedia(false);
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
    // localStorage is empty and matchMedia returns false (light) — hook defaults to light
    render(<HeaderBar {...defaultProps} />);
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
    // Seed localStorage so the hook initialises to dark
    localStorage.setItem("agentic.theme", "dark");
    render(<HeaderBar {...defaultProps} />);
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

  // W.1.2 — running pill and Stop button
  it("shows running pill with formatted elapsed time when runState is running", () => {
    render(<HeaderBar {...defaultProps} runState="running" elapsedMs={154000} />);
    const runState = screen.getByTestId("header-run-state");
    expect(runState).toHaveTextContent(/Pipeline running · 02:34/);
  });

  it("shows Stop button when runState is running", () => {
    render(<HeaderBar {...defaultProps} runState="running" elapsedMs={154000} />);
    expect(screen.getByTestId("header-stop")).toBeInTheDocument();
  });

  // W.1.2 — completed pill and Re-run button
  it("shows completed pill with formatted elapsed time when runState is completed", () => {
    render(<HeaderBar {...defaultProps} runState="completed" elapsedMs={258000} />);
    const runState = screen.getByTestId("header-run-state");
    expect(runState).toHaveTextContent(/Completed · 04:18/);
  });

  it("shows Re-run button when runState is completed", () => {
    render(<HeaderBar {...defaultProps} runState="completed" elapsedMs={258000} />);
    expect(screen.getByTestId("header-rerun")).toBeInTheDocument();
  });

  // W.1.2 — callback wiring
  it("clicking Stop fires onStopRun once", async () => {
    const user = userEvent.setup();
    const onStopRun = vi.fn();
    render(<HeaderBar {...defaultProps} runState="running" elapsedMs={1000} onStopRun={onStopRun} />);
    await user.click(screen.getByTestId("header-stop"));
    expect(onStopRun).toHaveBeenCalledTimes(1);
  });

  it("clicking Re-run fires onRerun once", async () => {
    const user = userEvent.setup();
    const onRerun = vi.fn();
    render(<HeaderBar {...defaultProps} runState="completed" elapsedMs={1000} onRerun={onRerun} />);
    await user.click(screen.getByTestId("header-rerun"));
    expect(onRerun).toHaveBeenCalledTimes(1);
  });

  // W.1.2 — idle button absent when non-idle
  it("does not render Run pipeline button when runState is running (explicit symmetry check)", () => {
    render(<HeaderBar {...defaultProps} runState="running" elapsedMs={5000} />);
    expect(screen.queryByTestId("header-run")).toBeNull();
  });

  // W.1.2 — completed: only Re-run shown, Run and Stop absent
  it("does not render Run pipeline or Stop button when runState is completed", () => {
    render(<HeaderBar {...defaultProps} runState="completed" elapsedMs={5000} />);
    expect(screen.queryByTestId("header-run")).toBeNull();
    expect(screen.queryByTestId("header-stop")).toBeNull();
  });

  // S1 — Re-run button absent when running (symmetric negative for completed-direction negatives)
  it("does not render Re-run button when runState is running", () => {
    render(<HeaderBar {...defaultProps} runState="running" elapsedMs={5000} />);
    expect(screen.queryByTestId("header-rerun")).toBeNull();
  });

  // S2 — nothing renders in run-state slot when running but elapsedMs is null (pre-first-tick)
  it("does not render running pill or Stop button when runState is running and elapsedMs is null", () => {
    render(<HeaderBar {...defaultProps} runState="running" elapsedMs={null} />);
    expect(screen.queryByTestId("header-running-pill")).toBeNull();
    expect(screen.queryByTestId("header-stop")).toBeNull();
  });

  // W.1.3 — theme toggle integration tests
  it("clicking theme toggle from light flips data-theme to dark and aria-pressed to true", () => {
    render(<HeaderBar {...defaultProps} />);
    const toggle = screen.getByTestId("header-theme-toggle");
    // Initially light: no data-theme attribute and aria-pressed=false
    expect(document.documentElement.getAttribute("data-theme")).toBeNull();
    expect(toggle).toHaveAttribute("aria-pressed", "false");

    fireEvent.click(toggle);

    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(toggle).toHaveAttribute("aria-pressed", "true");
  });

  it("clicking theme toggle from dark back to light removes data-theme and sets aria-pressed to false", () => {
    render(<HeaderBar {...defaultProps} />);
    const toggle = screen.getByTestId("header-theme-toggle");

    // First click: light → dark
    fireEvent.click(toggle);
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");

    // Second click: dark → light
    fireEvent.click(toggle);
    expect(document.documentElement.getAttribute("data-theme")).toBeNull();
    expect(toggle).toHaveAttribute("aria-pressed", "false");
  });

  it("theme persists to localStorage and a fresh instance reads dark from it", () => {
    const { unmount } = render(<HeaderBar {...defaultProps} />);
    const toggle = screen.getByTestId("header-theme-toggle");

    // Toggle to dark
    fireEvent.click(toggle);
    expect(localStorage.getItem("agentic.theme")).toBe("dark");

    unmount();
    cleanup();

    // Mount a fresh instance — should read dark from localStorage
    render(<HeaderBar {...defaultProps} />);
    const freshToggle = screen.getByTestId("header-theme-toggle");
    expect(freshToggle).toHaveAttribute("aria-pressed", "true");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  // W.9.5 — heroicons cog SVG for settings button
  describe("settings icon SVG", () => {
    it("settings button svg has viewBox 0 0 20 20", () => {
      render(<HeaderBar {...defaultProps} />);
      const btn = screen.getByTestId("header-settings");
      const svg = btn.querySelector("svg");
      expect(svg).not.toBeNull();
      expect(svg!.getAttribute("viewBox")).toBe("0 0 20 20");
    });

    it("settings button path d starts with heroicons cog prefix", () => {
      render(<HeaderBar {...defaultProps} />);
      const btn = screen.getByTestId("header-settings");
      const path = btn.querySelector("path");
      expect(path).not.toBeNull();
      const d = path!.getAttribute("d") ?? "";
      expect(d.startsWith("M7.84 1.804A1 1 0 018.82 1")).toBe(true);
    });

    it("settings button path d contains inner circle subpath", () => {
      render(<HeaderBar {...defaultProps} />);
      const btn = screen.getByTestId("header-settings");
      const path = btn.querySelector("path");
      expect(path).not.toBeNull();
      const d = path!.getAttribute("d") ?? "";
      expect(d).toContain("M10 13a3 3 0 100-6");
    });

    it("old proprietary path starting with M7.0 0.5 is not in document", () => {
      render(<HeaderBar {...defaultProps} />);
      const allPaths = Array.from(document.querySelectorAll("path"));
      const oldPath = allPaths.find((p) => (p.getAttribute("d") ?? "").startsWith("M7.0 0.5"));
      expect(oldPath).toBeUndefined();
    });
  });

  // F.1.3 — backend segmented control
  describe("backend segmented control", () => {
    it("renders the backend segmented control with two buttons", () => {
      render(<HeaderBar {...defaultProps} />);
      expect(screen.getByTestId("header-backend-claude-code")).toBeInTheDocument();
      expect(screen.getByTestId("header-backend-copilot-cli")).toBeInTheDocument();
    });

    it("marks claude-code as pressed by default", () => {
      render(<HeaderBar {...defaultProps} />);
      expect(screen.getByTestId("header-backend-claude-code")).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByTestId("header-backend-copilot-cli")).toHaveAttribute("aria-pressed", "false");
    });

    it("clicking copilot-cli flips the active backend", async () => {
      const user = userEvent.setup();
      render(<HeaderBar {...defaultProps} />);
      await user.click(screen.getByTestId("header-backend-copilot-cli"));
      expect(screen.getByTestId("header-backend-copilot-cli")).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByTestId("header-backend-claude-code")).toHaveAttribute("aria-pressed", "false");
      expect(localStorage.getItem("agentic.backend")).toBe("copilot-cli");
    });

    it("clicking claude-code flips back when copilot-cli was active", async () => {
      const user = userEvent.setup();
      localStorage.setItem("agentic.backend", "copilot-cli");
      render(<HeaderBar {...defaultProps} />);
      // copilot-cli should start as active
      expect(screen.getByTestId("header-backend-copilot-cli")).toHaveAttribute("aria-pressed", "true");
      await user.click(screen.getByTestId("header-backend-claude-code"));
      expect(screen.getByTestId("header-backend-claude-code")).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByTestId("header-backend-copilot-cli")).toHaveAttribute("aria-pressed", "false");
      expect(localStorage.getItem("agentic.backend")).toBe("claude-code");
    });

    it("backend selector container appears before header-run-state in the DOM", () => {
      render(<HeaderBar {...defaultProps} />);
      const selector = screen.getByTestId("header-backend-selector");
      const runState = screen.getByTestId("header-run-state");
      // DOCUMENT_POSITION_FOLLOWING = 4 means runState follows selector
      const position = selector.compareDocumentPosition(runState);
      expect(position & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it("click on already-active button is a no-op (no state flip, no localStorage write beyond mount)", () => {
      // Clean slate — localStorage is empty before render.
      localStorage.clear();
      render(<HeaderBar {...defaultProps} />);

      const claudeBtn = screen.getByTestId("header-backend-claude-code");
      const copilotBtn = screen.getByTestId("header-backend-copilot-cli");

      // Default: claude-code is active. The useEffect fires on mount and writes the
      // default value, so localStorage now holds "claude-code".
      expect(claudeBtn).toHaveAttribute("aria-pressed", "true");
      expect(copilotBtn).toHaveAttribute("aria-pressed", "false");
      expect(localStorage.getItem("agentic.backend")).toBe("claude-code");

      fireEvent.click(claudeBtn);

      // React bails on the identical setState call — no re-render, no useEffect.
      // The segmented control state must not flip.
      expect(claudeBtn).toHaveAttribute("aria-pressed", "true");
      expect(copilotBtn).toHaveAttribute("aria-pressed", "false");
      // localStorage value stays "claude-code" (written on mount, not overwritten again).
      expect(localStorage.getItem("agentic.backend")).toBe("claude-code");
    });
  });

  // W.1.2 — formatMmSs unit tests
  describe("formatMmSs", () => {
    it("formats 0ms as 00:00", () => {
      expect(formatMmSs(0)).toBe("00:00");
    });

    it("formats 59000ms as 00:59", () => {
      expect(formatMmSs(59000)).toBe("00:59");
    });

    it("formats 60000ms as 01:00", () => {
      expect(formatMmSs(60000)).toBe("01:00");
    });

    it("formats 154000ms as 02:34", () => {
      expect(formatMmSs(154000)).toBe("02:34");
    });

    it("formats 3599000ms as 59:59", () => {
      expect(formatMmSs(3599000)).toBe("59:59");
    });

    it("formats 3600000ms as 60:00 (no hour rollover — MM keeps counting)", () => {
      expect(formatMmSs(3600000)).toBe("60:00");
    });

    it("clamps negative values to 00:00", () => {
      expect(formatMmSs(-5)).toBe("00:00");
    });
  });
});
