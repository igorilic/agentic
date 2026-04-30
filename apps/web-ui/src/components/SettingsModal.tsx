import { useState } from "react";
import Modal from "./Modal";
import GeneralTab from "./GeneralTab";
import HistoryTab from "./HistoryTab";

export type SettingsModalProps = {
  open: boolean;
  initialTab?: "general" | "history";
  onClose: () => void;
  onSelectRun?: (runId: string) => void;
};

export default function SettingsModal({
  open,
  initialTab,
  onClose,
  onSelectRun,
}: SettingsModalProps) {
  const [activeTab, setActiveTab] = useState<"general" | "history">(
    initialTab ?? "general"
  );

  return (
    <Modal
      open={open}
      onClose={onClose}
      ariaLabel="Settings"
      backdropTestId="settings-modal-backdrop"
      panelTestId="settings-modal"
      widthClass="w-[720px]"
    >
      {/* Header row */}
      <div className="flex items-center justify-between px-5 pt-5 pb-0">
        <h2 className="text-[14px] font-semibold text-fg">Settings</h2>
        <button
          type="button"
          data-testid="settings-modal-close"
          onClick={onClose}
          className="text-fg-muted hover:text-fg text-lg leading-none"
          aria-label="Close"
        >
          ×
        </button>
      </div>

      {/* Tab strip */}
      <div
        role="tablist"
        className="flex gap-4 px-5 pt-3 pb-0 border-b border-border"
      >
        <button
          type="button"
          role="tab"
          data-testid="settings-tab-general"
          aria-selected={activeTab === "general"}
          onClick={() => setActiveTab("general")}
          className={`pb-2 text-[13px] font-medium border-b-2 ${
            activeTab === "general"
              ? "border-fg text-fg"
              : "border-transparent text-fg-muted"
          }`}
        >
          General
        </button>
        <button
          type="button"
          role="tab"
          data-testid="settings-tab-history"
          aria-selected={activeTab === "history"}
          onClick={() => setActiveTab("history")}
          className={`pb-2 text-[13px] font-medium border-b-2 ${
            activeTab === "history"
              ? "border-fg text-fg"
              : "border-transparent text-fg-muted"
          }`}
        >
          History
        </button>
      </div>

      {/* Tab body */}
      <div className="p-5">
        {activeTab === "general" ? (
          <GeneralTab />
        ) : (
          <HistoryTab onSelectRun={onSelectRun} />
        )}
      </div>
    </Modal>
  );
}
