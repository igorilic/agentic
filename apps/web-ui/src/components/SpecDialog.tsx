import { useState } from "react";

export type SpecDialogProps = {
  open: boolean;
  onClose: () => void;
  onSubmit: (title: string, body: string) => void;
};

export default function SpecDialog({ open, onClose, onSubmit }: SpecDialogProps) {
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");

  if (!open) return null;

  const submitDisabled = title.trim() === "";

  const handleSubmit = () => {
    if (submitDisabled) return;
    onSubmit(title, body);
  };

  return (
    <div
      data-testid="spec-dialog-backdrop"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          onClose();
        }
      }}
      className="fixed inset-0 z-30 bg-black/40 flex items-center justify-center"
    >
      <div
        data-testid="spec-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="New spec"
        onClick={(e) => e.stopPropagation()}
        className="w-[560px] max-h-[80vh] overflow-y-auto rounded-[14px] border border-border bg-bg-surface shadow-modal p-5 flex flex-col gap-4"
      >
        {/* Header */}
        <header className="flex items-start gap-3">
          <svg
            viewBox="0 0 16 16"
            className="h-5 w-5 text-fg-muted mt-0.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            aria-hidden="true"
          >
            <path d="M3 1.5h7l3 3v10H3z M10 1.5v3h3" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          <div className="flex flex-col gap-0.5">
            <h2 className="text-[14px] font-semibold text-fg">New spec</h2>
            <p className="text-[11px] text-fg-muted">Spec will be handed to the Architect.</p>
          </div>
        </header>

        {/* Body */}
        <div className="flex flex-col gap-3">
          <input
            type="text"
            data-testid="spec-dialog-title-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Title"
            autoFocus
            className="w-full rounded-md border border-border px-3 py-2 text-[13px] focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <textarea
            data-testid="spec-dialog-body-textarea"
            value={body}
            onChange={(e) => setBody(e.target.value)}
            placeholder="Description & acceptance criteria"
            rows={8}
            className="w-full rounded-md border border-border px-3 py-2 font-mono text-[12px] focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
          />
        </div>

        {/* Footer */}
        <footer className="flex items-center justify-end gap-2">
          <button
            type="button"
            data-testid="spec-dialog-cancel"
            onClick={onClose}
            className="rounded-md px-3 py-1.5 text-xs font-semibold text-fg hover:bg-bg-surface-2"
          >
            Cancel
          </button>
          <button
            type="button"
            data-testid="spec-dialog-submit"
            disabled={submitDisabled}
            onClick={handleSubmit}
            className="rounded-md bg-[#18181b] px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Create &amp; run
          </button>
        </footer>
      </div>
    </div>
  );
}
