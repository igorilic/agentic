import { describe, expect, it, vi } from "vitest";
import { dispatchSlashCommand, type SlashServices } from "../slash/dispatcher";

function makeMockServices(overrides: Partial<SlashServices> = {}): SlashServices {
  return {
    plan: vi.fn().mockResolvedValue("run-123"),
    status: vi.fn().mockResolvedValue("Status text"),
    cancel: vi.fn().mockResolvedValue(true),
    ...overrides,
  };
}

describe("dispatchSlashCommand", () => {
  it("plan calls services.plan with the ticket", async () => {
    const services = makeMockServices();
    const result = await dispatchSlashCommand({ kind: "plan", ticket: "#42" }, services);
    expect(services.plan).toHaveBeenCalledWith("#42");
    expect(result.runId).toBe("run-123");
    expect(result.message).toContain("run-123");
    expect(result.message).toContain("#42");
  });

  it("status calls services.status with runId or null", async () => {
    const services = makeMockServices();
    await dispatchSlashCommand({ kind: "status", runId: null }, services);
    expect(services.status).toHaveBeenCalledWith(null);
    await dispatchSlashCommand({ kind: "status", runId: "run-x" }, services);
    expect(services.status).toHaveBeenCalledWith("run-x");
  });

  it("cancel returns 'Cancelled' message when service returns true", async () => {
    const services = makeMockServices({ cancel: vi.fn().mockResolvedValue(true) });
    const result = await dispatchSlashCommand({ kind: "cancel", runId: "run-x" }, services);
    expect(result.message).toContain("Cancelled");
    expect(result.message).toContain("run-x");
  });

  it("cancel returns 'No active run' when service returns false", async () => {
    const services = makeMockServices({ cancel: vi.fn().mockResolvedValue(false) });
    const result = await dispatchSlashCommand({ kind: "cancel", runId: "run-x" }, services);
    expect(result.message).toContain("No active run");
  });

  it("propagates service rejection", async () => {
    const services = makeMockServices({
      plan: vi.fn().mockRejectedValue(new Error("backend down")),
    });
    await expect(
      dispatchSlashCommand({ kind: "plan", ticket: "x" }, services),
    ).rejects.toThrow("backend down");
  });
});
