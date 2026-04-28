import * as vscode from "vscode";

/**
 * Placeholder — full implementation in GREEN phase.
 * Tests will fail until resolveWebviewView is implemented.
 */
export class AgenticChatViewProvider implements vscode.WebviewViewProvider {
  static readonly viewType = "agentic.chat";

  constructor(private readonly _extensionUri: vscode.Uri) {}

  resolveWebviewView(
    _webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken,
  ): void {
    // stub — intentionally empty so tests fail
  }
}
