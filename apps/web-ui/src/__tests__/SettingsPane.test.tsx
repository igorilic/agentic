import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import SettingsPane from "../components/SettingsPane";
import type { AuthAccount } from "../types/auth";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function makeAccount(overrides: Partial<AuthAccount> = {}): AuthAccount {
  return {
    id: "github:github.com",
    provider: "github",
    host: "github.com",
    username: "octocat",
    client_id: "Iv1.abc",
    token_expires_at: null,
    created_at: 100,
    last_used_at: null,
    ...overrides,
  };
}

describe("SettingsPane", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("on mount, calls list_auth_accounts and renders the empty state when none exist", async () => {
    invokeMock.mockResolvedValueOnce([]);

    render(<SettingsPane />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("list_auth_accounts");
    });
    expect(screen.getByText(/no accounts/i)).toBeInTheDocument();
  });

  it("renders one row per account", async () => {
    invokeMock.mockResolvedValueOnce([
      makeAccount({ id: "github:github.com", provider: "github" }),
      makeAccount({ id: "gitlab:gitlab.com", provider: "gitlab", host: "gitlab.com" }),
    ]);

    render(<SettingsPane />);

    await waitFor(() => {
      expect(screen.getAllByTestId(/auth-account-row-/)).toHaveLength(2);
    });
    expect(screen.getByTestId("auth-account-row-github:github.com")).toHaveTextContent(
      "github.com",
    );
    expect(screen.getByTestId("auth-account-row-gitlab:gitlab.com")).toHaveTextContent(
      "gitlab.com",
    );
  });

  it("clicking the Connect button calls connect_github_via_gh (zero-config)", async () => {
    invokeMock
      .mockResolvedValueOnce([]) // list on mount
      .mockResolvedValueOnce(makeAccount()) // connect_github_via_gh
      .mockResolvedValueOnce([makeAccount()]); // list on refetch

    const user = userEvent.setup();
    render(<SettingsPane />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());

    await user.click(screen.getByTestId("connect-github-submit"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("connect_github_via_gh");
    });
    // After success, the account should appear in the list.
    await waitFor(() => {
      expect(
        screen.getByTestId("auth-account-row-github:github.com"),
      ).toBeInTheDocument();
    });
  });

  it("disables the connect button while a connection is in flight", async () => {
    invokeMock.mockResolvedValueOnce([]); // list on mount

    let resolveConnect: ((value: AuthAccount) => void) | undefined;
    invokeMock
      .mockImplementationOnce(
        () =>
          new Promise<AuthAccount>((resolve) => {
            resolveConnect = resolve;
          }),
      )
      .mockResolvedValueOnce([]); // post-connect refresh

    const user = userEvent.setup();
    render(<SettingsPane />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());

    await user.click(screen.getByTestId("connect-github-submit"));

    await waitFor(() => {
      expect(screen.getByTestId("connect-github-submit")).toBeDisabled();
    });
    expect(screen.getByTestId("connect-github-submit")).toHaveTextContent(
      /connecting/i,
    );

    act(() => {
      resolveConnect!(makeAccount());
    });

    // Drain pending promises so the test doesn't leak.
    await waitFor(() => {
      expect(screen.getByTestId("connect-github-submit")).not.toBeDisabled();
    });
  });

  it("clicking the disconnect button invokes delete_auth_account and removes the row", async () => {
    invokeMock
      .mockResolvedValueOnce([makeAccount({ id: "github:github.com" })])
      .mockResolvedValueOnce(true) // delete
      .mockResolvedValueOnce([]); // refetch

    const user = userEvent.setup();
    render(<SettingsPane />);
    await waitFor(() =>
      expect(screen.getByTestId("auth-account-row-github:github.com")).toBeInTheDocument(),
    );

    await user.click(screen.getByTestId("disconnect-github:github.com"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("delete_auth_account", {
        accountId: "github:github.com",
      });
    });
    await waitFor(() => {
      expect(screen.queryByTestId("auth-account-row-github:github.com")).toBeNull();
    });
  });

  it("surfaces a connect-error banner when connect_github_via_gh rejects", async () => {
    invokeMock
      .mockResolvedValueOnce([]) // list on mount
      .mockRejectedValueOnce(
        "no existing gh session — run `gh auth login` and try again",
      );

    const user = userEvent.setup();
    render(<SettingsPane />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());

    await user.click(screen.getByTestId("connect-github-submit"));

    await waitFor(() => {
      expect(screen.getByTestId("connect-github-error")).toBeInTheDocument();
    });
    expect(screen.getByTestId("connect-github-error")).toHaveTextContent(
      /gh auth login/i,
    );
    // Button re-enables for retry.
    expect(screen.getByTestId("connect-github-submit")).not.toBeDisabled();
  });

  // Responsive layout assertions.
  it("account row has flex-col base layout so actions stack at narrow widths", async () => {
    invokeMock.mockResolvedValueOnce([
      makeAccount({ id: "github:github.com" }),
    ]);
    render(<SettingsPane />);
    await waitFor(() =>
      expect(screen.getByTestId("auth-account-row-github:github.com")).toBeInTheDocument(),
    );
    const row = screen.getByTestId("auth-account-row-github:github.com");
    expect(row.className).toMatch(/flex-col/);
  });

  it("account row has sm:flex-row class to restore inline layout at sm breakpoint", async () => {
    invokeMock.mockResolvedValueOnce([
      makeAccount({ id: "github:github.com" }),
    ]);
    render(<SettingsPane />);
    await waitFor(() =>
      expect(screen.getByTestId("auth-account-row-github:github.com")).toBeInTheDocument(),
    );
    const row = screen.getByTestId("auth-account-row-github:github.com");
    expect(row.className).toMatch(/sm:flex-row/);
  });
});
