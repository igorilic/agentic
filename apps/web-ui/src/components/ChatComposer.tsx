import { useRef, useState } from "react";

export type ChatComposerProps = {
  onSend: (text: string) => void;
};

const QUICK_PICK_CHIPS = [
  { id: "plan", label: "Plan", command: "/plan " },
  { id: "brainstorm", label: "Brainstorm", command: "/brainstorm " },
  { id: "develop", label: "Develop", command: "/develop " },
  { id: "spec", label: "Spec", command: "/spec " },
] as const;

export default function ChatComposer({ onSend }: ChatComposerProps) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  const handleChipClick = (command: string) => {
    setValue(command);
    textareaRef.current?.focus();
  };

  const handleSend = () => {
    const text = value.trim();
    if (text === "") return;
    onSend(value);
    setValue("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSend();
    }
    // Enter alone: default behavior (newline insertion)
  };

  return (
    <div data-testid="chat-composer" className="flex flex-col gap-2 p-3">
      <div className="flex gap-2">
        {QUICK_PICK_CHIPS.map((chip) => (
          <button
            key={chip.id}
            type="button"
            data-testid={`chat-composer-chip-${chip.id}`}
            onClick={() => handleChipClick(chip.command)}
            className="rounded-md border border-border-strong px-2 py-1 text-xs text-fg hover:bg-bg-surface-2"
          >
            {chip.label}
          </button>
        ))}
      </div>
      <div className="flex items-end gap-2">
        <textarea
          ref={textareaRef}
          data-testid="chat-composer-textarea"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 rounded-xl border border-[rgb(0_0_0_/_0.1)] px-[14px] py-[10px] text-sm font-sans focus:outline-none focus:ring-2 focus:ring-[#18181b] focus:ring-offset-2 resize-none"
          rows={1}
          placeholder="Type a message…"
        />
        <button
          type="button"
          data-testid="chat-composer-send"
          onClick={handleSend}
          aria-label="Send"
          className="h-9 w-9 rounded-md bg-[#18181b] text-white flex items-center justify-center"
        >
          <svg
            viewBox="0 0 16 16"
            className="h-4 w-4"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M8 14V2 M3 7l5-5 5 5" />
          </svg>
        </button>
      </div>
    </div>
  );
}
