export type ConnectorProps = {
  active: boolean;
};

export default function Connector({ active }: ConnectorProps) {
  return (
    <div
      data-testid="connector"
      data-active={active ? "true" : "false"}
      className="flex items-center gap-1"
      aria-hidden="true"
    >
      <span
        className={
          active
            ? "h-px w-8 border-t border-dashed border-zinc-300 animate-pulse"
            : "h-px w-8 border-t border-zinc-300"
        }
      />
      <svg
        data-testid="connector-chevron"
        viewBox="0 0 16 16"
        className="h-3 w-3 text-zinc-300"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M6 4l4 4-4 4" />
      </svg>
    </div>
  );
}
