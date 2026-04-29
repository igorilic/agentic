import * as vscode from "vscode";

/**
 * Single source of truth: every entry drives both the runtime
 * `registerCommand` call AND the test that asserts manifest parity.
 * Importers must NOT hardcode their own copy.
 *
 * `title` matches `package.json#contributes.commands[].title` exactly so
 * the help QuickPick can show user-readable labels and so a parity
 * test can compare the two without a mapping table.
 */
export const COMMANDS: ReadonlyArray<{
  readonly id: string;
  readonly title: string;
  readonly stubMessage: string;
}> = [
  { id: "agentic.plan", title: "Agentic: Plan", stubMessage: "Plan: implementation pending" },
  { id: "agentic.status", title: "Agentic: Show Status", stubMessage: "Status: implementation pending" },
  { id: "agentic.cancel", title: "Agentic: Cancel Run", stubMessage: "Cancel: implementation pending" },
  { id: "agentic.triage", title: "Agentic: Triage Finding", stubMessage: "Triage: implementation pending" },
  { id: "agentic.answer", title: "Agentic: Answer Clarifying Question", stubMessage: "Answer: implementation pending" },
  { id: "agentic.retry", title: "Agentic: Retry Step", stubMessage: "Retry: implementation pending" },
  { id: "agentic.resume", title: "Agentic: Resume Run", stubMessage: "Resume: implementation pending" },
  { id: "agentic.workspace", title: "Agentic: Switch Workspace", stubMessage: "Workspace: implementation pending" },
  { id: "agentic.backend", title: "Agentic: Switch Backend", stubMessage: "Backend: implementation pending" },
  { id: "agentic.model", title: "Agentic: Switch Model", stubMessage: "Model: implementation pending" },
  { id: "agentic.settings", title: "Agentic: Open Settings", stubMessage: "Settings: implementation pending" },
  { id: "agentic.runs", title: "Agentic: Past Runs", stubMessage: "Runs: implementation pending" },
  { id: "agentic.pr", title: "Agentic: Open PR", stubMessage: "PR: implementation pending" },
  { id: "agentic.clear", title: "Agentic: Clear Chat", stubMessage: "Clear: implementation pending" },
];

const HELP_COMMAND = { id: "agentic.help", title: "Agentic: Help" };

/**
 * Every command id this extension contributes — the stubbed set plus
 * the `agentic.help` meta-command. Tests use this to assert manifest
 * ↔ runtime parity.
 */
export const ALL_COMMAND_IDS: readonly string[] = [
  ...COMMANDS.map((c) => c.id),
  HELP_COMMAND.id,
];

export function registerCommands(context: vscode.ExtensionContext): void {
  for (const cmd of COMMANDS) {
    context.subscriptions.push(
      vscode.commands.registerCommand(cmd.id, () => {
        vscode.window.showInformationMessage(cmd.stubMessage);
      }),
    );
  }

  // `agentic.help` is the one non-stub: shows a QuickPick with
  // user-readable titles and dispatches to the picked command.
  context.subscriptions.push(
    vscode.commands.registerCommand(HELP_COMMAND.id, async () => {
      // Build items from the local COMMANDS array — deterministic and
      // doesn't pull in unrelated `agentic.*` ids that other extensions
      // might register at runtime. `description` carries the id so we
      // can dispatch on selection without a parallel lookup.
      const items: vscode.QuickPickItem[] = [
        ...COMMANDS.map((c) => ({ label: c.title, description: c.id })),
        { label: HELP_COMMAND.title, description: HELP_COMMAND.id },
      ];
      const picked = await vscode.window.showQuickPick(items, {
        placeHolder: "Agentic commands",
      });
      if (picked && picked.description) {
        await vscode.commands.executeCommand(picked.description);
      }
    }),
  );
}
