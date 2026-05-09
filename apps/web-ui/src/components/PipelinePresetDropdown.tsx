import { useCallback, useEffect, useRef, useState } from "react";
import Modal from "./Modal";
import { usePipelinePresets } from "../hooks/usePipelinePresets";
import type { PipelinePreset } from "../hooks/usePipelinePresets";

export type PipelinePresetDropdownProps = {
  pipelineAgents: string[];
  onLoadPreset: (agents: string[]) => void;
};

// ---------------------------------------------------------------------------
// Deep-equal helper for string arrays
// ---------------------------------------------------------------------------
function arraysEqual(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((v, i) => v === b[i]);
}

// ---------------------------------------------------------------------------
// PromptDialog — name input modal shared by Save and Rename
// ---------------------------------------------------------------------------
type PromptDialogProps = {
  open: boolean;
  title: string;
  initialValue: string;
  onSubmit: (name: string) => void;
  onCancel: () => void;
};

function PromptDialog({ open, title, initialValue, onSubmit, onCancel }: PromptDialogProps) {
  const [value, setValue] = useState(initialValue);

  // Sync when the dialog opens with a new initial value
  useEffect(() => {
    if (open) setValue(initialValue);
  }, [open, initialValue]);

  if (!open) return null;

  return (
    <Modal open={open} onClose={onCancel} ariaLabel={title} widthClass="w-[360px]">
      <div data-testid="preset-name-dialog" className="p-5 flex flex-col gap-4">
        <h3 className="text-[14px] font-semibold text-fg">{title}</h3>
        <input
          data-testid="preset-name-input"
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="Preset name"
          className="w-full rounded border border-border bg-bg-page px-3 py-1.5 text-sm text-fg placeholder:text-fg-muted focus:outline-none focus:ring-1 focus:ring-border-strong"
          autoFocus
          onKeyDown={(e) => {
            if (e.key === "Enter" && value.trim()) onSubmit(value.trim());
          }}
        />
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded px-3 py-1.5 text-xs text-fg-muted hover:text-fg"
          >
            Cancel
          </button>
          <button
            type="button"
            data-testid="preset-name-submit"
            disabled={!value.trim()}
            onClick={() => { if (value.trim()) onSubmit(value.trim()); }}
            className="rounded bg-fg px-3 py-1.5 text-xs font-semibold text-bg-page disabled:opacity-40"
          >
            Save
          </button>
        </div>
      </div>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// ConfirmDeleteDialog
// ---------------------------------------------------------------------------
type ConfirmDeleteDialogProps = {
  open: boolean;
  presetName: string;
  onConfirm: () => void;
  onCancel: () => void;
};

function ConfirmDeleteDialog({ open, presetName, onConfirm, onCancel }: ConfirmDeleteDialogProps) {
  if (!open) return null;
  return (
    <Modal open={open} onClose={onCancel} ariaLabel="Confirm delete" widthClass="w-[360px]">
      <div data-testid="preset-confirm-delete-dialog" className="p-5 flex flex-col gap-4">
        <p className="text-sm text-fg">
          Delete preset &ldquo;{presetName}&rdquo;?
        </p>
        <div className="flex justify-end gap-2">
          <button type="button" onClick={onCancel} className="rounded px-3 py-1.5 text-xs text-fg-muted hover:text-fg">
            Cancel
          </button>
          <button
            type="button"
            data-testid="preset-confirm-delete-confirm"
            onClick={onConfirm}
            className="rounded bg-red-600 px-3 py-1.5 text-xs font-semibold text-white"
          >
            Delete
          </button>
        </div>
      </div>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// KebabMenu — small inline action menu per preset row
// ---------------------------------------------------------------------------
type KebabMenuProps = {
  preset: PipelinePreset;
  onLoad: () => void;
  onRename: () => void;
  onDelete: () => void;
  onClose: () => void;
};

function KebabMenu({ preset, onLoad, onRename, onDelete, onClose }: KebabMenuProps) {
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const onMouseDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const timerId = setTimeout(() => {
      document.addEventListener("mousedown", onMouseDown);
    }, 0);
    return () => {
      clearTimeout(timerId);
      document.removeEventListener("mousedown", onMouseDown);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="absolute right-0 top-full mt-1 z-20 min-w-[120px] rounded border border-border bg-bg-surface shadow-modal py-1"
    >
      <button
        type="button"
        data-testid={`preset-kebab-load-${preset.id}`}
        onClick={onLoad}
        className="w-full text-left px-3 py-1.5 text-xs text-fg hover:bg-bg-page"
      >
        Load
      </button>
      <button
        type="button"
        data-testid={`preset-kebab-rename-${preset.id}`}
        onClick={onRename}
        className="w-full text-left px-3 py-1.5 text-xs text-fg hover:bg-bg-page"
      >
        Rename
      </button>
      <button
        type="button"
        data-testid={`preset-kebab-delete-${preset.id}`}
        onClick={onDelete}
        className="w-full text-left px-3 py-1.5 text-xs text-red-500 hover:bg-bg-page"
      >
        Delete
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------
type DialogState =
  | { type: "none" }
  | { type: "save" }
  | { type: "rename"; preset: PipelinePreset }
  | { type: "delete"; preset: PipelinePreset };

export default function PipelinePresetDropdown({
  pipelineAgents,
  onLoadPreset,
}: PipelinePresetDropdownProps) {
  const { presets, error, refresh, save, update, remove } = usePipelinePresets();

  const [open, setOpen] = useState(false);
  const [openKebab, setOpenKebab] = useState<string | null>(null);
  const [dialog, setDialog] = useState<DialogState>({ type: "none" });
  const [inlineError, setInlineError] = useState<string | null>(null);

  const popoverRef = useRef<HTMLDivElement | null>(null);

  // Derive active preset label
  const activePreset = presets.find((p) => arraysEqual(p.agents, pipelineAgents));
  const label = activePreset ? activePreset.name : "(unsaved)";

  // Outside-click dismissal for the popover.
  // Disabled when a dialog is open so clicks inside the modal don't close the popover.
  const dialogOpen = dialog.type !== "none";
  useEffect(() => {
    if (!open || dialogOpen) return;
    const onMouseDown = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setOpen(false);
        setOpenKebab(null);
      }
    };
    const timerId = setTimeout(() => {
      document.addEventListener("mousedown", onMouseDown);
    }, 0);
    return () => {
      clearTimeout(timerId);
      document.removeEventListener("mousedown", onMouseDown);
    };
  }, [open, dialogOpen]);

  // --- handlers ---

  const handleLoad = useCallback(
    (preset: PipelinePreset) => {
      onLoadPreset(preset.agents);
      setOpen(false);
      setOpenKebab(null);
    },
    [onLoadPreset],
  );

  const handleSaveNew = useCallback(() => {
    setOpenKebab(null);
    setDialog({ type: "save" });
  }, []);

  const handleSaveSubmit = useCallback(
    async (name: string) => {
      setInlineError(null);
      try {
        await save(name, pipelineAgents);
        setDialog({ type: "none" });
      } catch (e) {
        setInlineError(String(e));
        setDialog({ type: "none" });
      }
    },
    [save, pipelineAgents],
  );

  const handleRenameSubmit = useCallback(
    async (preset: PipelinePreset, newName: string) => {
      setInlineError(null);
      try {
        await update(preset.id, newName, preset.agents);
        setDialog({ type: "none" });
      } catch (e) {
        setInlineError(String(e));
        setDialog({ type: "none" });
      }
    },
    [update],
  );

  const handleDeleteConfirm = useCallback(
    async (preset: PipelinePreset) => {
      setInlineError(null);
      try {
        await remove(preset.id);
        setDialog({ type: "none" });
      } catch (e) {
        setInlineError(String(e));
        setDialog({ type: "none" });
      }
    },
    [remove],
  );

  const effectiveError = inlineError ?? (error ? error : null);

  return (
    <div className="relative flex items-center gap-2 px-[18px] py-1.5 border-b border-border-soft bg-bg-surface text-xs text-fg-muted">
      {/* Toggle button */}
      <button
        type="button"
        data-testid="preset-dropdown-toggle"
        onClick={() => {
          setOpen((v) => !v);
          setOpenKebab(null);
        }}
        className="rounded border border-border px-2.5 py-1 text-xs text-fg-muted hover:text-fg flex items-center gap-1"
      >
        Preset: {label} ▾
      </button>

      {/* Popover */}
      {open && (
        <div
          ref={popoverRef}
          data-testid="preset-popover"
          className="absolute top-full left-[18px] mt-1 z-10 min-w-[240px] rounded border border-border bg-bg-surface shadow-modal py-1"
        >
          {/* Inline error */}
          {effectiveError && (
            <div data-testid="preset-error" className="px-3 py-1.5 text-xs text-red-500">
              {effectiveError}
            </div>
          )}

          {/* Preset list */}
          {presets.length === 0 && (
            <div className="px-3 py-1.5 text-xs text-fg-muted">No presets yet</div>
          )}
          {presets.map((preset) => (
            <div key={preset.id} className="relative flex items-center">
              <button
                type="button"
                data-testid={`preset-load-${preset.id}`}
                onClick={() => handleLoad(preset)}
                className="flex-1 text-left px-3 py-1.5 text-xs text-fg hover:bg-bg-page"
              >
                {preset.name}
              </button>
              <button
                type="button"
                data-testid={`preset-kebab-${preset.id}`}
                onClick={() => setOpenKebab((v) => (v === preset.id ? null : preset.id))}
                className="px-2 py-1.5 text-fg-muted hover:text-fg"
                aria-label={`Actions for ${preset.name}`}
              >
                ⋮
              </button>
              {openKebab === preset.id && (
                <KebabMenu
                  preset={preset}
                  onLoad={() => handleLoad(preset)}
                  onRename={() => {
                    setOpenKebab(null);
                    setDialog({ type: "rename", preset });
                  }}
                  onDelete={() => {
                    setOpenKebab(null);
                    setDialog({ type: "delete", preset });
                  }}
                  onClose={() => setOpenKebab(null)}
                />
              )}
            </div>
          ))}

          {/* Divider + save */}
          {presets.length > 0 && <div className="my-1 border-t border-border" />}
          <button
            type="button"
            data-testid="preset-save-new"
            disabled={pipelineAgents.length === 0}
            onClick={handleSaveNew}
            className="w-full text-left px-3 py-1.5 text-xs text-fg hover:bg-bg-page disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Save current as preset…
          </button>
        </div>
      )}

      {/* Name dialog (save + rename share it) */}
      {(dialog.type === "save" || dialog.type === "rename") && (
        <PromptDialog
          open
          title={dialog.type === "save" ? "Save as preset" : "Rename preset"}
          initialValue={dialog.type === "rename" ? dialog.preset.name : ""}
          onSubmit={(name) => {
            if (dialog.type === "save") {
              void handleSaveSubmit(name);
            } else {
              void handleRenameSubmit(dialog.preset, name);
            }
          }}
          onCancel={() => setDialog({ type: "none" })}
        />
      )}

      {/* Delete confirm dialog */}
      {dialog.type === "delete" && (
        <ConfirmDeleteDialog
          open
          presetName={dialog.preset.name}
          onConfirm={() => void handleDeleteConfirm(dialog.preset)}
          onCancel={() => setDialog({ type: "none" })}
        />
      )}

      {/* Refresh button (hidden in production, useful for stale lists) */}
      <button
        type="button"
        onClick={() => void refresh()}
        className="sr-only"
        aria-label="Refresh presets"
      >
        Refresh
      </button>
    </div>
  );
}
