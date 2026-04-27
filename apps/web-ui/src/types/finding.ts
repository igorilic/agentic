export type Triage = "fix" | "tech-debt" | "ignore";

export type Finding = {
  id: string;
  run_id: string;
  step_id: string;
  severity: string;
  file_path: string | null;
  line: number | null;
  message: string;
  suggestion: string | null;
  triage: Triage | null;
  triaged_at: number | null;
  created_at: number;
};
