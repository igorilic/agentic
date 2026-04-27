import { act, renderHook, waitFor } from "@testing-library/react";
import { useTauriEvents, MAX_EVENTS } from "../hooks/useTauriEvents";

let capturedHandler: ((event: { payload: unknown }) => void) | null = null;
const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_channel: string, handler: (event: { payload: unknown }) => void) => {
    capturedHandler = handler;
    return () => {};
  }),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("useTauriEvents", () => {
  beforeEach(() => {
    capturedHandler = null;
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("caps the buffer at MAX_EVENTS", async () => {
    const { result } = renderHook(() => useTauriEvents());

    // Wait for the async setup to complete and listener to be registered.
    await waitFor(() => {
      expect(capturedHandler).not.toBeNull();
    });

    // Push MAX_EVENTS + 100 envelopes through the listener.
    const overflow = MAX_EVENTS + 100;
    act(() => {
      for (let i = 0; i < overflow; i++) {
        capturedHandler!({
          payload: {
            schema_version: 1,
            event_id: `e${i}`,
            run_id: "r",
            step_id: null,
            timestamp_ms: i,
            event: { type: "TextDelta", data: { content: String(i) } },
          },
        });
      }
    });

    expect(result.current.length).toBe(MAX_EVENTS);
    // Sliding window keeps the most recent — first event_id should be e100.
    expect(result.current[0].event_id).toBe(`e${overflow - MAX_EVENTS}`);
    expect(result.current[MAX_EVENTS - 1].event_id).toBe(`e${overflow - 1}`);
  });

  it("fetches history when runId is provided and dedupes by event_id", async () => {
    // Mock invoke to return history on get_event_history; succeed on subscribe_events.
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_event_history") {
        return [
          {
            schema_version: 1,
            event_id: "h1",
            run_id: "r1",
            step_id: null,
            timestamp_ms: 1,
            event: { type: "TextDelta", data: { content: "hello" } },
          },
          {
            schema_version: 1,
            event_id: "h2",
            run_id: "r1",
            step_id: null,
            timestamp_ms: 2,
            event: { type: "TextDelta", data: { content: "world" } },
          },
        ];
      }
      if (cmd === "subscribe_events") {
        return undefined;
      }
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    const { result } = renderHook(() => useTauriEvents("r1"));
    await waitFor(() => {
      expect(result.current.length).toBe(2);
    });

    // Simulate a live event with the same event_id as one already in history.
    act(() => {
      capturedHandler!({
        payload: {
          schema_version: 1,
          event_id: "h2",
          run_id: "r1",
          step_id: null,
          timestamp_ms: 2,
          event: { type: "TextDelta", data: { content: "world" } },
        },
      });
    });
    // Still 2 — duplicate suppressed.
    expect(result.current.length).toBe(2);

    // New event with fresh event_id is appended.
    act(() => {
      capturedHandler!({
        payload: {
          schema_version: 1,
          event_id: "h3",
          run_id: "r1",
          step_id: null,
          timestamp_ms: 3,
          event: { type: "TextDelta", data: { content: "new" } },
        },
      });
    });
    expect(result.current.length).toBe(3);
  });

  it("clears state when runId changes to a new run", async () => {
    invokeMock.mockImplementation(async (cmd: string, args?: { runId?: string }) => {
      if (cmd === "get_event_history") {
        if (args?.runId === "run-a") {
          return [
            {
              schema_version: 1,
              event_id: "a1",
              run_id: "run-a",
              step_id: null,
              timestamp_ms: 1,
              event: { type: "TextDelta", data: { content: "from-a" } },
            },
          ];
        }
        if (args?.runId === "run-b") {
          return [
            {
              schema_version: 1,
              event_id: "b1",
              run_id: "run-b",
              step_id: null,
              timestamp_ms: 2,
              event: { type: "TextDelta", data: { content: "from-b" } },
            },
          ];
        }
        return [];
      }
      return undefined;
    });

    const { result, rerender } = renderHook((runId: string | undefined) => useTauriEvents(runId), {
      initialProps: "run-a" as string | undefined,
    });

    await waitFor(() => {
      expect(result.current.length).toBe(1);
    });
    expect(result.current[0].event_id).toBe("a1");

    // Switch to run-b — state should be cleared and run-b history loaded.
    rerender("run-b");

    await waitFor(() => {
      expect(result.current.length).toBe(1);
    });
    expect(result.current[0].event_id).toBe("b1");
  });
});
