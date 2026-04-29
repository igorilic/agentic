import * as vscode from "vscode";
import { registerCommands } from "./commands";
import { AgenticChatViewProvider } from "./views/sidebar";

export function activate(context: vscode.ExtensionContext): void {
  registerCommands(context);

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
