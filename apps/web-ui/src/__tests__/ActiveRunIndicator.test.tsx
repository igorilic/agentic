import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ActiveRunIndicator from "../components/ActiveRunIndicator";

describe("ActiveRunIndicator", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders nothing when runId is null", () => {
    const { container } = render(
      <ActiveRunIndicator runId={null} startedAtMs={null} onCancel={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("shows the truncated run id and a Cancel button when active", () => {
    render(
      <ActiveRunIndicator
        runId="01k1234567890abcdefghij"
        startedAtMs={Date.now()}
        onCancel={vi.fn()}
      />,
    );
    const ind = screen.getByTestId("active-run-indicator");
    // Truncate to first 8 chars to keep the strip narrow.
    expect(ind).toHaveTextContent("01k12345");
    expect(screen.getByTestId("active-run-cancel")).toBeInTheDocument();
  });

  it("renders 'starting…' when startedAtMs is null", () => {
    render(
      <ActiveRunIndicator runId="01abc" startedAtMs={null} onCancel={vi.fn()} />,
    );
    expect(screen.getByTestId("active-run-indicator")).toHaveTextContent(
      /starting/i,
    );
  });

  it("updates the elapsed time at least once per second while running", () => {
    // vi.advanceTimersByTime advances both the timer queue and Date.now()
    // when fake timers are active, so capturing `start` then advancing is
    // enough. Adding setSystemTime on top would double-advance the clock.
    const start = Date.now();

    render(
      <ActiveRunIndicator runId="01abc" startedAtMs={start} onCancel={vi.fn()} />,
    );
    expect(screen.getByTestId("active-run-elapsed")).toHaveTextContent(/0s/);

    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(screen.getByTestId("active-run-elapsed")).toHaveTextContent(/5s/);

    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.getByTestId("active-run-elapsed")).toHaveTextContent(/1m/);
  });

  it("calls onCancel when the Cancel button is clicked", async () => {
    vi.useRealTimers(); // userEvent needs real timers
    const onCancel = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(
      <ActiveRunIndicator
        runId="01abc"
        startedAtMs={Date.now()}
        onCancel={onCancel}
      />,
    );

    await user.click(screen.getByTestId("active-run-cancel"));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("disables the Cancel button while cancellation is in flight", async () => {
    vi.useRealTimers();
    let resolveCancel: (() => void) | undefined;
    const onCancel = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveCancel = resolve;
        }),
    );
    const user = userEvent.setup();
    render(
      <ActiveRunIndicator
        runId="01abc"
        startedAtMs={Date.now()}
        onCancel={onCancel}
      />,
    );

    await user.click(screen.getByTestId("active-run-cancel"));
    await waitFor(() => {
      expect(screen.getByTestId("active-run-cancel")).toBeDisabled();
    });

    resolveCancel!();
    await waitFor(() => {
      expect(screen.getByTestId("active-run-cancel")).not.toBeDisabled();
    });
  });

  // Responsive layout assertions.
  it("indicator container has flex-wrap so content wraps at narrow widths", () => {
    render(
      <ActiveRunIndicator runId="01abc" startedAtMs={Date.now()} onCancel={vi.fn()} />,
    );
    const indicator = screen.getByTestId("active-run-indicator");
    expect(indicator.className).toMatch(/flex-wrap/);
  });
});
