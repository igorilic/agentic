import { AGENT_LIBRARY } from "../types/pipeline";

// ── Types ──────────────────────────────────────────────────────────────────

export type LibraryResult = { kind: "library"; iconKey: string };
export type InitialResult = { kind: "initial"; initial: string; bgClass: string };
export type AgentIconResult = LibraryResult | InitialResult;

// ── Keyword table (priority: first matching entry wins) ───────────────────
//
// Priority rule: iterate tokens extracted from the agent name; for each
// token, scan the table top-to-bottom. The first table row whose keyword
// list contains any extracted token determines the icon.
//
// Consequence for multi-keyword names like 'perf_engineer':
//   tokens = ['perf', 'engineer']
//   'perf' hits the gauge row (row 7) before 'engineer' can hit code (row 2)
//   → gauge wins.
// Consequence for 'requirements-engineer':
//   tokens = ['requirements', 'engineer']
//   'requirements' hits book (row 5) before 'engineer' hits code (row 2)?
//   No — we scan all tokens for each table row in row order. Row 2 (code) is
//   checked before row 5 (book). 'requirements' doesn't match row 2; 'engineer'
//   DOES match row 2 → code... unless we want book to win.
//
// To make 'requirements-engineer' → book, the KEYWORD_TABLE must place the
// research/book row BEFORE the code/engineer row. The table below is ordered
// so that more-specific domain keywords come before the generic code row.

const KEYWORD_TABLE: [string[], string][] = [
  // architect / design — must be before code so 'architect' beats generic patterns
  [["architect", "design"], "blueprint"],
  // qa / test / quality — check
  [["qa", "test", "quality"], "check"],
  // review / reviewer — eye
  [["review", "reviewer"], "eye"],
  // research / requirements / analyst — book
  // placed BEFORE code so 'requirements-engineer' resolves to book
  [["research", "requirements", "analyst"], "book"],
  // security / audit / vuln — shield
  [["security", "audit", "vuln"], "shield"],
  // perf / performance / optimize — gauge
  // placed BEFORE code so 'perf_engineer' resolves to gauge
  [["perf", "performance", "optimize"], "gauge"],
  // doc / docs / writer / tech-writer — doc
  [["doc", "docs", "writer"], "doc"],
  // designer / ui / ux / figma — palette
  [["designer", "ui", "ux", "figma"], "palette"],
  // db / database / data / sql — database
  [["db", "database", "data", "sql"], "database"],
  // devops / infra / cloud / k8s / deploy — cloud
  [["devops", "infra", "cloud", "k8s", "deploy"], "cloud"],
  // a11y / accessibility — a11y
  [["a11y", "accessibility"], "a11y"],
  // develop / tdd / code / engineer / implement — code (generic, last)
  [["develop", "tdd", "code", "engineer", "implement"], "code"],
];

// ── Stable hash for initial fallback ─────────────────────────────────────

// 12 bg tailwind classes derived from the agentAccents palette.
// Literals kept verbatim so Tailwind JIT picks them up.
const INITIAL_BG_CLASSES = [
  "bg-sky-600",
  "bg-lime-700",
  "bg-fuchsia-700",
  "bg-amber-700",
  "bg-indigo-700",
  "bg-red-700",
  "bg-teal-700",
  "bg-zinc-600",
  "bg-pink-700",
  "bg-violet-700",
  "bg-cyan-700",
  "bg-emerald-700",
] as const;

function stableHash(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) {
    h = (h * 31 + name.charCodeAt(i)) & 0xffffffff;
  }
  return Math.abs(h);
}

// ── Core resolver ─────────────────────────────────────────────────────────

/**
 * Resolves an agent name to an icon result using 3-layer lookup:
 *
 * 1. Exact id match in AGENT_LIBRARY — preserves current known-agent behavior.
 * 2. Keyword heuristic — name normalized to lowercase tokens; KEYWORD_TABLE
 *    scanned top-to-bottom; first row whose keywords intersect the tokens wins.
 *    Priority is determined by table row order (higher rows win).
 * 3. Colored-initial fallback — first alphabetical character of name,
 *    uppercased; background class stable-hashed from the full name.
 */
export function resolveAgentIcon(name: string): AgentIconResult {
  // Layer 1 — exact library match
  const entry = AGENT_LIBRARY.find((a) => a.id === name);
  if (entry) {
    return { kind: "library", iconKey: entry.icon };
  }

  // Normalize: lowercase, split on non-alphanumeric runs
  const tokens = name.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);

  // Layer 2 — keyword heuristic (table order = priority)
  for (const [keywords, iconKey] of KEYWORD_TABLE) {
    for (const token of tokens) {
      if (keywords.includes(token)) {
        return { kind: "library", iconKey };
      }
    }
  }

  // Layer 3 — colored-initial fallback
  const firstAlpha = name.match(/[a-zA-Z]/)?.[0]?.toUpperCase() ?? "?";
  const idx = stableHash(name) % INITIAL_BG_CLASSES.length;
  const bgClass = INITIAL_BG_CLASSES[idx];
  return { kind: "initial", initial: firstAlpha, bgClass };
}
