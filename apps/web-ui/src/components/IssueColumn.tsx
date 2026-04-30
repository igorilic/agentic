import type { ActionItem, IssueTicket, RunStateOverall } from "../types/pipeline";

export type IssueColumnProps = {
  ticket: IssueTicket;
  runState?: RunStateOverall;
  actionItems?: ActionItem[];
};

const ACTION_ITEM_ICON: Record<ActionItem["kind"], string> = {
  issue: "✓",
  warning: "⚠",
  followup: "↗",
};

export default function IssueColumn({ ticket, runState, actionItems }: IssueColumnProps) {
  const acceptanceChecked = runState === "completed";
  const showActionItems = runState === "completed" && (actionItems?.length ?? 0) > 0;

  return (
    <div
      data-testid="issue-column"
      className="flex h-full flex-col gap-3.5 overflow-y-auto bg-bg-surface p-[18px]"
    >
      {/* Header strip */}
      <div className="flex flex-col gap-1">
        <span
          data-testid="issue-id"
          className="text-[11px] font-bold text-fg-subtle"
        >
          {ticket.id}
        </span>
        <h1
          data-testid="issue-title"
          className="text-[15px] font-bold text-fg leading-tight"
        >
          {ticket.title}
        </h1>
      </div>

      {/* Labels row */}
      {ticket.labels.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {ticket.labels.map((label) => (
            <span
              key={label}
              data-testid={`issue-label-${label}`}
              className="rounded border border-border-strong px-1.5 py-0.5 text-[11px] text-fg-muted"
            >
              {label}
            </span>
          ))}
        </div>
      )}

      {/* Description body */}
      {ticket.body.length > 0 && (
        <div className="flex flex-col gap-2">
          {ticket.body.map((paragraph, i) => (
            <p
              key={i}
              data-testid="issue-body-paragraph"
              className="text-[13px] leading-[1.5] text-fg"
            >
              {paragraph}
            </p>
          ))}
        </div>
      )}

      {/* Acceptance checklist */}
      {ticket.acceptance.length > 0 && (
        <ul
          role="list"
          className="flex flex-col gap-1 font-mono text-[12px] text-fg"
        >
          {ticket.acceptance.map((item, i) => (
            <li
              key={i}
              data-testid="issue-acceptance-item"
              data-checked={acceptanceChecked ? "true" : "false"}
              className="flex items-start gap-2"
            >
              <span className="select-none">{acceptanceChecked ? "[x]" : "[ ]"}</span>
              <span>{item}</span>
            </li>
          ))}
        </ul>
      )}

      {/* Action items section — only when completed and non-empty */}
      {showActionItems && (
        <section data-testid="issue-action-items" className="flex flex-col gap-2">
          <h2 className="text-[12px] font-bold uppercase tracking-[0.05em] text-fg-muted">
            Action items
          </h2>
          <ul role="list" className="flex flex-col gap-2">
            {(actionItems ?? []).map((item) => (
              <li
                key={item.id}
                data-testid={`action-item-${item.id}`}
                className="flex items-start gap-2"
              >
                <span
                  data-testid={`action-item-${item.id}-icon`}
                  aria-hidden="true"
                  className="select-none"
                >
                  {ACTION_ITEM_ICON[item.kind]}
                </span>
                <div className="flex flex-col gap-0.5">
                  <span className="text-[13px] font-semibold text-fg">{item.title}</span>
                  {item.description !== undefined && (
                    <span className="text-[12px] text-fg-muted">{item.description}</span>
                  )}
                </div>
              </li>
            ))}
          </ul>
          <button
            type="button"
            data-testid="issue-create-spec"
            className="mt-2 self-start rounded-md bg-[#18181b] px-3 py-1.5 text-xs font-semibold text-white"
          >
            Create spec
          </button>
        </section>
      )}
    </div>
  );
}
