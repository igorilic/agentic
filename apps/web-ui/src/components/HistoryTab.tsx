import PastRunsPane from "./PastRunsPane";

export type HistoryTabProps = {
  onSelectRun?: (runId: string) => void;
};

export default function HistoryTab({ onSelectRun }: HistoryTabProps) {
  return <PastRunsPane onSelectRun={onSelectRun} />;
}
