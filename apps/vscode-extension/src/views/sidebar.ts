import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

/**
 * Provides the Agentic Chat sidebar view.
 *
 * Reads `<extension>/web-ui-dist/index.html` (copied from `apps/web-ui/dist`
 * by the pretest / vscode:prepublish hook so the path stays inside the
 * packaged `.vsix`), rewrites every absolute-path `src`/`href` through
 * `asWebviewUri` so VS Code can serve the files via the resource scheme,
 * and injects a CSP meta tag tied to `webview.cspSource`.
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
    // The web-ui-dist directory is copied into the extension root by the
    // pretest / vscode:prepublish hooks so this path stays inside the
    // extension folder both in dev (monorepo) and in a packaged .vsix.
    const distUri = vscode.Uri.joinPath(this._extensionUri, "web-ui-dist");

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [distUri],
    };

    webviewView.webview.html = this._buildHtml(webviewView.webview, distUri);
  }

  private _buildHtml(webview: vscode.Webview, distUri: vscode.Uri): string {
    const indexPath = path.join(distUri.fsPath, "index.html");
    let html: string;
    try {
      html = fs.readFileSync(indexPath, "utf8");
    } catch (err) {
      // Most common cause: web-ui hasn't been built. Surface an actionable
      // message instead of leaving the webview blank.
      const message =
        err instanceof Error && err.message
          ? err.message
          : String(err);
      vscode.window.showErrorMessage(
        `Agentic webview: web-ui build missing. Run \`pnpm --filter @agentic/web-ui build\` first. (${message})`,
      );
      return this._fallbackHtml(message);
    }

    // Rewrite every absolute root-relative URL (src / href) through
    // asWebviewUri. Generalised from `/assets/` so files emitted under
    // `apps/web-ui/public/` (e.g. a future favicon.svg) survive too.
    // Schemed URLs (`http:`, `data:`, `vscode-resource:`) start with a
    // scheme, not `/`, so the regex won't touch them.
    html = html.replace(
      /(src|href)="(\/[^"]+)"/g,
      (_match, attr: string, assetPath: string) => {
        const assetUri = vscode.Uri.joinPath(distUri, assetPath);
        const webviewUri = webview.asWebviewUri(assetUri);
        return `${attr}="${webviewUri.toString()}"`;
      },
    );

    // Inject a CSP meta tag after the opening <head> tag. VS Code's
    // default webview CSP is permissive when enableScripts is set; the
    // VS Code docs explicitly recommend a tight per-extension CSP. All
    // scripts/styles after the rewrite come from `webview.cspSource`.
    const csp = [
      `default-src 'none'`,
      `script-src ${webview.cspSource}`,
      `style-src ${webview.cspSource} 'unsafe-inline'`,
      `img-src ${webview.cspSource} data: https:`,
      `font-src ${webview.cspSource}`,
      `connect-src ${webview.cspSource}`,
    ].join("; ");
    const cspMeta = `<meta http-equiv="Content-Security-Policy" content="${csp}">`;
    html = html.replace(/<head(\s[^>]*)?>/i, (match) => `${match}\n  ${cspMeta}`);

    return html;
  }

  private _fallbackHtml(detail: string): string {
    // Static, no rewrites — no scripts loaded, so a CSP meta with
    // `default-src 'none'` is enough.
    return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';" />
    <title>Agentic — build required</title>
  </head>
  <body style="font-family: var(--vscode-font-family); padding: 1em;">
    <h2>Agentic web-ui not built</h2>
    <p>The sidebar is empty because the web-ui bundle is missing.</p>
    <p>From the repo root, run:</p>
    <pre style="background: var(--vscode-textCodeBlock-background); padding: 0.5em;">pnpm --filter @agentic/web-ui build</pre>
    <p style="opacity: 0.7; font-size: 0.9em;">${escapeHtml(detail)}</p>
  </body>
</html>`;
  }
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
