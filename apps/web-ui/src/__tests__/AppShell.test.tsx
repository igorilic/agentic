import AppShell from "../components/AppShell";
import { render, screen } from "@testing-library/react";

describe("AppShell", () => {
  it("renders three column children", () => {
    render(
      <AppShell dense={false}>
        <div data-testid="col-1" />
        <div data-testid="col-2" />
        <div data-testid="col-3" />
      </AppShell>
    );
    expect(screen.getByTestId("col-1")).toBeInTheDocument();
    expect(screen.getByTestId("col-2")).toBeInTheDocument();
    expect(screen.getByTestId("col-3")).toBeInTheDocument();
  });

  it("renders the grid container with class grid and grid-cols-[1fr_1fr_340px] when dense=false", () => {
    render(
      <AppShell dense={false}>
        <div data-testid="col-1" />
        <div data-testid="col-2" />
        <div data-testid="col-3" />
      </AppShell>
    );
    const grid = screen.getByTestId("app-shell-grid");
    expect(grid).toBeInTheDocument();
    expect(grid.className).toContain("grid");
    expect(grid.className).toContain("grid-cols-[1fr_1fr_340px]");
  });

  it("renders grid-cols-[1fr_1fr_280px] when dense=true and does NOT contain 1fr_1fr_340px", () => {
    render(
      <AppShell dense={true}>
        <div data-testid="col-1" />
        <div data-testid="col-2" />
        <div data-testid="col-3" />
      </AppShell>
    );
    const grid = screen.getByTestId("app-shell-grid");
    expect(grid.className).toContain("grid-cols-[1fr_1fr_280px]");
    expect(grid.className).not.toContain("grid-cols-[1fr_1fr_340px]");
  });

  it("renders grid-cols-[1fr_1fr_340px] when dense=false explicitly", () => {
    render(
      <AppShell dense={false}>
        <div />
      </AppShell>
    );
    const grid = screen.getByTestId("app-shell-grid");
    expect(grid.className).toContain("grid-cols-[1fr_1fr_340px]");
  });

  it("renders grid-cols-[1fr_1fr_340px] when dense prop is omitted (default non-dense)", () => {
    render(
      <AppShell>
        <div />
      </AppShell>
    );
    const grid = screen.getByTestId("app-shell-grid");
    expect(grid.className).toContain("grid-cols-[1fr_1fr_340px]");
  });

  it("renders app-shell-header region even when header prop is not provided", () => {
    render(
      <AppShell>
        <div />
      </AppShell>
    );
    expect(screen.getByTestId("app-shell-header")).toBeInTheDocument();
  });

  it("renders app-shell-pipeline region even when pipelineBar prop is not provided", () => {
    render(
      <AppShell>
        <div />
      </AppShell>
    );
    expect(screen.getByTestId("app-shell-pipeline")).toBeInTheDocument();
  });

  it("renders custom header content inside app-shell-header", () => {
    render(
      <AppShell header={<div data-testid="my-header" />}>
        <div />
      </AppShell>
    );
    const headerRegion = screen.getByTestId("app-shell-header");
    expect(headerRegion).toContainElement(screen.getByTestId("my-header"));
  });

  it("renders custom pipelineBar content inside app-shell-pipeline", () => {
    render(
      <AppShell pipelineBar={<div data-testid="my-pipeline" />}>
        <div />
      </AppShell>
    );
    const pipelineRegion = screen.getByTestId("app-shell-pipeline");
    expect(pipelineRegion).toContainElement(screen.getByTestId("my-pipeline"));
  });

  it("does not crash when fewer than 3 children are passed and still renders the grid", () => {
    render(
      <AppShell>
        <div data-testid="only-child" />
      </AppShell>
    );
    expect(screen.getByTestId("app-shell-grid")).toBeInTheDocument();
    expect(screen.getByTestId("only-child")).toBeInTheDocument();
  });
});
