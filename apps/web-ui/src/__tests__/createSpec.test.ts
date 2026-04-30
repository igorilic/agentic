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
    invokeMock.mockResolvedValueOnce({ run_id: "run-42" });

    await createSpec("My new spec");

    expect(invokeMock).toHaveBeenCalledWith("start_ticket_run", {
      ticket: "My new spec",
      backend: "claude-code",
      model: null,
    });
  });

  it("returns the run_id string when IPC resolves with { run_id: '...' }", async () => {
    invokeMock.mockResolvedValueOnce({ run_id: "run-99" });

    const result = await createSpec("Spec title");

    expect(result).toBe("run-99");
  });

  it("returns undefined when IPC response lacks run_id (malformed)", async () => {
    invokeMock.mockResolvedValueOnce({});

    const result = await createSpec("Spec title");

    expect(result).toBeUndefined();
  });

  it("returns undefined when IPC response has non-string run_id", async () => {
    invokeMock.mockResolvedValueOnce({ run_id: 123 });

    const result = await createSpec("Spec title");

    expect(result).toBeUndefined();
  });

  it("propagates IPC rejection (does not swallow errors)", async () => {
    invokeMock.mockRejectedValueOnce(new Error("ipc failure"));

    await expect(createSpec("Spec title")).rejects.toThrow("ipc failure");
  });
});
