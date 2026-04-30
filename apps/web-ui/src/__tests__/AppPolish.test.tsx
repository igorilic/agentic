import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "../App";

// Mock the Tauri APIs since they're not available in jsdom.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// invokeMock is a named reference so we can reset it in afterEach.
// Return type is unknown so per-test mockImplementation can return strings or arrays.
const invokeMock = vi.fn(async (_cmd: string): Promise<unknown> => undefined);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...(args as [string])),
}));

// HeaderBar uses useTheme which calls window.matchMedia — stub it for jsdom.
function stubMatchMedia() {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: false,
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

beforeEach(() => {
  stubMatchMedia();
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  invokeMock.mockReset();
  invokeMock.mockImplementation(async (cmd: string): Promise<unknown> => {
    if (cmd === "list_runs") return [];
    if (cmd === "list_findings") return [];
    if (cmd === "get_event_history") return [];
    return undefined;
  });
});

afterEach(() => {
  document.documentElement.removeAttribute("data-theme");
  localStorage.clear();
  invokeMock.mockReset();
});

describe("App polish (W.9.8)", () => {
  it("agent cards render with the per-agent SVG glyph (W.9.2)", () => {
    render(<App />);
    const cards = screen.getAllByTestId(/^agent-card-/);
    // At least one card contains an SVG with viewBox 0 0 20 20.
    const hasIcon = cards.some(
      (card) => card.querySelector('svg[viewBox="0 0 20 20"]') !== null,
    );
    expect(hasIcon).toBe(true);
  });

  it("header settings button renders the heroicons cog (W.9.5)", () => {
    render(<App />);
    const settingsBtn = screen.getByTestId("header-settings");
    const svg = settingsBtn.querySelector("svg");
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute("viewBox")).toBe("0 0 20 20");
  });

  it("chat composer has the new placeholder (W.9.3)", () => {
    render(<App />);
    // ChatColumn (W.4.6) overrides the composer textarea testid to "chat-input"
    // for backward compatibility with ChatPane.test.tsx + mentionChatPane.test.tsx.
    // The placeholder contract is the same — verify on the actual testid in
    // the App tree.
    const textarea = screen.getByTestId("chat-input") as HTMLTextAreaElement;
    expect(textarea.placeholder).toBe(
      "Ask a question, or use /plan, /develop, /@agent…",
    );
  });

  it("chat composer renders the New-spec doc icon (W.9.4)", () => {
    render(<App />);
    expect(screen.getByTestId("chat-composer-new-spec")).toBeInTheDocument();
  });

  it("after SpecDialog submit, IssueColumn title updates to the typed title (B1)", async () => {
    invokeMock.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "start_ticket_run") return "run-b1-test";
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });
    const user = userEvent.setup();
    render(<App />);

    // Open SpecDialog from the chat composer new-spec button
    await user.click(screen.getByTestId("chat-composer-new-spec"));
    await user.type(screen.getByTestId("spec-dialog-title-input"), "fix issue 88");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    await waitFor(() => {
      expect(screen.getByTestId("issue-title")).toHaveTextContent("fix issue 88");
    });
  });

  it("after SpecDialog submit with body, IssueColumn body paragraph reflects the body field (B1-body)", async () => {
    invokeMock.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "start_ticket_run") return "run-b1-body-test";
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByTestId("chat-composer-new-spec"));
    await user.type(screen.getByTestId("spec-dialog-title-input"), "fix issue 88");
    await user.type(screen.getByTestId("spec-dialog-body-textarea"), "This is the description.");
    await user.click(screen.getByTestId("spec-dialog-submit"));

    await waitFor(() => {
      const para = screen.getByTestId("issue-body-paragraph");
      expect(para).toHaveTextContent("This is the description.");
    });
  });

  it("/plan splits on first dot — IssueColumn title and body paragraph reflect the split (B2)", async () => {
    invokeMock.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "start_ticket_run") return "run-plan-split";
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });
    const user = userEvent.setup();
    render(<App />);

    await user.type(
      screen.getByTestId("chat-input"),
      "/plan Add rate limiting. Pro tier issue.",
    );
    await user.click(screen.getByTestId("chat-send"));

    await waitFor(() => {
      expect(screen.getByTestId("issue-title")).toHaveTextContent("Add rate limiting");
    });
    const para = screen.getByTestId("issue-body-paragraph");
    expect(para).toHaveTextContent("Pro tier issue.");
  });

  it("clicking New-spec opens SpecDialog; Esc closes it (W.9.4)", async () => {
    render(<App />);
    expect(screen.queryByTestId("spec-dialog")).toBeNull();
    fireEvent.click(screen.getByTestId("chat-composer-new-spec"));
    expect(screen.getByTestId("spec-dialog")).toBeInTheDocument();

    // Esc closes — dispatch keyDown on the dialog panel itself, since W.6.5's
    // fix loop moved the Esc handler from the backdrop to the panel.
    const panel = screen.getByTestId("spec-dialog");
    fireEvent.keyDown(panel, { key: "Escape" });
    await waitFor(() => {
      expect(screen.queryByTestId("spec-dialog")).toBeNull();
    });
  });
});
