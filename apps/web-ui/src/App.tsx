import { useState } from "react";
import EventList from "./components/EventList";
import StartRunForm from "./components/StartRunForm";
import { useTauriEvents } from "./hooks/useTauriEvents";

export default function App() {
  const [activeRunId, setActiveRunId] = useState<string | undefined>(undefined);
  const events = useTauriEvents(activeRunId);

  return (
    <main className="min-h-screen bg-gray-50">
      <header className="px-6 py-4 border-b border-gray-200">
        <h1 className="text-2xl font-bold text-gray-900">Agentic</h1>
      </header>
      <StartRunForm
        events={events}
        activeRunId={activeRunId}
        onActiveRunIdChange={setActiveRunId}
      />
      <section className="p-6">
        <EventList events={events} />
      </section>
    </main>
  );
}
