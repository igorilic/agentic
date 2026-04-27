import { useMemo, useState } from "react";
import EventList from "./components/EventList";
import StartRunForm from "./components/StartRunForm";
import Stepper from "./components/Stepper";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { deriveRunState } from "./types/run";

export default function App() {
  const [activeRunId, setActiveRunId] = useState<string | undefined>(undefined);
  const { events, historyError } = useTauriEvents(activeRunId);

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
        <EventList events={events} />
      </section>
    </main>
  );
}
