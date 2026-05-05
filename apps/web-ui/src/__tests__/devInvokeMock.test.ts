import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { installDevInvokeMock } from "../utils/devInvokeMock";

type WindowWithTauri = typeof window & {
  __TAURI_INTERNALS__?: {
    invoke: (cmd: string, args?: unknown) => Promise<unknown>;
    transformCallback?: (handler: (event: unknown) => void, once?: boolean) => number;
  };
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

    it("start_ticket_run returns a bare run_id string starting with dev-mock-", async () => {
      const result = (await invoke("start_ticket_run", {
        ticket: "test",
        backend: "claude-code",
        model: null,
        agents: ["architect", "tdd-developer", "qa", "reviewer"],
      })) as string;
      expect(typeof result).toBe("string");
      expect(result).toMatch(/^dev-mock-\d+$/);
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

  describe("B2 — Tauri 2 callback protocol + simulated event stream", () => {
    let invoke: (cmd: string, args?: unknown) => Promise<unknown>;
    let internals: NonNullable<WindowWithTauri["__TAURI_INTERNALS__"]>;

    beforeEach(() => {
      vi.useFakeTimers();
      vi.stubEnv("DEV", true);
      installDevInvokeMock();
      internals = (window as WindowWithTauri).__TAURI_INTERNALS__!;
      invoke = internals.invoke;
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it("transformCallback is present on __TAURI_INTERNALS__", () => {
      expect(typeof internals.transformCallback).toBe("function");
    });

    it("transformCallback registers a handler and returns a unique numeric id", () => {
      const handlerA = vi.fn();
      const handlerB = vi.fn();
      const idA = internals.transformCallback!(handlerA);
      const idB = internals.transformCallback!(handlerB);
      expect(typeof idA).toBe("number");
      expect(typeof idB).toBe("number");
      expect(idA).not.toBe(idB);
    });

    it("plugin:event|listen with a registered callback id stores the subscription", async () => {
      const handler = vi.fn();
      const cbId = internals.transformCallback!(handler);
      // Register listener for "agentic://event"
      await invoke("plugin:event|listen", { event: "agentic://event", handler: cbId, target: "any" });
      // Now emit an event — handler should be called
      await invoke("start_ticket_run", { ticket: "test", backend: "claude-code", model: null, agents: ["architect", "tdd-developer", "qa", "reviewer"] });
      // Advance timers to fire the first scheduled event (RunStarted at 250ms)
      vi.advanceTimersByTime(300);
      expect(handler).toHaveBeenCalled();
    });

    it("plugin:event|unlisten removes the handler so no further events are dispatched", async () => {
      const handler = vi.fn();
      const cbId = internals.transformCallback!(handler);
      await invoke("plugin:event|listen", { event: "agentic://event", handler: cbId, target: "any" });
      // Unlisten immediately
      await invoke("plugin:event|unlisten", { eventId: cbId });
      // Start a run and advance timers
      await invoke("start_ticket_run", { ticket: "test", backend: "claude-code", model: null, agents: ["architect", "tdd-developer", "qa", "reviewer"] });
      vi.advanceTimersByTime(5000);
      expect(handler).not.toHaveBeenCalled();
    });

    it("after start_ticket_run, emits RunStarted envelope to agentic://event listeners", async () => {
      const received: unknown[] = [];
      const cbId = internals.transformCallback!((event) => received.push(event));
      await invoke("plugin:event|listen", { event: "agentic://event", handler: cbId, target: "any" });
      const runId = await invoke("start_ticket_run", { ticket: "fix issue 88", backend: "claude-code", model: null, agents: ["architect", "tdd-developer", "qa", "reviewer"] }) as string;

      vi.advanceTimersByTime(300); // fire RunStarted at 250ms

      expect(received).toHaveLength(1);
      const envelope = (received[0] as { payload: { event: { type: string }; run_id: string } }).payload;
      expect(envelope.event.type).toBe("RunStarted");
      expect(envelope.run_id).toBe(runId);
    });

    it("emits 4× StepStarted and 4× StepComplete envelopes for the standard agents", async () => {
      const received: unknown[] = [];
      const cbId = internals.transformCallback!((event) => received.push(event));
      await invoke("plugin:event|listen", { event: "agentic://event", handler: cbId, target: "any" });
      await invoke("start_ticket_run", { ticket: "task", backend: "claude-code", model: null, agents: ["architect", "tdd-developer", "qa", "reviewer"] });

      // Advance past RunStarted + all 4 StepStarted+StepComplete pairs (no RunComplete yet)
      // RunStarted: 250ms, then per-agent: start at N, complete at N+800, next start at N+1000
      // 4 agents × 1000ms each = 4000ms after first step starts (250 + 400 = 650ms base)
      vi.advanceTimersByTime(5600); // covers all step events but not RunComplete

      const envelopes = received.map(
        (e) => ((e as { payload: { event: { type: string } } }).payload.event.type)
      );
      expect(envelopes.filter((t) => t === "StepStarted")).toHaveLength(4);
      expect(envelopes.filter((t) => t === "StepComplete")).toHaveLength(4);
    });

    it("emits RunComplete as the last envelope", async () => {
      const received: unknown[] = [];
      const cbId = internals.transformCallback!((event) => received.push(event));
      await invoke("plugin:event|listen", { event: "agentic://event", handler: cbId, target: "any" });
      await invoke("start_ticket_run", { ticket: "task", backend: "claude-code", model: null, agents: ["architect", "tdd-developer", "qa", "reviewer"] });

      vi.advanceTimersByTime(10000); // well past all events

      const types = received.map(
        (e) => ((e as { payload: { event: { type: string } } }).payload.event.type)
      );
      expect(types[types.length - 1]).toBe("RunComplete");
    });

    it("StepComplete envelopes have status='passed' and duration_ms > 0", async () => {
      const received: unknown[] = [];
      const cbId = internals.transformCallback!((event) => received.push(event));
      await invoke("plugin:event|listen", { event: "agentic://event", handler: cbId, target: "any" });
      await invoke("start_ticket_run", { ticket: "task", backend: "claude-code", model: null, agents: ["architect", "tdd-developer", "qa", "reviewer"] });
      vi.advanceTimersByTime(10000);

      const stepCompletes = received
        .map((e) => (e as { payload: { event: { type: string; data: Record<string, unknown> } } }).payload)
        .filter((env) => env.event.type === "StepComplete");

      expect(stepCompletes.length).toBe(4);
      for (const env of stepCompletes) {
        expect(env.event.data.status).toBe("passed");
        expect(typeof env.event.data.duration_ms).toBe("number");
        expect((env.event.data.duration_ms as number)).toBeGreaterThan(0);
      }
    });
  });
});
