import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import DismissableBanner from "../components/DismissableBanner";

describe("DismissableBanner", () => {
  it("renders the message with role='alert' and the provided test id", () => {
    render(
      <DismissableBanner
        testId="findings-error-banner"
        severity="error"
        message="something went wrong"
      />,
    );
    const banner = screen.getByTestId("findings-error-banner");
    expect(banner).toBeInTheDocument();
    expect(banner).toHaveAttribute("role", "alert");
    expect(banner).toHaveTextContent(/something went wrong/i);
  });

  it("renders nothing when message is null or empty", () => {
    const { rerender } = render(
      <DismissableBanner
        testId="findings-error-banner"
        severity="error"
        message={null}
      />,
    );
    expect(screen.queryByTestId("findings-error-banner")).toBeNull();

    rerender(
      <DismissableBanner
        testId="findings-error-banner"
        severity="error"
        message=""
      />,
    );
    expect(screen.queryByTestId("findings-error-banner")).toBeNull();
  });

  it("calls onDismiss when the close button is clicked", async () => {
    const onDismiss = vi.fn();
    const user = userEvent.setup();
    render(
      <DismissableBanner
        testId="findings-error-banner"
        severity="error"
        message="boom"
        onDismiss={onDismiss}
      />,
    );

    await user.click(screen.getByTestId("findings-error-banner-dismiss"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("hides the dismiss button when no onDismiss handler is supplied", () => {
    render(
      <DismissableBanner
        testId="history-error-banner"
        severity="warning"
        message="couldn't replay history"
      />,
    );
    // Without onDismiss the banner is decorative — no close button.
    expect(screen.queryByTestId("history-error-banner-dismiss")).toBeNull();
  });

  it("applies new token classes for error severity", () => {
    render(<DismissableBanner testId="b" severity="error" message="x" />);
    const el = screen.getByTestId("b");
    expect(el.className).toContain("bg-red-500/10");
    expect(el.className).toContain("border-red-300");
    expect(el.className).toContain("text-red-700");
  });

  it("applies new token classes for warning severity", () => {
    render(<DismissableBanner testId="b" severity="warning" message="x" />);
    const el = screen.getByTestId("b");
    expect(el.className).toContain("bg-amber-500/10");
    expect(el.className).toContain("border-amber-300");
    expect(el.className).toContain("text-amber-700");
  });

  it("applies new token classes for info severity", () => {
    render(<DismissableBanner testId="b" severity="info" message="x" />);
    const el = screen.getByTestId("b");
    expect(el.className).toContain("bg-blue-500/10");
    expect(el.className).toContain("border-blue-300");
    expect(el.className).toContain("text-blue-700");
  });

  it("uses px-3 horizontal padding to align with sibling sections", () => {
    render(
      <DismissableBanner testId="b" severity="info" message="aligned" />,
    );
    expect(screen.getByTestId("b").className).toMatch(/\bpx-3\b/);
    expect(screen.getByTestId("b").className).not.toMatch(/\bpx-4\b/);
  });
});
