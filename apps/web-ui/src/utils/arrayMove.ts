/**
 * Pure array-mutation helpers used by the pipeline reorder / insert logic.
 * These are intentionally free of React dependencies so they can be tested
 * in isolation and reused in any future state-machine layer.
 */

/**
 * Return a new array with the element at `from` moved to `to`.
 * If either index is out of bounds, or `from === to`, returns a shallow clone
 * of the original (no mutation).
 */
export function reorderArray<T>(arr: readonly T[], from: number, to: number): T[] {
  if (from < 0 || from >= arr.length) return [...arr];
  if (to < 0 || to >= arr.length) return [...arr];
  if (from === to) return [...arr];
  const out = [...arr];
  const [removed] = out.splice(from, 1);
  out.splice(to, 0, removed!);
  return out;
}

/**
 * Return a new array with `item` inserted at position `at`.
 * The index is clamped to `[0, arr.length]` so out-of-bounds values are safe.
 */
export function insertAt<T>(arr: readonly T[], at: number, item: T): T[] {
  const clamped = Math.max(0, Math.min(at, arr.length));
  const out = [...arr];
  out.splice(clamped, 0, item);
  return out;
}
