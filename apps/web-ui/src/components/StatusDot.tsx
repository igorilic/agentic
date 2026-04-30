import type { AgentStatus } from "../types/pipeline";

export type StatusDotProps = {
  status: AgentStatus;
};

type DotStyle = {
  pillClass: string;
  dotClass: string;
  label: string;
};

const STATUS_STYLES: Record<AgentStatus, DotStyle> = {
  queued:  { pillClass: "bg-zinc-100 text-zinc-500",   dotClass: "bg-zinc-400",                label: "Queued"  },
  active:  { pillClass: "bg-blue-100 text-blue-700",   dotClass: "bg-blue-600 animate-pulse",  label: "Running" },
  done:    { pillClass: "bg-green-100 text-green-700", dotClass: "bg-green-600",               label: "Done"    },
  failed:  { pillClass: "bg-red-100 text-red-700",     dotClass: "bg-red-600",                 label: "Failed"  },
  errored: { pillClass: "bg-red-100 text-red-700",     dotClass: "bg-red-600",                 label: "Errored" },
  skipped: { pillClass: "bg-zinc-100 text-zinc-400",   dotClass: "bg-zinc-400 opacity-50",     label: "Skipped" },
};

export default function StatusDot({ status }: StatusDotProps) {
  const style = STATUS_STYLES[status];
  return (
    <span
      data-testid="status-dot"
      data-status={status}
      className={`inline-flex items-center gap-1 rounded-full px-1.5 py-0.5 text-[10px] font-medium ${style.pillClass}`}
    >
      <span
        data-testid="status-dot-marker"
        aria-hidden="true"
        className={`inline-block h-1.5 w-1.5 rounded-full ${style.dotClass}`}
      />
      {style.label}
    </span>
  );
}
