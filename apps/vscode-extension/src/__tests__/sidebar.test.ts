import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";
import { AgenticChatViewProvider } from "../views/sidebar";

suite("AgenticChatViewProvider", () => {
  /**
   * Build a minimal hand-rolled mock for vscode.WebviewView.
   * This avoids the workbench-command timing flake and lets us do
   * synchronous assertions right after resolveWebviewView returns.
   */
  function makeMockWebviewView(): {
    view: vscode.WebviewView;
    capturedHtml: () => string;
    capturedUris: () => string[];
  } {
    const rewrittenUris: string[] = [];

    const webview: vscode.Webview = {
      options: {} as vscode.WebviewOptions,
      html: "",
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      onDidReceiveMessage: ((_listener: any) =>
        ({ dispose: () => {} } as vscode.Disposable)) as vscode.Event<unknown>,
      postMessage: (_message: unknown) => Promise.resolve(true),
      asWebviewUri: (uri: vscode.Uri): vscode.Uri => {
        const rewritten = vscode.Uri.parse(
          `vscode-resource://${uri.fsPath}`,
        );
        rewrittenUris.push(rewritten.toString());
        return rewritten;
      },
      cspSource: "https://vscode-cdn.net",
    };

    let _html = "";
    Object.defineProperty(webview, "html", {
      get: () => _html,
      set: (value: string) => {
        _html = value;
      },
    });

    const makeVoidEvent = (): vscode.Event<void> =>
      ((_listener: (e: void) => void) =>
        ({ dispose: () => {} } as vscode.Disposable)) as vscode.Event<void>;

    const view: vscode.WebviewView = {
      viewType: AgenticChatViewProvider.viewType,
      webview,
      title: undefined,
      description: undefined,
      badge: undefined,
      visible: true,
      onDidChangeVisibility: makeVoidEvent(),
      onDidDispose: makeVoidEvent(),
      show: (_preserveFocus?: boolean) => {},
    };

    return {
      view,
      capturedHtml: () => _html,
      capturedUris: () => rewrittenUris,
    };
  }

  // The pretest hook copies web-ui/dist into <extension>/web-ui-dist,
  // so resolving __dirname → out/__tests__ → up two levels → extension
  // root gives us a directory containing both `out/` and `web-ui-dist/`.
  const extensionRoot = path.resolve(__dirname, "..", "..");

  test("resolveWebviewView sets html containing <div id=root>", () => {
    const provider = new AgenticChatViewProvider(vscode.Uri.file(extensionRoot));
    const { view, capturedHtml } = makeMockWebviewView();

    provider.resolveWebviewView(
      view,
      {} as vscode.WebviewViewResolveContext,
      new vscode.CancellationTokenSource().token,
    );

    const html = capturedHtml();
    if (!html.includes('<div id="root">')) {
      throw new Error(
        `Expected html to contain '<div id="root">' but got:\n${html.slice(0, 500)}`,
      );
    }
  });

  test("rewrites root-absolute paths through asWebviewUri (positive assertion)", () => {
    const provider = new AgenticChatViewProvider(vscode.Uri.file(extensionRoot));
    const { view, capturedHtml, capturedUris } = makeMockWebviewView();

    provider.resolveWebviewView(
      view,
      {} as vscode.WebviewViewResolveContext,
      new vscode.CancellationTokenSource().token,
    );

    const html = capturedHtml();
    if (/(?:src|href)="\/[^"]+"/.test(html)) {
      throw new Error(
        `Expected all root-absolute paths to be rewritten via asWebviewUri, ` +
          `but found one in html:\n${html.slice(0, 500)}`,
      );
    }
    // Positive assertion: asWebviewUri actually got called. Without
    // this a hardcoded-disk-path implementation would still pass the
    // negative check above.
    if (capturedUris().length === 0) {
      throw new Error(
        "Expected asWebviewUri to be called at least once, but capturedUris is empty",
      );
    }
  });

  test("html includes a Content-Security-Policy meta tag using cspSource", () => {
    const provider = new AgenticChatViewProvider(vscode.Uri.file(extensionRoot));
    const { view, capturedHtml } = makeMockWebviewView();

    provider.resolveWebviewView(
      view,
      {} as vscode.WebviewViewResolveContext,
      new vscode.CancellationTokenSource().token,
    );

    const html = capturedHtml();
    if (!/<meta\s+http-equiv="Content-Security-Policy"/i.test(html)) {
      throw new Error(
        `Expected a CSP meta tag in html, got:\n${html.slice(0, 500)}`,
      );
    }
    if (!html.includes("https://vscode-cdn.net")) {
      throw new Error(
        "Expected the CSP to reference webview.cspSource (mocked as https://vscode-cdn.net)",
      );
    }
  });

  test("missing dist falls back to a build-instruction page", () => {
    // Point extensionUri at a tmpdir without web-ui-dist.
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "agentic-vsx-empty-"));
    const provider = new AgenticChatViewProvider(vscode.Uri.file(tmp));
    const { view, capturedHtml } = makeMockWebviewView();

    provider.resolveWebviewView(
      view,
      {} as vscode.WebviewViewResolveContext,
      new vscode.CancellationTokenSource().token,
    );

    const html = capturedHtml();
    if (!html.includes("pnpm --filter @agentic/web-ui build")) {
      throw new Error(
        `Expected fallback html to include the build command, got:\n${html.slice(0, 500)}`,
      );
    }
    // Fallback must still include a CSP meta so it isn't more permissive
    // than the happy path.
    if (!/<meta\s+http-equiv="Content-Security-Policy"/i.test(html)) {
      throw new Error("Fallback html must include a CSP meta");
    }

    fs.rmSync(tmp, { recursive: true, force: true });
  });

  test("viewType constant matches package.json contributes.views entry", () => {
    if (AgenticChatViewProvider.viewType !== "agentic.chat") {
      throw new Error(
        `viewType should be "agentic.chat", got "${AgenticChatViewProvider.viewType}"`,
      );
    }
  });

  test("webview localResourceRoots includes the extension's web-ui-dist directory", () => {
    const provider = new AgenticChatViewProvider(vscode.Uri.file(extensionRoot));
    const { view } = makeMockWebviewView();

    provider.resolveWebviewView(
      view,
      {} as vscode.WebviewViewResolveContext,
      new vscode.CancellationTokenSource().token,
    );

    const roots = view.webview.options.localResourceRoots;
    if (!roots || roots.length === 0) {
      throw new Error("Expected localResourceRoots to be set but it was empty");
    }
    const rootPaths = roots.map((u) => u.fsPath);
    const hasDistDir = rootPaths.some((p) => p.endsWith("web-ui-dist"));
    if (!hasDistDir) {
      throw new Error(
        `Expected localResourceRoots to end with 'web-ui-dist', got: ${rootPaths.join(", ")}`,
      );
    }
  });
});
