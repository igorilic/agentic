import { useState, useEffect, useRef } from "react";
import type { AgentStatus } from "../types/pipeline";
import AgentCard from "./AgentCard";
import Connector from "./Connector";
import AgentPicker from "./AgentPicker";

export type PipelineBarProps = {
  agents: string[];
  statuses: Record<string, AgentStatus>;
  activeIndex: number;
  onReorder?: (from: number, to: number) => void;
  onInsert?: (atIndex: number, agentId: string) => void;
  onRemove?: (atIndex: number) => void;
  onSkip?: (atIndex: number) => void;
};

function useDragReorder(onReorder?: (from: number, to: number) => void) {
  const [dragFromIndex, setDragFromIndex] = useState<number | null>(null);
  const [dropGapIndex, setDropGapIndex] = useState<number | null>(null);

  function onCardDragStart(index: number) {
    setDragFromIndex(index);
  }

  function onCardDragEnd() {
    setDragFromIndex(null);
    setDropGapIndex(null);
  }

  function getGapHandlers(rightIndex: number) {
    return {
      onDragOver(e: React.DragEvent) {
        e.preventDefault();
        setDropGapIndex(rightIndex);
      },
      onDragLeave() {
        setDropGapIndex((prev) => (prev === rightIndex ? null : prev));
      },
      onDrop() {
        if (dragFromIndex !== null) {
          const adjusted =
            dragFromIndex < rightIndex ? rightIndex - 1 : rightIndex;
          if (adjusted !== dragFromIndex) {
            onReorder?.(dragFromIndex, adjusted);
          }
        }
        setDragFromIndex(null);
        setDropGapIndex(null);
      },
    };
  }

  return { dragFromIndex, dropGapIndex, onCardDragStart, onCardDragEnd, getGapHandlers };
}

export default function PipelineBar({
  agents,
  statuses,
  onReorder,
  onInsert,
}: PipelineBarProps) {
  const [pickerOpenAt, setPickerOpenAt] = useState<number | "end" | null>(null);
  const pickerRef = useRef<HTMLDivElement | null>(null);

  const {
    dragFromIndex,
    dropGapIndex,
    onCardDragStart,
    onCardDragEnd,
    getGapHandlers,
  } = useDragReorder(onReorder);

  useEffect(() => {
    if (pickerOpenAt === null) return;
    const onMouseDown = (e: MouseEvent) => {
      if (pickerRef.current && !pickerRef.current.contains(e.target as Node)) {
        setPickerOpenAt(null);
      }
    };
    // Defer by one tick so the same mousedown that opens the picker
    // doesn't immediately close it.
    const timerId = setTimeout(() => {
      document.addEventListener("mousedown", onMouseDown);
    }, 0);
    return () => {
      clearTimeout(timerId);
      document.removeEventListener("mousedown", onMouseDown);
    };
  }, [pickerOpenAt]);

  function renderInterCardGap(gapIndex: number) {
    const gapHandlers = getGapHandlers(gapIndex);
    const isActive = dropGapIndex === gapIndex;
    return (
      <div
        data-testid={`pipeline-gap-${gapIndex}`}
        data-drop-active={isActive ? "true" : "false"}
        className="flex items-center gap-1"
        {...gapHandlers}
      >
        {isActive && (
          <div
            aria-hidden="true"
            className="w-0.5 h-11 bg-status-active rounded flex-shrink-0"
          />
        )}
        <button
          type="button"
          data-testid={`pipeline-insert-${gapIndex}`}
          aria-label={`Insert agent at position ${gapIndex}`}
          onClick={() => setPickerOpenAt(gapIndex)}
          className="opacity-0 hover:opacity-100 focus:opacity-100 transition-opacity h-4 w-4 rounded-full border border-border-strong text-fg-muted text-[11px] leading-none flex items-center justify-center"
        >
          +
        </button>
        <Connector active={false} />
      </div>
    );
  }

  function renderEndGap() {
    const gapIndex = agents.length;
    const gapHandlers = getGapHandlers(gapIndex);
    const isActive = dropGapIndex === gapIndex;
    return (
      <div
        data-testid={`pipeline-gap-${gapIndex}`}
        data-drop-active={isActive ? "true" : "false"}
        className="flex items-center"
        {...gapHandlers}
      >
        {isActive && (
          <div
            aria-hidden="true"
            className="w-0.5 h-11 bg-status-active rounded flex-shrink-0"
          />
        )}
      </div>
    );
  }

  return (
    <div
      data-testid="pipeline-bar"
      className="relative flex h-[84px] items-center gap-3 bg-bg-surface border-b border-border-soft px-[18px]"
    >
      {agents.map((agent, i) => (
        <div key={agent} className="contents">
          <AgentCard
            agent={agent}
            status={statuses[agent] ?? "queued"}
            draggable={true}
            dragging={dragFromIndex === i}
            onDragStart={() => onCardDragStart(i)}
            onDragEnd={() => onCardDragEnd()}
          />
          {i < agents.length - 1 && renderInterCardGap(i + 1)}
        </div>
      ))}
      {/* gap-N after the last card, before + Add agent */}
      {agents.length > 0 && renderEndGap()}
      <button
        type="button"
        data-testid="pipeline-add-agent"
        onClick={() => setPickerOpenAt("end")}
        className="rounded-md border border-dashed border-border-strong px-3 py-1.5 text-xs font-semibold text-fg-muted"
      >
        + Add agent
      </button>
      {pickerOpenAt !== null && (
        <div
          ref={pickerRef}
          className="absolute top-full left-0 mt-1 z-10"
        >
          <AgentPicker
            excludeIds={agents}
            onPick={(agentId) => {
              const resolvedIndex =
                pickerOpenAt === "end" ? agents.length : pickerOpenAt;
              onInsert?.(resolvedIndex, agentId);
              setPickerOpenAt(null);
            }}
            onClose={() => setPickerOpenAt(null)}
          />
        </div>
      )}
    </div>
  );
}
