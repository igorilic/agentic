import type { Finding } from "../types/finding";
import type { ActionItem } from "../types/pipeline";

const SEVERITY_TO_KIND: Record<string, ActionItem["kind"]> = {
  error: "warning",
  warning: "followup",
  info: "issue",
};

export function findingsToActionItems(findings: readonly Finding[]): ActionItem[] {
  const out: ActionItem[] = [];
  for (const f of findings) {
    if (f.triage !== null) continue;
    const kind = SEVERITY_TO_KIND[f.severity];
    if (kind === undefined) continue;
    const item: ActionItem = {
      id: f.id,
      kind,
      title: f.message,
      fromAgent: f.step_id,
    };
    if (f.suggestion !== null) item.description = f.suggestion;
    out.push(item);
  }
  return out;
}
