export type AuthProvider = "github" | "gitlab" | "jira" | "claude" | "copilot";

export type AuthAccount = {
  id: string;
  provider: AuthProvider;
  host: string;
  username: string | null;
  client_id: string | null;
  token_expires_at: number | null;
  created_at: number;
  last_used_at: number | null;
};
