import { act, renderHook, waitFor } from "@testing-library/react";
import {
  useMentionEvents,
  MAX_MENTION_EVENTS,
  MENTION_EVENT_CHANNEL,
} from "../hooks/useMentionEvents";

let capturedHandler: ((event: { payload: unknown }) => void) | null = null;
let capturedChannel: string | null = null;
const listenMock = vi.fn(
  async (channel: string, handler: (event: { payload: unknown }) => void) => {
    capturedChannel = channel;
    capturedHandler = handler;
    return () => {};
  },
);

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) =>
    listenMock(
      ...(args as [string, (event: { payload: unknown }) => void]),
    ),
}));

describe("useMentionEvents", () => {
  beforeEach(() => {
    capturedHandler = null;
    capturedChannel = null;
    listenMock.mockClear();
  });

  it("subscribes to the dedicated agentic://mention-event channel", async () => {
    renderHook(() => useMentionEvents());
    await waitFor(() => {
      expect(capturedChannel).toBe(MENTION_EVENT_CHANNEL);
    });
  });

  it("dedupes envelopes by event_id", async () => {
    const { result } = renderHook(() => useMentionEvents());
    await waitFor(() => {
      expect(capturedHandler).not.toBeNull();
    });

    const env = {
      schema_version: 1,
      event_id: "m1",
      run_id: "r1",
      step_id: null,
      timestamp_ms: 1,
      event: { type: "TextDelta", data: { content: "hello" } },
    };

    act(() => {
      capturedHandler!({ payload: env });
      capturedHandler!({ payload: env });
    });

    expect(result.current.length).toBe(1);
  });

  it("caps the buffer at MAX_MENTION_EVENTS", async () => {
    const { result } = renderHook(() => useMentionEvents());
    await waitFor(() => {
      expect(capturedHandler).not.toBeNull();
    });

    const overflow = MAX_MENTION_EVENTS + 50;
    act(() => {
      for (let i = 0; i < overflow; i++) {
        capturedHandler!({
          payload: {
            schema_version: 1,
            event_id: `m${i}`,
            run_id: "r1",
            step_id: null,
            timestamp_ms: i,
            event: { type: "TextDelta", data: { content: String(i) } },
          },
        });
      }
    });

    expect(result.current.length).toBe(MAX_MENTION_EVENTS);
    expect(result.current[0].event_id).toBe(`m${overflow - MAX_MENTION_EVENTS}`);
    expect(result.current[MAX_MENTION_EVENTS - 1].event_id).toBe(`m${overflow - 1}`);
  });
});
