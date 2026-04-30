export type ActivityFilter = "all" | "tool" | "perm" | "error";

export type ActivityCounts = {
  all: number;
  tool: number;
  perm: number;
  error: number;
};

export type ActivityHeaderProps = {
  counts: ActivityCounts;
  filter: ActivityFilter;
  onFilterChange: (filter: ActivityFilter) => void;
};

const TABS: ReadonlyArray<{ key: ActivityFilter; label: string }> = [
  { key: "all", label: "All" },
  { key: "tool", label: "Tool calls" },
  { key: "perm", label: "Permissions" },
  { key: "error", label: "Errors" },
];

export default function ActivityHeader({ counts, filter, onFilterChange }: ActivityHeaderProps) {
  return (
    <div
      data-testid="activity-header"
      className="flex items-center justify-between px-4 py-3 border-b border-border-soft"
    >
      <h2 className="text-[13px] font-semibold text-fg">Activity</h2>
      <div role="tablist" className="flex items-center gap-3">
        {TABS.map((tab) => {
          const active = tab.key === filter;
          return (
            <button
              key={tab.key}
              type="button"
              role="tab"
              aria-selected={active}
              data-testid={`activity-tab-${tab.key}`}
              onClick={() => onFilterChange(tab.key)}
              className={`flex items-center gap-1 pb-1 text-[12px] font-medium ${
                active
                  ? "border-b-2 border-[#18181b] text-fg"
                  : "border-b-2 border-transparent text-fg-muted"
              }`}
            >
              <span>{tab.label}</span>
              <span
                data-testid={`activity-tab-${tab.key}-count`}
                className="inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-bg-surface-2 px-1 text-[10px] text-fg-muted"
              >
                {counts[tab.key]}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
