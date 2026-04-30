import { render, screen } from "@testing-library/react";
import IssueColumn from "../components/IssueColumn";
import type { IssueTicket } from "../types/pipeline";

const fixture: IssueTicket = {
  id: "AGT-204",
  title: "Add multi-tenant rate limiting to the public API",
  labels: ["backend", "api", "infra"],
  body: [
    "Customers on Pro tier hit noisy-neighbor issues.",
    "Add a token-bucket limiter keyed on tenant_id.",
  ],
  acceptance: [
    "Per-tenant token bucket persisted in Redis",
    "429 with Retry-After once empty",
  ],
};

describe("IssueColumn", () => {
  it("renders outer container with data-testid='issue-column'", () => {
    render(<IssueColumn ticket={fixture} />);
    expect(screen.getByTestId("issue-column")).toBeInTheDocument();
  });

  it("renders issue id with data-testid='issue-id' and styling classes", () => {
    render(<IssueColumn ticket={fixture} />);
    const idEl = screen.getByTestId("issue-id");
    expect(idEl).toHaveTextContent("AGT-204");
    expect(idEl.className).toContain("text-[11px]");
    expect(idEl.className).toContain("text-fg-subtle");
    expect(idEl.className).toMatch(/font-bold/);
  });

  it("renders issue title with data-testid='issue-title'", () => {
    render(<IssueColumn ticket={fixture} />);
    const titleEl = screen.getByTestId("issue-title");
    expect(titleEl).toHaveTextContent(
      "Add multi-tenant rate limiting to the public API"
    );
  });

  it("renders 3 label chips with correct testids", () => {
    render(<IssueColumn ticket={fixture} />);
    expect(screen.getByTestId("issue-label-backend")).toHaveTextContent("backend");
    expect(screen.getByTestId("issue-label-api")).toHaveTextContent("api");
    expect(screen.getByTestId("issue-label-infra")).toHaveTextContent("infra");
  });

  it("label chips have border and rounded classes", () => {
    render(<IssueColumn ticket={fixture} />);
    const chip = screen.getByTestId("issue-label-backend");
    expect(chip.className).toContain("border");
    expect(chip.className).toMatch(/rounded/);
  });

  it("renders 2 body paragraphs with data-testid='issue-body-paragraph'", () => {
    render(<IssueColumn ticket={fixture} />);
    const paragraphs = screen.getAllByTestId("issue-body-paragraph");
    expect(paragraphs).toHaveLength(2);
    expect(paragraphs[0]).toHaveTextContent(
      "Customers on Pro tier hit noisy-neighbor issues."
    );
    expect(paragraphs[1]).toHaveTextContent(
      "Add a token-bucket limiter keyed on tenant_id."
    );
  });

  it("renders acceptance checklist as <ul role='list'>", () => {
    render(<IssueColumn ticket={fixture} />);
    const list = screen.getByRole("list");
    expect(list.tagName).toBe("UL");
  });

  it("renders 2 acceptance items with data-testid='issue-acceptance-item'", () => {
    render(<IssueColumn ticket={fixture} />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent("Per-tenant token bucket persisted in Redis");
    expect(items[1]).toHaveTextContent("429 with Retry-After once empty");
  });

  it("acceptance items contain monospace '[ ]' marker", () => {
    render(<IssueColumn ticket={fixture} />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    items.forEach((item) => {
      expect(item).toHaveTextContent("[ ]");
    });
  });

  it("acceptance items default to data-checked='false'", () => {
    render(<IssueColumn ticket={fixture} />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    items.forEach((item) => {
      expect(item).toHaveAttribute("data-checked", "false");
    });
  });

  it("renders nothing for labels when labels array is empty", () => {
    render(<IssueColumn ticket={{ ...fixture, labels: [] }} />);
    expect(screen.queryAllByTestId(/^issue-label-/)).toHaveLength(0);
  });

  it("renders nothing for body paragraphs when body array is empty", () => {
    render(<IssueColumn ticket={{ ...fixture, body: [] }} />);
    expect(screen.queryAllByTestId("issue-body-paragraph")).toHaveLength(0);
    // Column still renders
    expect(screen.getByTestId("issue-column")).toBeInTheDocument();
  });

  it("renders nothing for acceptance items when acceptance array is empty", () => {
    render(<IssueColumn ticket={{ ...fixture, acceptance: [] }} />);
    expect(screen.queryAllByTestId("issue-acceptance-item")).toHaveLength(0);
  });

  // W.6.2 — runState prop drives acceptance checked state

  it("runState='completed': all items have data-checked='true' and '[x]' marker", () => {
    render(<IssueColumn ticket={fixture} runState="completed" />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    items.forEach((item) => {
      expect(item).toHaveAttribute("data-checked", "true");
      expect(item).toHaveTextContent("[x]");
    });
  });

  it("runState='running': all items have data-checked='false' and '[ ]' marker", () => {
    render(<IssueColumn ticket={fixture} runState="running" />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    items.forEach((item) => {
      expect(item).toHaveAttribute("data-checked", "false");
      expect(item).toHaveTextContent("[ ]");
    });
  });

  it("runState='failed': all items have data-checked='false' and '[ ]' marker", () => {
    render(<IssueColumn ticket={fixture} runState="failed" />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    items.forEach((item) => {
      expect(item).toHaveAttribute("data-checked", "false");
      expect(item).toHaveTextContent("[ ]");
    });
  });

  it("runState='idle': all items have data-checked='false' and '[ ]' marker", () => {
    render(<IssueColumn ticket={fixture} runState="idle" />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    items.forEach((item) => {
      expect(item).toHaveAttribute("data-checked", "false");
      expect(item).toHaveTextContent("[ ]");
    });
  });

  it("omitting runState prop is backward-compatible: items remain data-checked='false'", () => {
    render(<IssueColumn ticket={fixture} />);
    const items = screen.getAllByTestId("issue-acceptance-item");
    items.forEach((item) => {
      expect(item).toHaveAttribute("data-checked", "false");
    });
  });
});
