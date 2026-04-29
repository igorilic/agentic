import type { ReactNode } from "react";

// Tailwind needs both literal class strings to extract them at build time:
//   grid-cols-[1fr_1fr_340px] grid-cols-[1fr_1fr_280px]

export type AppShellProps = {
  dense?: boolean;
  header?: ReactNode;
  pipelineBar?: ReactNode;
  children: ReactNode;
};

export default function AppShell({
  dense = false,
  header,
  pipelineBar,
  children,
}: AppShellProps) {
  const gridCols = dense
    ? "grid-cols-[1fr_1fr_280px]"
    : "grid-cols-[1fr_1fr_340px]";
  return (
    <div className="flex h-screen flex-col bg-bg-page">
      <div data-testid="app-shell-header">{header}</div>
      <div data-testid="app-shell-pipeline">{pipelineBar}</div>
      <div
        data-testid="app-shell-grid"
        className={`grid flex-1 min-h-0 ${gridCols}`}
      >
        {children}
      </div>
    </div>
  );
}
