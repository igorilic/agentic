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
import { usePipelinePersistence } from "./hooks/usePipelinePersistence";
import { deriveRunState } from "./types/run";
import type { ActivityFilter } from "./components/ActivityHeader";
import type { IssueTicket } from "./types/pipeline";
import { findingsToActionItems } from "./utils/findingsToActionItems";
import { isTauriDense } from "./utils/isTauriDense";
import { usePipelineMutation } from "./hooks/usePipelineMutation";
import { useBackend } from "./hooks/useBackend";

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

  // Resolve workspace id once on mount — used as the localStorage key.
  const [wsId, setWsId] = useState<string | null>(null);
  useEffect(() => {
    invoke<string>("get_workspace_id")
      .then((id) => setWsId(id))
      .catch((err) => {
        console.error("[App] get_workspace_id failed — wsId will stay null, pipeline persistence disabled:", err);
      });
  }, []);

  const { events } = useTauriEvents(activeRunId);
  const { findings } = useFindings(findingsRunId, findingsRefetchKey);
  const { backend } = useBackend();

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

  const runState = useMemo(() => deriveRunState(events, activeRunId), [events, activeRunId]);

  // Scan from the tail to find the most recent envelope with a non-null step_id
  // that belongs to a StepStarted event.
  const latestStepId = useMemo(() => {
    for (let i = events.length - 1; i >= 0; i--) {
      const env = events[i];
      if (env.event.type === "StepStarted" && env.step_id) {
        return env.step_id;
      }
    }
    return undefined;
  }, [events]);

  const { overallRunState, elapsedMs } = useRunStateOverall(events, activeRunId);
  const { pipelineStatuses, activeIndex } = usePipelineFromRunState(runState);

  // Persistence — canonical source of truth for pipeline agents list.
  const { pipelineAgents, setPipelineAgents } = usePipelinePersistence(wsId);

  // Local mutation state (reorder, insert, remove, skip).
  // Persistence state is passed in so mutations write through to localStorage.
  const {
    pipelineSkipped,
    onReorder,
    onInsert,
    onRemove,
    onSkip,
  } = usePipelineMutation(runState, activeRunId, pipelineAgents, setPipelineAgents);

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
    void invoke("start_ticket_run", {
      ticket: "Untitled run",
      backend,
      model: null,
      agents: pipelineAgents,
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
  }, [backend, pipelineAgents]);

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
            hasAgents={pipelineAgents.length > 0}
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
          pipelineAgents={pipelineAgents}
        />
        <ActivityColumn
          events={events}
          filter={activityFilter}
          onFilterChange={setActivityFilter}
          runId={activeRunId}
          stepId={latestStepId}
        />
        <IssueColumn
          ticket={ticket}
          runState={overallRunState}
          actionItems={actionItems}
          onTicketRunStarted={handleTicketRunStarted}
          pipelineAgents={pipelineAgents}
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
