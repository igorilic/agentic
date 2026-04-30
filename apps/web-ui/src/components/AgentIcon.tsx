import type { ReactNode } from "react";
import { AGENT_LIBRARY } from "../types/pipeline";

// Icons transcribed verbatim from design handoff agents.jsx lines 5-18.
// The map is Record<string, ReactNode> because several glyphs (eye, gauge,
// palette, database, a11y) use <g> containers with multiple child elements —
// they do not fit a Record<string, string> path-only shape.
const AGENT_ICONS: Record<string, ReactNode> = {
  blueprint: (<path d="M3 4h14v12H3zM3 8h14M7 4v12M11 12h2" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round" />),
  code:      (<path d="M7 6l-4 4 4 4M13 6l4 4-4 4M11 4l-2 12" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />),
  check:     (<path d="M3 10l2 5 4-9 5 7 3-6" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />),
  eye:       (<g stroke="currentColor" strokeWidth="1.4" fill="none"><path d="M2 10s3-5 8-5 8 5 8 5-3 5-8 5-8-5-8-5z" /><circle cx="10" cy="10" r="2.2" /></g>),
  book:      (<path d="M4 4h5a2 2 0 012 2v10a2 2 0 00-2-2H4zM16 4h-5a2 2 0 00-2 2v10a2 2 0 012-2h5z" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinejoin="round" />),
  shield:    (<path d="M10 2l6 2v5c0 4-3 7-6 9-3-2-6-5-6-9V4l6-2zm-2 7l1.5 1.5L13 7" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />),
  gauge:     (<g stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round"><path d="M3 13a7 7 0 0114 0" /><path d="M10 13l3-4" /><circle cx="10" cy="13" r="0.8" fill="currentColor" /></g>),
  doc:       (<path d="M5 3h7l3 3v11H5zM12 3v3h3M7 9h6M7 12h6M7 15h4" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinejoin="round" strokeLinecap="round" />),
  palette:   (<g stroke="currentColor" strokeWidth="1.4" fill="none"><path d="M10 2a8 8 0 100 16c1.5 0 2-1.5 1-2.5s-.5-2.5 1-2.5h2a4 4 0 004-4A8 8 0 0010 2z" /><circle cx="6" cy="9" r="0.9" fill="currentColor" /><circle cx="9" cy="6" r="0.9" fill="currentColor" /><circle cx="13" cy="7" r="0.9" fill="currentColor" /></g>),
  database:  (<g stroke="currentColor" strokeWidth="1.4" fill="none"><ellipse cx="10" cy="5" rx="6" ry="2" /><path d="M4 5v10c0 1.1 2.7 2 6 2s6-.9 6-2V5M4 10c0 1.1 2.7 2 6 2s6-.9 6-2" /></g>),
  cloud:     (<path d="M6 14a3.5 3.5 0 010-7 4 4 0 017.5 1A3 3 0 0114 14H6z" stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinejoin="round" />),
  a11y:      (<g stroke="currentColor" strokeWidth="1.4" fill="none" strokeLinecap="round"><circle cx="10" cy="4" r="1.5" fill="currentColor" stroke="none" /><path d="M4 8h12M10 8v3m-3 5l3-5 3 5" /></g>),
};

const FALLBACK_GLYPH: ReactNode = (
  <rect x="4" y="4" width="12" height="12" rx="2" stroke="currentColor" strokeWidth="1.4" fill="none" />
);

export type AgentIconProps = {
  agent: string;
  size?: number;
};

export default function AgentIcon({ agent, size = 18 }: AgentIconProps) {
  const lib = AGENT_LIBRARY.find((a) => a.id === agent);
  const iconKey = lib?.icon;
  const inner =
    iconKey !== undefined && iconKey in AGENT_ICONS
      ? AGENT_ICONS[iconKey]
      : FALLBACK_GLYPH;
  return (
    <svg
      data-testid={`agent-icon-${agent}`}
      viewBox="0 0 20 20"
      width={size}
      height={size}
      aria-hidden="true"
    >
      {inner}
    </svg>
  );
}

export { AGENT_ICONS };
