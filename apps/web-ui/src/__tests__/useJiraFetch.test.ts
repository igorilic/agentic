import { renderHook, act } from "@testing-library/react";
import { useJiraFetch } from "../hooks/useJiraFetch";
import type { JiraTicketDto } from "../types/jira";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

describe("useJiraFetch", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("returns fetch, isLoading, error in initial state", () => {
    const { result } = renderHook(() => useJiraFetch());
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBe(null);
    expect(typeof result.current.fetch).toBe("function");
  });

  it("invokes fetch_jira_ticket with the key and returns the DTO", async () => {
    const dto: JiraTicketDto = { key: "PROJ-1", title: "T", body: "B", ac: null };
    invokeMock.mockResolvedValue(dto);

    const { result } = renderHook(() => useJiraFetch());

    let returned: JiraTicketDto | undefined;
    await act(async () => {
      returned = await result.current.fetch("PROJ-1");
    });

    expect(invokeMock).toHaveBeenCalledWith("fetch_jira_ticket", { key: "PROJ-1" });
    expect(returned).toEqual(dto);
    expect(result.current.error).toBe(null);
    expect(result.current.isLoading).toBe(false);
  });

  it("sets isLoading=true while in flight", async () => {
    let resolveInvoke!: (value: JiraTicketDto) => void;
    const deferred = new Promise<JiraTicketDto>((res) => {
      resolveInvoke = res;
    });
    invokeMock.mockReturnValue(deferred);

    const { result } = renderHook(() => useJiraFetch());

    // Start the fetch without awaiting
    let fetchPromise!: Promise<JiraTicketDto>;
    act(() => {
      fetchPromise = result.current.fetch("PROJ-1");
    });

    // isLoading should now be true
    expect(result.current.isLoading).toBe(true);

    // Resolve the deferred promise
    await act(async () => {
      resolveInvoke({ key: "PROJ-1", title: "T", body: "B", ac: null });
      await fetchPromise;
    });

    expect(result.current.isLoading).toBe(false);
  });

  it("captures error message on rejection", async () => {
    invokeMock.mockRejectedValue("missing environment variables: JIRA_URL");

    const { result } = renderHook(() => useJiraFetch());

    await act(async () => {
      await result.current.fetch("PROJ-1").catch(() => {});
    });

    expect(result.current.error).toBe("missing environment variables: JIRA_URL");
    expect(result.current.isLoading).toBe(false);
  });

  it("clears error on subsequent successful fetch", async () => {
    const dto: JiraTicketDto = { key: "PROJ-1", title: "T", body: "B", ac: null };

    // First call: reject
    invokeMock.mockRejectedValueOnce("missing environment variables: JIRA_URL");
    // Second call: resolve
    invokeMock.mockResolvedValueOnce(dto);

    const { result } = renderHook(() => useJiraFetch());

    // First fetch — error is set
    await act(async () => {
      await result.current.fetch("PROJ-1").catch(() => {});
    });
    expect(result.current.error).toBe("missing environment variables: JIRA_URL");

    // Second fetch — error is cleared
    await act(async () => {
      await result.current.fetch("PROJ-1");
    });
    expect(result.current.error).toBe(null);
  });

  it("DTO type round-trip: a returned DTO with all 4 fields matches the TS shape", async () => {
    const dto: JiraTicketDto = {
      key: "AB-42",
      title: "Full ticket",
      body: "All fields populated",
      ac: "Given X, when Y, then Z",
    };
    invokeMock.mockResolvedValue(dto);

    const { result } = renderHook(() => useJiraFetch());

    let returned: JiraTicketDto | undefined;
    await act(async () => {
      returned = await result.current.fetch("AB-42");
    });

    expect(returned).toMatchObject<JiraTicketDto>({
      key: "AB-42",
      title: "Full ticket",
      body: "All fields populated",
      ac: "Given X, when Y, then Z",
    });
  });
});
