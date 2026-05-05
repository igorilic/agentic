/**
 * Webview-safe agent descriptor returned by the `list_agents` IPC.
 * No filesystem paths are included — the backend strips them.
 */
export type AgentInfoDto = {
  name: string;
  description: string | null;
  source: "project" | "home";
};
