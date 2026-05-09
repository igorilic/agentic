import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import PipelinePresetDropdown from "../components/PipelinePresetDropdown";

function makeWirePreset(id: string, name: string, agents: string[]) {
  return { id, name, agents, created_at: 1000, updated_at: 2000 };
}

describe("PipelinePresetDropdown", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  // ── Label display ────────────────────────────────────────────────────────────

  it('renders "Preset: (unsaved) ▾" when current pipelineAgents matches no preset', async () => {
    invokeMock.mockResolvedValueOnce([
      makeWirePreset("p1", "My Preset", ["architect", "qa"]),
    ]);

    render(
      <PipelinePresetDropdown
        pipelineAgents={["reviewer"]}
        onLoadPreset={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("preset-dropdown-toggle")).toHaveTextContent("(unsaved)");
    });
  });

  it("renders preset name when current pipelineAgents deep-equals an existing preset", async () => {
    invokeMock.mockResolvedValueOnce([
      makeWirePreset("p1", "My Preset", ["architect", "qa"]),
    ]);

    render(
      <PipelinePresetDropdown
        pipelineAgents={["architect", "qa"]}
        onLoadPreset={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("preset-dropdown-toggle")).toHaveTextContent("My Preset");
    });
  });

  // ── Popover opening ──────────────────────────────────────────────────────────

  it("clicking the button opens the popover with the preset list", async () => {
    invokeMock.mockResolvedValueOnce([
      makeWirePreset("p1", "Alpha", ["architect"]),
      makeWirePreset("p2", "Beta", ["qa", "reviewer"]),
    ]);

    render(
      <PipelinePresetDropdown pipelineAgents={[]} onLoadPreset={vi.fn()} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));

    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));

    expect(screen.getByTestId("preset-popover")).toBeInTheDocument();
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  // ── Load preset ──────────────────────────────────────────────────────────────

  it("clicking a preset row calls onLoadPreset with that preset's agents and closes the popover", async () => {
    invokeMock.mockResolvedValueOnce([
      makeWirePreset("p1", "Alpha", ["architect", "qa"]),
    ]);

    const onLoadPreset = vi.fn();
    render(
      <PipelinePresetDropdown pipelineAgents={[]} onLoadPreset={onLoadPreset} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));

    fireEvent.click(screen.getByTestId("preset-load-p1"));

    expect(onLoadPreset).toHaveBeenCalledWith(["architect", "qa"]);
    expect(screen.queryByTestId("preset-popover")).not.toBeInTheDocument();
  });

  // ── Save flow ────────────────────────────────────────────────────────────────

  it('"Save current as preset…" opens a modal with a name input', async () => {
    invokeMock.mockResolvedValueOnce([]);

    render(
      <PipelinePresetDropdown
        pipelineAgents={["architect"]}
        onLoadPreset={vi.fn()}
      />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));
    fireEvent.click(screen.getByTestId("preset-save-new"));

    expect(screen.getByTestId("preset-name-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("preset-name-input")).toBeInTheDocument();
  });

  it("submitting the save modal calls save_pipeline_preset with current pipelineAgents and refreshes", async () => {
    // initial list
    invokeMock.mockResolvedValueOnce([]);
    // save call
    invokeMock.mockResolvedValueOnce(makeWirePreset("p-new", "Draft", ["architect"]));
    // refetch
    invokeMock.mockResolvedValueOnce([makeWirePreset("p-new", "Draft", ["architect"])]);

    const user = userEvent.setup();
    render(
      <PipelinePresetDropdown
        pipelineAgents={["architect"]}
        onLoadPreset={vi.fn()}
      />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));
    fireEvent.click(screen.getByTestId("preset-save-new"));

    const input = screen.getByTestId("preset-name-input");
    await user.clear(input);
    await user.type(input, "Draft");

    fireEvent.click(screen.getByTestId("preset-name-submit"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("save_pipeline_preset", {
        name: "Draft",
        agents: ["architect"],
      });
    });

    // modal closed
    expect(screen.queryByTestId("preset-name-dialog")).not.toBeInTheDocument();
  });

  it("the save action is disabled when pipelineAgents is empty", async () => {
    invokeMock.mockResolvedValueOnce([]);

    render(
      <PipelinePresetDropdown pipelineAgents={[]} onLoadPreset={vi.fn()} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));

    expect(screen.getByTestId("preset-save-new")).toBeDisabled();
  });

  // ── Delete flow ──────────────────────────────────────────────────────────────

  it("kebab → Delete opens a confirm modal mentioning the preset name", async () => {
    invokeMock.mockResolvedValueOnce([
      makeWirePreset("p1", "Alpha", ["architect"]),
    ]);

    render(
      <PipelinePresetDropdown pipelineAgents={[]} onLoadPreset={vi.fn()} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));
    fireEvent.click(screen.getByTestId("preset-kebab-p1"));
    fireEvent.click(screen.getByTestId("preset-kebab-delete-p1"));

    expect(screen.getByTestId("preset-confirm-delete-dialog")).toBeInTheDocument();
    expect(screen.getByTestId("preset-confirm-delete-dialog")).toHaveTextContent("Alpha");
  });

  it("confirming delete calls delete_pipeline_preset with the preset id and closes the modal", async () => {
    invokeMock.mockResolvedValueOnce([makeWirePreset("p1", "Alpha", ["architect"])]);
    invokeMock.mockResolvedValueOnce(undefined); // delete
    invokeMock.mockResolvedValueOnce([]); // refetch

    render(
      <PipelinePresetDropdown pipelineAgents={[]} onLoadPreset={vi.fn()} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));
    fireEvent.click(screen.getByTestId("preset-kebab-p1"));
    fireEvent.click(screen.getByTestId("preset-kebab-delete-p1"));
    fireEvent.click(screen.getByTestId("preset-confirm-delete-confirm"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("delete_pipeline_preset", { id: "p1" });
    });

    expect(screen.queryByTestId("preset-confirm-delete-dialog")).not.toBeInTheDocument();
  });

  // ── Rename flow ──────────────────────────────────────────────────────────────

  it("kebab → Rename opens the modal pre-filled and Save calls update_pipeline_preset", async () => {
    invokeMock.mockResolvedValueOnce([makeWirePreset("p1", "Alpha", ["architect"])]);
    invokeMock.mockResolvedValueOnce(makeWirePreset("p1", "Beta", ["architect"]));
    invokeMock.mockResolvedValueOnce([makeWirePreset("p1", "Beta", ["architect"])]);

    const user = userEvent.setup();
    render(
      <PipelinePresetDropdown pipelineAgents={["architect"]} onLoadPreset={vi.fn()} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));
    fireEvent.click(screen.getByTestId("preset-kebab-p1"));
    fireEvent.click(screen.getByTestId("preset-kebab-rename-p1"));

    expect(screen.getByTestId("preset-name-dialog")).toBeInTheDocument();
    const input = screen.getByTestId("preset-name-input") as HTMLInputElement;
    expect(input.value).toBe("Alpha"); // pre-filled

    await user.clear(input);
    await user.type(input, "Beta");
    fireEvent.click(screen.getByTestId("preset-name-submit"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_pipeline_preset", {
        id: "p1",
        name: "Beta",
        agents: ["architect"],
      });
    });
  });

  // ── Error display ────────────────────────────────────────────────────────────

  it("error from save shows inline error in the popover", async () => {
    invokeMock.mockResolvedValueOnce([]); // initial list
    invokeMock.mockRejectedValueOnce("save failed"); // save call

    const user = userEvent.setup();
    render(
      <PipelinePresetDropdown pipelineAgents={["architect"]} onLoadPreset={vi.fn()} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));
    fireEvent.click(screen.getByTestId("preset-save-new"));

    await user.type(screen.getByTestId("preset-name-input"), "Draft");
    fireEvent.click(screen.getByTestId("preset-name-submit"));

    await waitFor(() => {
      expect(screen.getByTestId("preset-error")).toBeInTheDocument();
    });
  });

  // ── Outside click ────────────────────────────────────────────────────────────

  it("popover closes on outside click", async () => {
    invokeMock.mockResolvedValueOnce([]);

    const user = userEvent.setup();
    render(
      <div>
        <div data-testid="outside">outside</div>
        <PipelinePresetDropdown pipelineAgents={[]} onLoadPreset={vi.fn()} />
      </div>,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    await user.click(screen.getByTestId("preset-dropdown-toggle"));
    expect(screen.getByTestId("preset-popover")).toBeInTheDocument();

    await user.click(screen.getByTestId("outside"));
    expect(screen.queryByTestId("preset-popover")).not.toBeInTheDocument();
  });

  // ── Empty state ──────────────────────────────────────────────────────────────

  it('popover shows "No presets yet" when list is empty', async () => {
    invokeMock.mockResolvedValueOnce([]);

    render(
      <PipelinePresetDropdown pipelineAgents={["architect"]} onLoadPreset={vi.fn()} />,
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets"));
    fireEvent.click(screen.getByTestId("preset-dropdown-toggle"));

    expect(screen.getByText("No presets yet")).toBeInTheDocument();
  });
});
