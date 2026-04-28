export type RunSummary = {
  id: string;
  workspace_id: string;
  status: string;
  backend: string;
  model: string;
  ticket_label: string | null;
  started_at: number;
  completed_at: number | null;
  duration_ms: number | null;
};
