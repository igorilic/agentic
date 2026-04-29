import type { RunState, StepStatus } from "../types/run";

type StepperProps = {
  state: RunState;
};

const ICONS: Record<StepStatus, string> = {
  pending: "○",
  running: "◐",
  passed: "✓",
  failed: "✗",
  needs_triage: "⚠",
  skipped: "⊘",
};

const COLORS: Record<StepStatus, string> = {
  pending: "text-gray-400",
  running: "text-blue-600",
  passed: "text-green-600",
  failed: "text-red-600",
  needs_triage: "text-orange-600",
  skipped: "text-yellow-600",
};

export default function Stepper({ state }: StepperProps) {
  return (
    <section
      className="px-6 py-4 border-b border-gray-200"
      data-testid="cockpit-stepper"
      aria-label="Pipeline progress"
    >
      <ol className="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4">
        {state.steps.map((step, idx) => (
          <li
            key={step.agent}
            data-testid={`stepper-step-${step.agent}`}
            data-status={step.status}
            className="flex items-center gap-2"
          >
            <span
              className={`text-2xl ${COLORS[step.status]}`}
              aria-label={`${step.agent} ${step.status}`}
              data-testid={`stepper-icon-${step.agent}`}
            >
              {ICONS[step.status]}
            </span>
            <span className="text-sm font-medium text-gray-700">
              {step.agent}
            </span>
            {idx < state.steps.length - 1 && (
              <span className="hidden sm:inline text-gray-300" aria-hidden="true">
                →
              </span>
            )}
          </li>
        ))}
      </ol>
      <div
        className="mt-2 text-xs text-gray-500"
        data-testid="stepper-totals"
      >
        Total tokens: <span className="font-mono">{state.totalTokens}</span>
        {state.totalCostUsd > 0 && (
          <>
            {" · Cost: "}
            <span className="font-mono">${state.totalCostUsd.toFixed(4)}</span>
          </>
        )}
      </div>
    </section>
  );
}
