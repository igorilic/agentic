export type EventEnvelope = {
  schema_version: number;
  event_id: string;
  run_id: string;
  step_id: string | null;
  timestamp_ms: number;
  event: {
    type: string;
    data?: unknown;
  };
};
