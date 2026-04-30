/**
 * Splits a /plan ticket text on the first period.
 *
 * The first sentence (up to and not including the first `.`) becomes
 * `ticketLabel`. The remainder (after the `.`, trimmed) becomes
 * `description`. If there is no `.`, or if the first sentence is empty
 * (dot at index 0), the full trimmed text is used as `ticketLabel` and
 * `description` is `undefined`.
 */
export function splitTicketText(text: string): {
  ticketLabel: string;
  description: string | undefined;
} {
  const trimmed = text.trim();
  const dotIdx = trimmed.indexOf(".");

  if (dotIdx === -1) {
    return { ticketLabel: trimmed, description: undefined };
  }

  const firstSentence = trimmed.slice(0, dotIdx).trim();

  // If the first sentence is empty (e.g. dot at position 0), fall back to
  // the full text so we don't silently drop the user's input.
  if (firstSentence.length === 0) {
    return { ticketLabel: trimmed, description: undefined };
  }

  const rest = trimmed.slice(dotIdx + 1).trim();
  return {
    ticketLabel: firstSentence,
    description: rest.length > 0 ? rest : undefined,
  };
}
