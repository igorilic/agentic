import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "../App";

// Mock the Tauri APIs since they're not available in jsdom.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// invokeMock is a named reference so we can reset it in afterEach.
const invokeMock = vi.fn(async (cmd: string) => {
  if (cmd === "list_runs") return [];
  if (cmd === "list_findings") return [];
  return undefined;
});

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
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "list_runs") return [];
    if (cmd === "list_findings") return [];
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
