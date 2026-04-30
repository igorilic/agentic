import { useEffect, useRef, useState } from "react";
import { SLASH_COMMAND_LIBRARY } from "../slash/library";
import AgentPicker from "./AgentPicker";

function findMentionTrigger(value: string): { atIndex: number; query: string } | null {
  const lastAt = value.lastIndexOf("@");
  if (lastAt < 0) return null;
  // Must be at index 0 or preceded by whitespace.
  if (lastAt > 0 && !/\s/.test(value[lastAt - 1])) return null;
  // Chars after `@` up to next whitespace or end.
  const after = value.slice(lastAt + 1);
  if (/\s/.test(after)) return null; // trigger ended (space after the query)
  return { atIndex: lastAt, query: after };
}

export type ChatComposerProps = {
  onSend: (text: string) => void;
  /** Override the textarea's data-testid. Defaults to "chat-composer-textarea". */
  inputTestId?: string;
  /** Override the send button's data-testid. Defaults to "chat-composer-send". */
  sendTestId?: string;
};

const QUICK_PICK_CHIPS = [
  { id: "plan", label: "Plan", command: "/plan " },
  { id: "brainstorm", label: "Brainstorm", command: "/brainstorm " },
  { id: "develop", label: "Develop", command: "/develop " },
  { id: "spec", label: "Spec", command: "/spec " },
] as const;

export default function ChatComposer({ onSend, inputTestId, sendTestId }: ChatComposerProps) {
  const [value, setValue] = useState("");
  const [slashSelectedIndex, setSlashSelectedIndex] = useState(0);
  const [escClosedForValue, setEscClosedForValue] = useState<string | null>(null);
  const [mentionEscClosedForValue, setMentionEscClosedForValue] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  const slashOpen =
    /^\/[a-z]*$/.test(value) && escClosedForValue !== value;

  const mentionTriggerRaw = findMentionTrigger(value);
  const mentionTrigger =
    mentionTriggerRaw && mentionEscClosedForValue !== value
      ? mentionTriggerRaw
      : null;
  const mentionOpen = !slashOpen && mentionTrigger !== null;

  const slashMatches = slashOpen
    ? SLASH_COMMAND_LIBRARY.filter((cmd) => cmd.name.startsWith(value.slice(1)))
    : [];

  // Reset selected index and esc-closed marker when value changes
  useEffect(() => {
    setSlashSelectedIndex(0);
    if (escClosedForValue !== null && !/^\/[a-z]*$/.test(value)) {
      setEscClosedForValue(null);
    }
  }, [value]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleChipClick = (command: string) => {
    setValue(command);
    textareaRef.current?.focus();
  };

  const handleSend = () => {
    const text = value.trim();
    if (text === "") return;
    onSend(text);
    setValue("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (slashOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashSelectedIndex((i) => (i + 1) % Math.max(slashMatches.length, 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashSelectedIndex(
          (i) => (i - 1 + slashMatches.length) % Math.max(slashMatches.length, 1),
        );
        return;
      }
      if (e.key === "Enter" && !(e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        if (slashMatches.length > 0) {
          const selected = slashMatches[slashSelectedIndex] ?? slashMatches[0];
          setValue(`/${selected.name} `);
        }
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setEscClosedForValue(value);
        return;
      }
    }

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
            className="rounded-md border border-border px-2 py-1 text-xs text-fg hover:bg-bg-surface-2"
          >
            {chip.label}
          </button>
        ))}
      </div>
      <div className="relative flex items-end gap-2">
        {slashOpen && (
          <div
            data-testid="slash-popover"
            role="listbox"
            aria-label="Slash commands"
            className="absolute bottom-full mb-2 w-[280px] rounded-xl border border-[rgb(0_0_0_/_0.08)] bg-bg-surface shadow-popover"
          >
            {slashMatches.map((cmd, i) => (
              <div
                key={cmd.name}
                data-testid={`slash-popover-row-${cmd.name}`}
                role="option"
                aria-selected={i === slashSelectedIndex}
                className={`px-3 py-2 text-sm ${i === slashSelectedIndex ? "bg-bg-surface-2" : ""}`}
              >
                <div className="font-semibold text-fg">/{cmd.name}</div>
                <div className="text-xs text-fg-muted">{cmd.desc}</div>
              </div>
            ))}
          </div>
        )}
        {mentionOpen && mentionTrigger && (
          <div
            data-testid="mention-popover"
            className="absolute bottom-full mb-2 left-0"
          >
            <AgentPicker
              key={mentionTrigger.query}
              width="narrow"
              excludeIds={[]}
              initialQuery={mentionTrigger.query}
              onPick={(agentId) => {
                const before = value.slice(0, mentionTrigger.atIndex);
                setValue(`${before}@${agentId} `);
                setMentionEscClosedForValue(null);
                textareaRef.current?.focus();
              }}
              onClose={() => setMentionEscClosedForValue(value)}
            />
          </div>
        )}
        <textarea
          ref={textareaRef}
          data-testid={inputTestId ?? "chat-composer-textarea"}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 rounded-xl border border-[rgb(0_0_0_/_0.1)] px-[14px] py-[10px] text-sm font-sans focus:outline-none focus:ring-2 focus:ring-[#18181b] focus:ring-offset-2 resize-none"
          rows={1}
          placeholder="Type a message…"
        />
        <button
          type="button"
          data-testid={sendTestId ?? "chat-composer-send"}
          onClick={handleSend}
          aria-label="Send"
          className="h-9 w-9 rounded-none bg-[#18181b] text-white flex items-center justify-center"
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
