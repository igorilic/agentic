import * as vscode from "vscode";

// The full MVP slash-command set that Step 14.4 must register.
// Keeping the list here as a single source of truth so this test
// and commands/index.ts can never drift.
const EXPECTED_COMMAND_IDS = [
  "agentic.hello",
  "agentic.plan",
  "agentic.status",
  "agentic.cancel",
  "agentic.triage",
  "agentic.answer",
  "agentic.retry",
  "agentic.resume",
  "agentic.workspace",
  "agentic.backend",
  "agentic.model",
  "agentic.settings",
  "agentic.runs",
  "agentic.pr",
  "agentic.clear",
  "agentic.help",
];

suite("Command Registration", () => {
  async function ensureActivated(): Promise<void> {
    const ext = vscode.extensions.getExtension("agentic.agentic");
    if (!ext) {
      throw new Error(
        "extension not found by id 'agentic.agentic' — verify package.json `publisher` and `name` fields",
      );
    }
    if (!ext.isActive) {
      await ext.activate();
    }
  }

  test("all MVP agentic.* commands are registered after activation", async () => {
    await ensureActivated();

    const registered = await vscode.commands.getCommands(true);
    const missing = EXPECTED_COMMAND_IDS.filter(
      (id) => !registered.includes(id),
    );

    if (missing.length > 0) {
      throw new Error(
        `Expected the following commands to be registered but they were missing:\n  ${missing.join("\n  ")}\n` +
          `Registered 'agentic.*' commands: ${registered.filter((c) => c.startsWith("agentic.")).join(", ")}`,
      );
    }
  });

  test("agentic.plan command executes without throwing", async () => {
    await ensureActivated();

    // The stub handler shows an info message and returns undefined.
    // We only assert that executeCommand resolves without rejecting.
    await vscode.commands.executeCommand("agentic.plan");
  });
});
