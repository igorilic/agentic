import * as vscode from "vscode";

export function activate(context: vscode.ExtensionContext): void {
  const disposable = vscode.commands.registerCommand("agentic.hello", () => {
    vscode.window.showInformationMessage("Hello from Agentic!");
  });

  context.subscriptions.push(disposable);
}

export function deactivate(): void {
  // nothing to clean up
}
