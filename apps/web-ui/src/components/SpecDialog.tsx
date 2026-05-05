import { useState } from "react";
import Modal from "./Modal";
import { useJiraFetch } from "../hooks/useJiraFetch";

export type SpecDialogProps = {
  open: boolean;
  onClose: () => void;
  onSubmit: (title: string, body: string) => void | Promise<void>;
};

const KEY_REGEX = /^[A-Z][A-Z0-9]+-\d+$/;

export default function SpecDialog({ open, onClose, onSubmit }: SpecDialogProps) {
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [jiraKey, setJiraKey] = useState("");
  const [pullError, setPullError] = useState<string | null>(null);
  const [pulling, setPulling] = useState(false);

  const { fetch: fetchJira } = useJiraFetch();

  const keyValid = KEY_REGEX.test(jiraKey);
  const submitDisabled = title.trim() === "";

  const handleSubmit = () => {
    if (submitDisabled) return;
    void onSubmit(title, body);
  };

  const handlePull = () => {
    if (!keyValid || pulling) return;
    setPulling(true);
    setPullError(null);
    fetchJira(jiraKey)
      .then((dto) => {
        setTitle(dto.title);
        setBody(dto.body + (dto.ac ? "\n\n## Acceptance Criteria\n" + dto.ac : ""));
        setPullError(null);
      })
      .catch((e: unknown) => {
        setPullError(typeof e === "string" ? e : String(e));
      })
      .finally(() => {
        setPulling(false);
      });
  };

  const missingEnvError =
    pullError?.startsWith("missing environment variables") ? pullError : undefined;

  return (
    <Modal
      open={open}
      onClose={onClose}
      ariaLabel="New spec"
      backdropTestId="spec-dialog-backdrop"
      panelTestId="spec-dialog"
      widthClass="w-[560px]"
    >
      <div className="p-5 flex flex-col gap-4">
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
          {/* Jira pull row */}
          <div className="flex gap-2">
            <input
              type="text"
              data-testid="spec-dialog-jira-key-input"
              value={jiraKey}
              onChange={(e) => setJiraKey(e.target.value)}
              placeholder="PROJ-123"
              className="flex-1 rounded-md border border-border px-3 py-2 text-[13px] focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              type="button"
              data-testid="spec-dialog-jira-pull-button"
              disabled={!keyValid || pulling}
              title={missingEnvError}
              onClick={handlePull}
              className="rounded-md border border-border px-3 py-2 text-[13px] font-medium text-fg hover:bg-bg-surface-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Pull from Jira
            </button>
          </div>
          {pullError && (
            <p data-testid="spec-dialog-jira-pull-error" className="text-[11px] text-red-600">
              {pullError}
            </p>
          )}

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
    </Modal>
  );
}
