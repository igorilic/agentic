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
  error: "bg-red-50 border-red-200 text-red-700",
  warning: "bg-yellow-50 border-yellow-200 text-yellow-800",
  info: "bg-blue-50 border-blue-200 text-blue-800",
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
      className={`px-4 py-2 border-b text-sm flex items-start gap-3 ${STYLES[severity]}`}
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
