import * as vscode from "vscode";

// Single source of truth: every entry drives both manifest (package.json
// contributes.commands) and runtime registration. Adding a command here
// without also adding it to package.json contributes.commands will cause
// Cmd+Shift+P not to surface it, but the handler will still be callable.
const COMMANDS: ReadonlyArray<{ id: string; stubMessage: string }> = [
  { id: "agentic.hello", stubMessage: "Hello from Agentic!" },
  { id: "agentic.plan", stubMessage: "Plan: implementation pending" },
  { id: "agentic.status", stubMessage: "Status: implementation pending" },
  { id: "agentic.cancel", stubMessage: "Cancel: implementation pending" },
  { id: "agentic.triage", stubMessage: "Triage: implementation pending" },
  { id: "agentic.answer", stubMessage: "Answer: implementation pending" },
  { id: "agentic.retry", stubMessage: "Retry: implementation pending" },
  { id: "agentic.resume", stubMessage: "Resume: implementation pending" },
  { id: "agentic.workspace", stubMessage: "Workspace: implementation pending" },
  { id: "agentic.backend", stubMessage: "Backend: implementation pending" },
  { id: "agentic.model", stubMessage: "Model: implementation pending" },
  { id: "agentic.settings", stubMessage: "Settings: implementation pending" },
  { id: "agentic.runs", stubMessage: "Runs: implementation pending" },
  { id: "agentic.pr", stubMessage: "PR: implementation pending" },
  { id: "agentic.clear", stubMessage: "Clear: implementation pending" },
];

export function registerCommands(context: vscode.ExtensionContext): void {
  for (const cmd of COMMANDS) {
    context.subscriptions.push(
      vscode.commands.registerCommand(cmd.id, () => {
        vscode.window.showInformationMessage(cmd.stubMessage);
      }),
    );
  }

  // agentic.help is special: it lists all Agentic commands via QuickPick.
  context.subscriptions.push(
    vscode.commands.registerCommand("agentic.help", async () => {
      const allCommands = await vscode.commands.getCommands(true);
      const agenticCommands = allCommands.filter((c) =>
        c.startsWith("agentic."),
      );
      await vscode.window.showQuickPick(agenticCommands, {
        placeHolder: "All registered Agentic commands",
      });
    }),
  );
}
