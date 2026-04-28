import { useEffect, useMemo, useState } from "react";
import ChatPane from "./components/ChatPane";
import EventList from "./components/EventList";
import FindingsTable from "./components/FindingsTable";
import SettingsPane from "./components/SettingsPane";
import StartRunForm from "./components/StartRunForm";
import Stepper from "./components/Stepper";
import { useFindings } from "./hooks/useFindings";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { deriveRunState } from "./types/run";

export default function App() {
  const [activeRunId, setActiveRunId] = useState<string | undefined>(undefined);
  // The run whose findings the cockpit should display. Pinned to the most
  // recent active run; survives RunComplete (which clears activeRunId in
  // StartRunForm) so the user can still triage findings from the run that
  // just ended.
  const [findingsRunId, setFindingsRunId] = useState<string | undefined>(undefined);
  // Bumped after RunComplete to force `useFindings` to refetch — the run
  // persists findings synchronously, so by RunComplete they are guaranteed
  // to be in the DB.
  const [findingsRefetchKey, setFindingsRefetchKey] = useState(0);

  const { events, historyError } = useTauriEvents(activeRunId);
  const { findings, error: findingsError } = useFindings(findingsRunId, findingsRefetchKey);

  useEffect(() => {
    if (activeRunId && activeRunId !== findingsRunId) {
      setFindingsRunId(activeRunId);
    }
  }, [activeRunId, findingsRunId]);

  useEffect(() => {
    if (!findingsRunId) return;
    const last = events[events.length - 1];
    if (last && last.event.type === "RunComplete" && last.run_id === findingsRunId) {
      setFindingsRefetchKey((n) => n + 1);
    }
  }, [events, findingsRunId]);

  const runState = useMemo(() => deriveRunState(events), [events]);

  return (
    <main className="min-h-screen bg-gray-50">
      <header className="px-6 py-4 border-b border-gray-200">
        <h1 className="text-2xl font-bold text-gray-900">Agentic</h1>
      </header>
      {historyError && (
        <div
          className="px-6 py-2 bg-yellow-50 border-b border-yellow-200 text-sm text-yellow-800"
          role="alert"
          data-testid="history-error-banner"
        >
          Could not load event history: {historyError}
        </div>
      )}
      <StartRunForm
        events={events}
        activeRunId={activeRunId}
        onActiveRunIdChange={setActiveRunId}
      />
      <Stepper state={runState} />
      <section className="p-6">
        <ChatPane />
      </section>
      <section className="px-6 pb-6">
        <EventList events={events} />
      </section>
      <section className="px-6 pb-6">
        {findingsError && (
          <div
            className="mb-2 px-3 py-2 bg-red-50 border border-red-200 rounded text-sm text-red-700"
            role="alert"
            data-testid="findings-error-banner"
          >
            Could not load findings: {findingsError}
          </div>
        )}
        <FindingsTable findings={findings} />
      </section>
      <section className="px-6 pb-6">
        <SettingsPane />
      </section>
    </main>
  );
}
