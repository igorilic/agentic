import { describe, expect, it, vi, afterEach } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// Import after mock is set up
const { createSpec } = await import("../utils/createSpec");

afterEach(() => invokeMock.mockReset());

describe("createSpec", () => {
  it("calls invoke with start_ticket_run and correct arg shape", async () => {
    invokeMock.mockResolvedValueOnce("run-42");

    await createSpec("My new spec");

    expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
      ticket: "My new spec",
      backend: "claude-code",
      model: null,
    });
  });

  it("returns the run_id when invoke resolves with a string", async () => {
    invokeMock.mockResolvedValueOnce("run-123");

    const result = await createSpec("Add rate limiting");

    expect(result).toBe("run-123");
  });

  it("returns undefined when invoke resolves with a non-string (object)", async () => {
    invokeMock.mockResolvedValueOnce({ unexpected: true });

    const result = await createSpec("Add rate limiting");

    expect(result).toBeUndefined();
  });

  it("returns undefined when invoke resolves with null", async () => {
    invokeMock.mockResolvedValueOnce(null);

    const result = await createSpec("Spec title");

    expect(result).toBeUndefined();
  });

  it("returns undefined when invoke resolves with undefined", async () => {
    invokeMock.mockResolvedValueOnce(undefined);

    const result = await createSpec("Spec title");

    expect(result).toBeUndefined();
  });

  it("propagates IPC rejection (does not swallow errors)", async () => {
    invokeMock.mockRejectedValueOnce(new Error("ipc failure"));

    await expect(createSpec("Spec title")).rejects.toThrow("ipc failure");
  });
});
