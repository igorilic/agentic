import { act, renderHook, waitFor } from "@testing-library/react";
import { useTauriEvents, MAX_EVENTS } from "../hooks/useTauriEvents";

let capturedHandler: ((event: { payload: unknown }) => void) | null = null;

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_channel: string, handler: (event: { payload: unknown }) => void) => {
    capturedHandler = handler;
    return () => {};
  }),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe("useTauriEvents", () => {
  beforeEach(() => {
    capturedHandler = null;
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
});
