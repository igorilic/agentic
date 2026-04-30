import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { installDevInvokeMock } from "../utils/devInvokeMock";

type WindowWithTauri = typeof window & {
  __TAURI_INTERNALS__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
};

describe("installDevInvokeMock", () => {
  beforeEach(() => {
    delete (window as WindowWithTauri).__TAURI_INTERNALS__;
    vi.unstubAllEnvs();
  });

  afterEach(() => {
    delete (window as WindowWithTauri).__TAURI_INTERNALS__;
    vi.unstubAllEnvs();
  });

  it("does NOT install in production builds", () => {
    vi.stubEnv("DEV", false);
    const installed = installDevInvokeMock();
    expect(installed).toBe(false);
    expect((window as WindowWithTauri).__TAURI_INTERNALS__).toBeUndefined();
  });

  it("does NOT install when real Tauri internals are present", () => {
    vi.stubEnv("DEV", true);
    const realInvoke = vi.fn();
    (window as WindowWithTauri).__TAURI_INTERNALS__ = { invoke: realInvoke };
    const installed = installDevInvokeMock();
    expect(installed).toBe(false);
    expect((window as WindowWithTauri).__TAURI_INTERNALS__?.invoke).toBe(realInvoke);
  });

  it("installs in dev when no real Tauri internals", () => {
    vi.stubEnv("DEV", true);
    const installed = installDevInvokeMock();
    expect(installed).toBe(true);
    expect(typeof (window as WindowWithTauri).__TAURI_INTERNALS__?.invoke).toBe("function");
  });

  describe("mock invoke responses", () => {
    let invoke: (cmd: string, args?: unknown) => Promise<unknown>;
    beforeEach(() => {
      vi.stubEnv("DEV", true);
      installDevInvokeMock();
      invoke = (window as WindowWithTauri).__TAURI_INTERNALS__!.invoke;
    });

    it("start_ticket_run returns a run_id starting with dev-mock-", async () => {
      const result = (await invoke("start_ticket_run", {})) as { run_id: string };
      expect(result.run_id).toMatch(/^dev-mock-\d+$/);
    });

    it("cancel_run resolves with undefined", async () => {
      await expect(invoke("cancel_run", {})).resolves.toBeUndefined();
    });

    it("mention_agent echoes the agent name", async () => {
      const result = (await invoke("mention_agent", { agent: "qa" })) as { agent: string; dispatched: boolean };
      expect(result.agent).toBe("qa");
      expect(result.dispatched).toBe(true);
    });

    it("chat_send_message returns a user_message + reply pair", async () => {
      const result = (await invoke("chat_send_message", { content: "hi" })) as { user_message: { content: string }; reply: { content: string; role: string } };
      expect(result.user_message.content).toBe("hi");
      expect(result.reply.role).toBe("assistant");
    });

    it("list_runs / list_findings / list_auth_accounts / get_event_history return empty arrays", async () => {
      await expect(invoke("list_runs", { limit: 50 })).resolves.toEqual([]);
      await expect(invoke("list_findings", {})).resolves.toEqual([]);
      await expect(invoke("list_auth_accounts")).resolves.toEqual([]);
      await expect(invoke("get_event_history", {})).resolves.toEqual([]);
    });

    it("plugin:event|listen returns a numeric subscription id", async () => {
      const id = await invoke("plugin:event|listen");
      expect(typeof id).toBe("number");
    });

    it("unknown commands log a warning and resolve with undefined", async () => {
      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      const result = await invoke("totally_unknown_command", { foo: "bar" });
      expect(result).toBeUndefined();
      expect(warnSpy).toHaveBeenCalledWith(
        "[dev-invoke-mock] unhandled command: totally_unknown_command",
        { foo: "bar" },
      );
      warnSpy.mockRestore();
    });
  });
});
