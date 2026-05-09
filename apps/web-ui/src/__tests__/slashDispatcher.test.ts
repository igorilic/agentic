import { describe, expect, it, vi } from "vitest";
import { dispatchSlashCommand, type SlashServices } from "../slash/dispatcher";
import { SLASH_COMMAND_LIBRARY } from "../slash/library";

function makeMockServices(overrides: Partial<SlashServices> = {}): SlashServices {
  return {
    plan: vi.fn().mockResolvedValue("run-123"),
    status: vi.fn().mockResolvedValue("Status text"),
    cancel: vi.fn().mockResolvedValue(true),
    ...overrides,
  };
}

describe("dispatchSlashCommand", () => {
  it("plan calls services.plan with the ticket and an undefined backend by default", async () => {
    const services = makeMockServices();
    const result = await dispatchSlashCommand({ kind: "plan", ticket: "#42" }, services);
    expect(services.plan).toHaveBeenCalledWith("#42", undefined);
    expect(result.runId).toBe("run-123");
    expect(result.message).toContain("run-123");
    expect(result.message).toContain("#42");
  });

  it("plan forwards the parsed BackendKind through to services.plan", async () => {
    const services = makeMockServices();
    const result = await dispatchSlashCommand(
      { kind: "plan", ticket: "implement export", backend: "copilot-cli" },
      services,
    );
    expect(services.plan).toHaveBeenCalledWith("implement export", "copilot-cli");
    expect(result.message).toContain("copilot-cli");
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

  // /help dispatcher tests
  it("dispatcher_help_produces_system_message_with_all_library_commands", async () => {
    const services = makeMockServices();
    const result = await dispatchSlashCommand({ kind: "help" }, services);
    // Must list all library commands by name
    for (const cmd of SLASH_COMMAND_LIBRARY) {
      expect(result.message).toContain(cmd.name);
    }
    // Specifically assert the 4 known names + help itself
    expect(result.message).toContain("plan");
    expect(result.message).toContain("brainstorm");
    expect(result.message).toContain("develop");
    expect(result.message).toContain("spec");
    expect(result.message).toContain("help");
  });

  it("dispatcher_help_does_not_invoke_backend — no service calls are made", async () => {
    const services = makeMockServices();
    await dispatchSlashCommand({ kind: "help" }, services);
    expect(services.plan).not.toHaveBeenCalled();
    expect(services.status).not.toHaveBeenCalled();
    expect(services.cancel).not.toHaveBeenCalled();
  });
});
