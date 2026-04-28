import * as vscode from "vscode";
import { AgenticChatViewProvider } from "./views/sidebar";

export function activate(context: vscode.ExtensionContext): void {
  const disposable = vscode.commands.registerCommand("agentic.hello", () => {
    vscode.window.showInformationMessage("Hello from Agentic!");
  });
  context.subscriptions.push(disposable);

  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      AgenticChatViewProvider.viewType,
      new AgenticChatViewProvider(context.extensionUri),
    ),
  );
}

export function deactivate(): void {
  // nothing to clean up
}
