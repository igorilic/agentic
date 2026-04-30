import type { ReactNode } from "react";

export type ModalProps = {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  ariaLabel: string;
  /** Override the default backdrop testid (`modal-backdrop`). */
  backdropTestId?: string;
  /** Override the default panel testid (`modal-panel`). */
  panelTestId?: string;
  /** Tailwind width class for the panel; default `w-[560px]`. */
  widthClass?: string;
};

export default function Modal({
  open,
  onClose,
  children,
  ariaLabel,
  backdropTestId,
  panelTestId,
  widthClass,
}: ModalProps) {
  if (!open) return null;

  return (
    <div
      data-testid={backdropTestId ?? "modal-backdrop"}
      onClick={onClose}
      className="fixed inset-0 z-30 bg-black/40 flex items-center justify-center"
    >
      <div
        data-testid={panelTestId ?? "modal-panel"}
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            onClose();
          }
        }}
        className={`${widthClass ?? "w-[560px]"} max-h-[80vh] overflow-y-auto rounded-[14px] border border-border bg-bg-surface shadow-modal`}
      >
        {children}
      </div>
    </div>
  );
}
