import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

/**
 * Provides the Agentic Chat sidebar view.
 *
 * Reads `apps/web-ui/dist/index.html` (built separately via the pretest
 * script), rewrites every `/assets/...` src/href through `asWebviewUri` so
 * VS Code can serve the files from disk, and sets the result as the
 * webview's HTML.
 *
 * Message-passing to @agentic/node is deferred to Step 14.4+.
 */
export class AgenticChatViewProvider implements vscode.WebviewViewProvider {
  static readonly viewType = "agentic.chat";

  constructor(private readonly _extensionUri: vscode.Uri) {}

  resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken,
  ): void {
    const distUri = vscode.Uri.joinPath(
      this._extensionUri,
      "..",
      "web-ui",
      "dist",
    );

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [distUri],
    };

    webviewView.webview.html = this._buildHtml(webviewView.webview, distUri);
  }

  private _buildHtml(webview: vscode.Webview, distUri: vscode.Uri): string {
    const indexPath = path.join(distUri.fsPath, "index.html");
    let html = fs.readFileSync(indexPath, "utf8");

    // Rewrite every src="/assets/..." and href="/assets/..." so VS Code can
    // serve the files from disk using the vscode-resource scheme.
    html = html.replace(
      /(?:src|href)="(\/assets\/[^"]+)"/g,
      (match, assetPath) => {
        const assetUri = vscode.Uri.joinPath(distUri, assetPath);
        const webviewUri = webview.asWebviewUri(assetUri);
        // Preserve the original attribute name (src vs href)
        const attr = match.startsWith("src") ? "src" : "href";
        return `${attr}="${webviewUri.toString()}"`;
      },
    );

    return html;
  }
}
