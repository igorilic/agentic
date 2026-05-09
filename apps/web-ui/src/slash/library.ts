export type SlashCommandSpec = {
  name: string;
  desc: string;
};

export const SLASH_COMMAND_LIBRARY: readonly SlashCommandSpec[] = [
  { name: "plan", desc: "Start the default pipeline against a ticket" },
  { name: "brainstorm", desc: "Explore an idea collaboratively" },
  { name: "develop", desc: "Drive the pipeline against a ticket" },
  { name: "spec", desc: "Author a spec from the chat thread" },
  { name: "help", desc: "List available slash commands" },
] as const;
