import { render, screen } from "@testing-library/react";
import IssueColumn from "../components/IssueColumn";
import type { ActionItem, IssueTicket } from "../types/pipeline";

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

  // W.6.3 — action items section

  const actionItemsFixture: ActionItem[] = [
    { id: "a1", kind: "issue",    title: "Document new headers in public API reference", description: "from docs", fromAgent: "docs"     },
    { id: "a2", kind: "warning",  title: "Reviewer flagged: lock contention under burst",                          fromAgent: "reviewer" },
    { id: "a3", kind: "followup", title: "Add Grafana alert: 429 rate spike per-tenant",  description: "from devops", fromAgent: "devops" },
  ];

  it("runState='completed' + 3 items: issue-action-items section is in document", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    expect(screen.getByTestId("issue-action-items")).toBeInTheDocument();
  });

  it("runState='completed' + 3 items: heading 'Action items' has uppercase and text-fg-muted classes", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    const heading = screen.getByRole("heading", { name: /action items/i });
    expect(heading.className).toContain("uppercase");
    expect(heading.className).toContain("text-fg-muted");
  });

  it("runState='completed' + 3 items: all 3 action item rows render with correct testids", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    expect(screen.getByTestId("action-item-a1")).toBeInTheDocument();
    expect(screen.getByTestId("action-item-a2")).toBeInTheDocument();
    expect(screen.getByTestId("action-item-a3")).toBeInTheDocument();
  });

  it("each action item row renders its title text", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    expect(screen.getByTestId("action-item-a1")).toHaveTextContent("Document new headers in public API reference");
    expect(screen.getByTestId("action-item-a2")).toHaveTextContent("Reviewer flagged: lock contention under burst");
    expect(screen.getByTestId("action-item-a3")).toHaveTextContent("Add Grafana alert: 429 rate spike per-tenant");
  });

  it("rows with description render the description text", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    expect(screen.getByTestId("action-item-a1")).toHaveTextContent("from docs");
    expect(screen.getByTestId("action-item-a3")).toHaveTextContent("from devops");
  });

  it("row without description (a2) does not render a description sub-element", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    const row = screen.getByTestId("action-item-a2");
    // No element with the description class inside this row
    expect(row.querySelectorAll(".text-fg-muted")).toHaveLength(0);
  });

  it("status icon a1 (issue) shows ✓", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    expect(screen.getByTestId("action-item-a1-icon")).toHaveTextContent("✓");
  });

  it("status icon a2 (warning) shows ⚠", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    expect(screen.getByTestId("action-item-a2-icon")).toHaveTextContent("⚠");
  });

  it("status icon a3 (followup) shows ↗", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    expect(screen.getByTestId("action-item-a3-icon")).toHaveTextContent("↗");
  });

  it("'Create spec' button renders with correct testid and text", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={actionItemsFixture} />);
    const btn = screen.getByTestId("issue-create-spec");
    expect(btn).toBeInTheDocument();
    expect(btn).toHaveTextContent(/create spec/i);
  });

  it("runState='completed' + empty actionItems: section is absent", () => {
    render(<IssueColumn ticket={fixture} runState="completed" actionItems={[]} />);
    expect(screen.queryByTestId("issue-action-items")).toBeNull();
  });

  it("runState='completed' + undefined actionItems: section is absent", () => {
    render(<IssueColumn ticket={fixture} runState="completed" />);
    expect(screen.queryByTestId("issue-action-items")).toBeNull();
  });

  it("runState='running' + 3 items: section is absent", () => {
    render(<IssueColumn ticket={fixture} runState="running" actionItems={actionItemsFixture} />);
    expect(screen.queryByTestId("issue-action-items")).toBeNull();
  });

  it("runState='failed' + 3 items: section is absent", () => {
    render(<IssueColumn ticket={fixture} runState="failed" actionItems={actionItemsFixture} />);
    expect(screen.queryByTestId("issue-action-items")).toBeNull();
  });

  it("runState='idle' + 3 items: section is absent", () => {
    render(<IssueColumn ticket={fixture} runState="idle" actionItems={actionItemsFixture} />);
    expect(screen.queryByTestId("issue-action-items")).toBeNull();
  });

  // W.9.7 — run-state pill (StatusDot) inline with issue id + section labels

  describe("W.9.7 run-state pill and section labels", () => {
    it("runState='running': status-dot is in document with text matching /Running/", () => {
      render(<IssueColumn ticket={fixture} runState="running" />);
      const dot = screen.getByTestId("status-dot");
      expect(dot).toBeInTheDocument();
      expect(dot).toHaveTextContent(/Running/);
    });

    it("runState='completed': status-dot text matches /Done/", () => {
      render(<IssueColumn ticket={fixture} runState="completed" />);
      const dot = screen.getByTestId("status-dot");
      expect(dot).toHaveTextContent(/Done/);
    });

    it("runState='idle': status-dot text matches /Queued/", () => {
      render(<IssueColumn ticket={fixture} runState="idle" />);
      const dot = screen.getByTestId("status-dot");
      expect(dot).toHaveTextContent(/Queued/);
    });

    it("runState='failed': status-dot text matches /Failed/", () => {
      render(<IssueColumn ticket={fixture} runState="failed" />);
      const dot = screen.getByTestId("status-dot");
      expect(dot).toHaveTextContent(/Failed/);
    });

    it("runState undefined: status-dot defaults to Queued", () => {
      render(<IssueColumn ticket={fixture} />);
      const dot = screen.getByTestId("status-dot");
      expect(dot).toHaveTextContent(/Queued/);
    });

    it("status-dot is inside the issue-column root (inline with issue id)", () => {
      render(<IssueColumn ticket={fixture} runState="running" />);
      const dot = screen.getByTestId("status-dot");
      const column = screen.getByTestId("issue-column");
      expect(column.contains(dot)).toBe(true);
    });

    it("Description label renders when body is non-empty", () => {
      render(<IssueColumn ticket={fixture} />);
      const label = screen.getByTestId("issue-section-description");
      expect(label).toBeInTheDocument();
      expect(label).toHaveTextContent("Description");
    });

    it("Description label is absent when body is empty", () => {
      render(<IssueColumn ticket={{ ...fixture, body: [] }} />);
      expect(screen.queryByTestId("issue-section-description")).toBeNull();
    });

    it("Acceptance criteria label renders when acceptance is non-empty", () => {
      render(<IssueColumn ticket={fixture} />);
      const label = screen.getByTestId("issue-section-acceptance");
      expect(label).toBeInTheDocument();
      expect(label).toHaveTextContent("Acceptance criteria");
    });

    it("Acceptance criteria label is absent when acceptance is empty", () => {
      render(<IssueColumn ticket={{ ...fixture, acceptance: [] }} />);
      expect(screen.queryByTestId("issue-section-acceptance")).toBeNull();
    });
  });

  // I.7 — pipelineAgents required + disabled state
  describe("I.7 — disabled Create-spec button when pipelineAgents is empty", () => {
    it("disables Create & run button when pipelineAgents is empty", () => {
      render(
        <IssueColumn
          ticket={fixture}
          runState="completed"
          actionItems={[
            { id: "a1", kind: "issue", title: "Something", fromAgent: "docs" },
          ]}
          pipelineAgents={[]}
        />,
      );
      const btn = screen.getByTestId("issue-create-spec");
      expect(btn).toBeDisabled();
      expect(btn).toHaveAttribute("title", "Pick agents in the pipeline rail first");
    });

    it("enables Create spec button when pipelineAgents has agents", () => {
      render(
        <IssueColumn
          ticket={fixture}
          runState="completed"
          actionItems={[
            { id: "a1", kind: "issue", title: "Something", fromAgent: "docs" },
          ]}
          pipelineAgents={["architect"]}
        />,
      );
      const btn = screen.getByTestId("issue-create-spec");
      expect(btn).not.toBeDisabled();
    });
  });
});
