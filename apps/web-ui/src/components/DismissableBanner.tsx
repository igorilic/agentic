/**
 * Top-of-page error/warning banner shown for transient cross-pane failures
 * (history fetch, findings fetch, etc.). Optional dismiss button — when
 * `onDismiss` is supplied the banner shows a close affordance; otherwise
 * it stays put until the message becomes null.
 *
 * Renders nothing when `message` is `null` or empty so callers can pass an
 * error state directly without conditional wrappers.
 */

export type BannerSeverity = "error" | "warning" | "info";

export type DismissableBannerProps = {
  testId: string;
  severity: BannerSeverity;
  message: string | null;
  onDismiss?: () => void;
};

const STYLES: Record<BannerSeverity, string> = {
  error: "bg-red-500/10 border-red-300 text-red-700",
  warning: "bg-amber-500/10 border-amber-300 text-amber-700",
  info: "bg-blue-500/10 border-blue-300 text-blue-700",
};

export default function DismissableBanner({
  testId,
  severity,
  message,
  onDismiss,
}: DismissableBannerProps) {
  if (!message) return null;
  return (
    <div
      role="alert"
      data-testid={testId}
      className={`px-3 py-2 border-b text-sm flex items-start gap-3 ${STYLES[severity]}`}
    >
      <span className="flex-1">{message}</span>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          data-testid={`${testId}-dismiss`}
          aria-label="Dismiss"
          className="shrink-0 px-2 py-0.5 text-xs rounded border border-current opacity-70 hover:opacity-100"
        >
          ✕
        </button>
      )}
    </div>
  );
}
