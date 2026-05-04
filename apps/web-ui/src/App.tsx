import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import AppShell from "./components/AppShell";
import HeaderBar from "./components/HeaderBar";
import PipelineBar from "./components/PipelineBar";
import ChatPane from "./components/ChatPane";
import ActivityColumn from "./components/ActivityColumn";
import IssueColumn from "./components/IssueColumn";
import SettingsModal from "./components/SettingsModal";
import { useFindings } from "./hooks/useFindings";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useRunStateOverall } from "./hooks/useRunStateOverall";
import { usePipelineFromRunState } from "./hooks/usePipelineFromRunState";
import { deriveRunState } from "./types/run";
import type { ActivityFilter } from "./components/ActivityHeader";
import type { IssueTicket } from "./types/pipeline";
import { findingsToActionItems } from "./utils/findingsToActionItems";
import { isTauriDense } from "./utils/isTauriDense";
import { usePipelineMutation } from "./hooks/usePipelineMutation";

const PLACEHOLDER_TICKET: IssueTicket = {
  id: "AGT-000",
  title: "No active ticket",
  labels: [],
  body: ["No description available — ticket source integration ships in a future phase."],
  acceptance: [],
};

export default function App() {
  const [activeRunId, setActiveRunId] = useState<string | undefined>(undefined);
  const [activeTicketLabel, setActiveTicketLabel] = useState<string | undefined>(undefined);
  const [activeTicketDescription, setActiveTicketDescription] = useState<string | undefined>(undefined);
  const [findingsRunId, setFindingsRunId] = useState<string | undefined>(undefined);
  const [findingsRefetchKey, setFindingsRefetchKey] = useState(0);

  const { events } = useTauriEvents(activeRunId);
  const { findings } = useFindings(findingsRunId, findingsRefetchKey);

  useEffect(() => {
    if (activeRunId && activeRunId !== findingsRunId) setFindingsRunId(activeRunId);
  }, [activeRunId, findingsRunId]);

  useEffect(() => {
    if (!findingsRunId) return;
    const last = events[events.length - 1];
    if (last && last.event.type === "RunComplete" && last.run_id === findingsRunId) {
      setFindingsRefetchKey((n) => n + 1);
    }
  }, [events, findingsRunId]);

  const runState = useMemo(() => deriveRunState(events), [events]);

  const { overallRunState, elapsedMs } = useRunStateOverall(events, activeRunId);
  const { pipelineStatuses, activeIndex } = usePipelineFromRunState(runState);

  // Local-only pipeline state (spec §6.8.3). Re-seeds on activeRunId change.
  const {
    pipelineAgents,
    pipelineSkipped,
    onReorder,
    onInsert,
    onRemove,
    onSkip,
  } = usePipelineMutation(runState, activeRunId);

  const [settingsOpen, setSettingsOpen] = useState(false);

  const [activityFilter, setActivityFilter] = useState<ActivityFilter>("all");
  const actionItems = useMemo(() => findingsToActionItems(findings), [findings]);

  const cancelActiveRun = useCallback(async () => {
    if (!activeRunId) return;
    await invoke("cancel_run", { runId: activeRunId });
  }, [activeRunId]);

  const handleTicketRunStarted = useCallback((info: { runId: string; ticketLabel: string; description?: string }) => {
    setActiveRunId(info.runId);
    setActiveTicketLabel(info.ticketLabel);
    setActiveTicketDescription(info.description);
  }, []);

  const handleRunPipeline = useCallback(() => {
    // Minimal placeholder: invokes start_ticket_run directly.
    // A SpecDialog-driven Run flow is tracked for a future W.8.x step.
    void invoke("start_ticket_run", {
      ticket: "Untitled run",
      backend: "claude-code",
      model: null,
    })
      .then((result: unknown) => {
        if (typeof result === "string") {
          setActiveRunId(result);
          setActiveTicketLabel("Untitled run");
          setActiveTicketDescription(undefined);
        }
      })
      .catch(() => {
        /* no-op; failure surfaces via the run-state pill remaining idle */
      });
  }, []);

  const dense = isTauriDense();
  const ticket: IssueTicket = useMemo(() => {
    if (activeTicketLabel === undefined) return PLACEHOLDER_TICKET;
    const body =
      activeTicketDescription !== undefined
        ? activeTicketDescription
            .split(/\n\n+/)
            .map((p) => p.trim())
            .filter((p) => p.length > 0)
        : ["No description available — ticket source integration ships in a future phase."];
    return {
      id: "AGT-DEV",
      title: activeTicketLabel,
      labels: [],
      body,
      acceptance: [],
    };
  }, [activeTicketLabel, activeTicketDescription]);

  return (
    <>
      <AppShell
        dense={dense}
        header={
          <HeaderBar
            brand="Agentic"
            ticketSlug={activeRunId ? ticket.id : null}
            runState={overallRunState}
            elapsedMs={elapsedMs}
            onOpenSettings={() => setSettingsOpen(true)}
            onRunPipeline={handleRunPipeline}
            onStopRun={() => {
              void cancelActiveRun();
            }}
            onRerun={handleRunPipeline}
          />
        }
        pipelineBar={
          <PipelineBar
            agents={pipelineAgents}
            statuses={pipelineStatuses}
            activeIndex={activeIndex}
            skipped={pipelineSkipped}
            onReorder={onReorder}
            onInsert={onInsert}
            onRemove={onRemove}
            onSkip={onSkip}
          />
        }
      >
        <ChatPane
          onTicketRunStarted={handleTicketRunStarted}
        />
        <ActivityColumn
          events={events}
          filter={activityFilter}
          onFilterChange={setActivityFilter}
          runId={undefined} // TODO(P.4.3): thread activeRunId here
        />
        <IssueColumn
          ticket={ticket}
          runState={overallRunState}
          actionItems={actionItems}
          onTicketRunStarted={handleTicketRunStarted}
        />
      </AppShell>
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onSelectRun={setFindingsRunId}
      />
    </>
  );
}
