import type { RunStateOverall } from "../types/pipeline";
import { useTheme } from "../hooks/useTheme";
import { useBackend } from "../hooks/useBackend";
import type { BackendKind } from "../hooks/useBackend";

export function formatMmSs(ms: number): string {
  if (ms < 0) ms = 0;
  const totalSec = Math.floor(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

type RunStateBadgeProps = {
  runState: RunStateOverall;
  elapsedMs: number | null;
  hasAgents: boolean;
  onRunPipeline: () => void;
  onStopRun: () => void;
  onRerun: () => void;
};

function RunStateBadge({ runState, elapsedMs, hasAgents, onRunPipeline, onStopRun, onRerun }: RunStateBadgeProps) {
  if (runState === "idle") {
    const disabled = !hasAgents;
    return (
      <button
        type="button"
        data-testid="header-run"
        onClick={disabled ? undefined : onRunPipeline}
        disabled={disabled}
        title={disabled ? "Pick agents in the pipeline rail first" : undefined}
        className="rounded-md bg-[#18181b] px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-40 disabled:cursor-not-allowed"
      >
        Run pipeline
      </button>
    );
  }

  if (runState === "running" && elapsedMs !== null) {
    return (
      <div className="flex items-center gap-2.5">
        <div
          data-testid="header-running-pill"
          className="flex items-center gap-2 rounded-full bg-blue-100 px-3 py-1 text-xs font-semibold text-blue-700"
        >
          <span className="h-1.5 w-1.5 rounded-full bg-blue-700 animate-pulse" aria-hidden="true" />
          Pipeline running · {formatMmSs(elapsedMs)}
        </div>
        <button
          type="button"
          data-testid="header-stop"
          onClick={onStopRun}
          className="rounded-md border border-border-strong px-2.5 py-1 text-xs font-semibold text-fg"
        >
          Stop
        </button>
      </div>
    );
  }

  if (runState === "completed" && elapsedMs !== null) {
    return (
      <div className="flex items-center gap-2.5">
        <div
          data-testid="header-completed-pill"
          className="flex items-center gap-2 rounded-full bg-green-100 px-3 py-1 text-xs font-semibold text-green-700"
        >
          <span className="h-1.5 w-1.5 rounded-full bg-green-700" aria-hidden="true" />
          Completed · {formatMmSs(elapsedMs)}
        </div>
        <button
          type="button"
          data-testid="header-rerun"
          onClick={onRerun}
          className="rounded-md border border-border-strong px-2.5 py-1 text-xs font-semibold text-fg"
        >
          Re-run
        </button>
      </div>
    );
  }

  return null;
}

type SegmentedButtonProps = {
  label: string;
  testid: string;
  pressed: boolean;
  onClick: () => void;
};

function SegmentedButton({ label, testid, pressed, onClick }: SegmentedButtonProps) {
  return (
    <button
      type="button"
      data-testid={testid}
      aria-pressed={pressed}
      onClick={onClick}
      className={
        pressed
          ? "rounded px-2 py-0.5 text-xs font-medium bg-bg-surface-2 text-fg"
          : "rounded px-2 py-0.5 text-xs font-medium text-fg-muted hover:text-fg"
      }
    >
      {label}
    </button>
  );
}

type BackendSelectorProps = {
  backend: BackendKind;
  setBackend: (b: BackendKind) => void;
};

function BackendSelector({ backend, setBackend }: BackendSelectorProps) {
  return (
    <div
      role="group"
      aria-label="Backend"
      data-testid="header-backend-selector"
      className="flex rounded-md border border-border-soft p-0.5"
    >
      <SegmentedButton
        label="Claude"
        testid="header-backend-claude-code"
        pressed={backend === "claude-code"}
        onClick={() => setBackend("claude-code")}
      />
      <SegmentedButton
        label="Copilot"
        testid="header-backend-copilot-cli"
        pressed={backend === "copilot-cli"}
        onClick={() => setBackend("copilot-cli")}
      />
    </div>
  );
}

export type HeaderBarProps = {
  brand: string;
  ticketSlug: string | null;
  runState: RunStateOverall;
  elapsedMs: number | null;
  /** When false the "Run pipeline" button is disabled with a tooltip. Defaults to true. */
  hasAgents?: boolean;
  onOpenSettings: () => void;
  onRunPipeline: () => void;
  onStopRun: () => void;
  onRerun: () => void;
};

export default function HeaderBar({
  brand,
  ticketSlug,
  runState,
  elapsedMs,
  hasAgents = true,
  onOpenSettings,
  onRunPipeline,
  onStopRun,
  onRerun,
}: HeaderBarProps) {
  const { theme, toggle } = useTheme();
  const { backend, setBackend } = useBackend();
  return (
    <header
      data-testid="header-bar"
      className="flex h-12 items-center justify-between border-b border-border-soft bg-bg-surface px-[18px] gap-3.5 font-sans"
    >
      <div className="flex items-center gap-3">
        {/* Brand tile: 26x26, bg #18181b, rounded-md (6px), white SVG diamond */}
        <div className="flex h-[26px] w-[26px] items-center justify-center rounded-md bg-[#18181b]">
          <svg
            viewBox="0 0 20 20"
            className="h-3.5 w-3.5 text-white"
            fill="currentColor"
            aria-hidden="true"
          >
            <path d="M10 2l8 5v6l-8 5-8-5V7zM10 4.5L4.5 8 10 11.5 15.5 8z" />
          </svg>
        </div>
        <span className="text-sm font-semibold leading-none text-fg">{brand}</span>
        {ticketSlug !== null && (
          <span
            data-testid="header-slug"
            className="text-[11px] leading-none text-fg-subtle"
          >
            / {ticketSlug}
          </span>
        )}
      </div>

      <div className="flex items-center gap-3.5">
        <BackendSelector backend={backend} setBackend={setBackend} />
        <div role="status" aria-live="polite" data-testid="header-run-state">
          <RunStateBadge
            runState={runState}
            elapsedMs={elapsedMs}
            hasAgents={hasAgents}
            onRunPipeline={onRunPipeline}
            onStopRun={onStopRun}
            onRerun={onRerun}
          />
        </div>

        <button
          type="button"
          data-testid="header-settings"
          onClick={onOpenSettings}
          aria-label="Settings"
          className="flex h-7 w-7 items-center justify-center rounded text-fg-muted hover:text-fg"
        >
          <svg viewBox="0 0 20 20" className="h-[14px] w-[14px]" fill="currentColor" aria-hidden="true">
            <path d="M7.84 1.804A1 1 0 018.82 1h2.36a1 1 0 01.98.804l.331 1.652a6.993 6.993 0 011.929 1.115l1.598-.54a1 1 0 011.186.447l1.18 2.044a1 1 0 01-.205 1.251l-1.267 1.113a7.047 7.047 0 010 2.228l1.267 1.113a1 1 0 01.206 1.25l-1.18 2.045a1 1 0 01-1.187.447l-1.598-.54a6.993 6.993 0 01-1.929 1.115l-.33 1.652a1 1 0 01-.98.804H8.82a1 1 0 01-.98-.804l-.331-1.652a6.993 6.993 0 01-1.929-1.115l-1.598.54a1 1 0 01-1.186-.447l-1.18-2.044a1 1 0 01.205-1.251l1.267-1.114a7.05 7.05 0 010-2.227L1.821 7.773a1 1 0 01-.206-1.25l1.18-2.045a1 1 0 011.187-.447l1.598.54A6.992 6.992 0 017.51 3.456l.33-1.652zM10 13a3 3 0 100-6 3 3 0 000 6z" />
          </svg>
        </button>

        <button
          type="button"
          data-testid="header-theme-toggle"
          onClick={toggle}
          aria-pressed={theme === "dark"}
          aria-label="Toggle theme"
          className="flex h-7 w-7 items-center justify-center rounded text-fg-muted hover:text-fg"
        >
          {theme === "dark" ? (
            /* Moon icon */
            <svg
              viewBox="0 0 16 16"
              className="h-[14px] w-[14px]"
              fill="currentColor"
              aria-hidden="true"
            >
              <path d="M6 .278a.768.768 0 0 1 .08.858 7.208 7.208 0 0 0-.878 3.46c0 4.021 3.278 7.277 7.318 7.277.527 0 1.04-.055 1.533-.16a.787.787 0 0 1 .81.316.733.733 0 0 1-.031.893A8.349 8.349 0 0 1 8.344 16C3.734 16 0 12.286 0 7.71 0 4.266 2.114 1.312 5.124.06A.752.752 0 0 1 6 .278z" />
            </svg>
          ) : (
            /* Sun icon */
            <svg
              viewBox="0 0 16 16"
              className="h-[14px] w-[14px]"
              fill="currentColor"
              aria-hidden="true"
            >
              <path d="M8 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM8 0a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 0zm0 13a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-1 0v-2A.5.5 0 0 1 8 13zm8-5a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2a.5.5 0 0 1 .5.5zM3 8a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1 0-1h2A.5.5 0 0 1 3 8zm10.657-5.657a.5.5 0 0 1 0 .707l-1.414 1.415a.5.5 0 1 1-.707-.708l1.414-1.414a.5.5 0 0 1 .707 0zm-9.193 9.193a.5.5 0 0 1 0 .707L3.05 13.657a.5.5 0 0 1-.707-.707l1.414-1.414a.5.5 0 0 1 .707 0zm9.193 2.121a.5.5 0 0 1-.707 0l-1.414-1.414a.5.5 0 0 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .707zM4.464 4.465a.5.5 0 0 1-.707 0L2.343 3.05a.5.5 0 1 1 .707-.707l1.414 1.414a.5.5 0 0 1 0 .708z" />
            </svg>
          )}
        </button>

        <div
          data-testid="header-avatar"
          role="img"
          className="flex h-7 w-7 items-center justify-center rounded-full bg-zinc-200 text-[11px] font-semibold text-fg-muted"
          aria-label="User"
        >
          {""}
        </div>
      </div>
    </header>
  );
}
