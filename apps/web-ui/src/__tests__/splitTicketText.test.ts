import { describe, expect, it } from "vitest";
import { splitTicketText } from "../utils/splitTicketText";

describe("splitTicketText", () => {
  it("returns full text as ticketLabel and undefined description when no dot", () => {
    expect(splitTicketText("create palindrome function")).toEqual({
      ticketLabel: "create palindrome function",
      description: undefined,
    });
  });

  it("returns trimmed label and undefined description when trailing dot produces empty suffix", () => {
    expect(splitTicketText("Add rate limiting.")).toEqual({
      ticketLabel: "Add rate limiting",
      description: undefined,
    });
  });

  it("splits on first dot — label is first sentence, description is the rest", () => {
    expect(splitTicketText("Add rate limiting. Pro tier issue.")).toEqual({
      ticketLabel: "Add rate limiting",
      description: "Pro tier issue.",
    });
  });

  it("only splits on the FIRST dot — rest including subsequent dots goes to description", () => {
    expect(splitTicketText("Multi. Sentences. Here.")).toEqual({
      ticketLabel: "Multi",
      description: "Sentences. Here.",
    });
  });

  it("returns empty ticketLabel and undefined description for empty string", () => {
    expect(splitTicketText("")).toEqual({
      ticketLabel: "",
      description: undefined,
    });
  });

  it("trims leading/trailing whitespace from both parts", () => {
    expect(splitTicketText("   leading whitespace.  trailing  ")).toEqual({
      ticketLabel: "leading whitespace",
      description: "trailing",
    });
  });

  it("falls back to full trimmed text as ticketLabel when dot is at index 0 (empty first sentence)", () => {
    // When the first sentence (before dot) is empty, the entire trimmed text
    // is used as ticketLabel and description is undefined.
    expect(splitTicketText(". starts with dot")).toEqual({
      ticketLabel: ". starts with dot",
      description: undefined,
    });
  });

  it("handles whitespace-only string as empty", () => {
    expect(splitTicketText("   ")).toEqual({
      ticketLabel: "",
      description: undefined,
    });
  });
});
