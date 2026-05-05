import { describe, expect, it, vi, afterEach } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// Import after mock is set up
const { createSpec } = await import("../utils/createSpec");

const DEFAULT_AGENTS = ["architect", "tdd-developer", "qa", "reviewer"];

afterEach(() => invokeMock.mockReset());

describe("createSpec", () => {
  it("calls invoke with start_ticket_run and correct arg shape including agents", async () => {
    invokeMock.mockResolvedValueOnce("run-42");

    await createSpec("My new spec", "claude-code", DEFAULT_AGENTS);

    expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
      ticket: "My new spec",
      backend: "claude-code",
      model: null,
      agents: DEFAULT_AGENTS,
    });
  });

  it("threads the active backend into the IPC payload", async () => {
    invokeMock.mockResolvedValueOnce("run-99");

    await createSpec("My new spec", "copilot-cli", DEFAULT_AGENTS);

    expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
      ticket: "My new spec",
      backend: "copilot-cli",
      model: null,
      agents: DEFAULT_AGENTS,
    });
  });

  it("forwards the agents array verbatim to the IPC payload", async () => {
    invokeMock.mockResolvedValueOnce("run-agents-2");
    const twoAgents = ["architect", "reviewer"];

    await createSpec("Spec with 2 agents", "claude-code", twoAgents);

    expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
      ticket: "Spec with 2 agents",
      backend: "claude-code",
      model: null,
      agents: twoAgents,
    });
  });

  it("returns the run_id when invoke resolves with a string", async () => {
    invokeMock.mockResolvedValueOnce("run-123");

    const result = await createSpec("Add rate limiting", "claude-code", DEFAULT_AGENTS);

    expect(result).toBe("run-123");
  });

  it("returns undefined when invoke resolves with a non-string (object)", async () => {
    invokeMock.mockResolvedValueOnce({ unexpected: true });

    const result = await createSpec("Add rate limiting", "claude-code", DEFAULT_AGENTS);

    expect(result).toBeUndefined();
  });

  it("returns undefined when invoke resolves with null", async () => {
    invokeMock.mockResolvedValueOnce(null);

    const result = await createSpec("Spec title", "claude-code", DEFAULT_AGENTS);

    expect(result).toBeUndefined();
  });

  it("returns undefined when invoke resolves with undefined", async () => {
    invokeMock.mockResolvedValueOnce(undefined);

    const result = await createSpec("Spec title", "claude-code", DEFAULT_AGENTS);

    expect(result).toBeUndefined();
  });

  it("propagates IPC rejection (does not swallow errors)", async () => {
    invokeMock.mockRejectedValueOnce(new Error("ipc failure"));

    await expect(createSpec("Spec title", "claude-code", DEFAULT_AGENTS)).rejects.toThrow("ipc failure");
  });

  // I.7 — reject empty agents before invoking IPC
  it("rejects empty agents with a clear error before invoking IPC", async () => {
    await expect(
      createSpec("My spec", "claude-code", []),
    ).rejects.toThrow("Pick at least one agent before creating a spec");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
