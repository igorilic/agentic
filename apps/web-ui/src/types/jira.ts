/** Mirrors `JiraTicketDto` in `crates/agentic-tauri/src/commands/jira.rs`. */
export type JiraTicketDto = {
  key: string;
  title: string;
  body: string;
  ac: string | null;
};
