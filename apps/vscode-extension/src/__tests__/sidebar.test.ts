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

    // Intercept the html setter so we can capture it
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

  test("resolveWebviewView sets html containing <div id=root>", async () => {
    // __dirname at runtime = out/__tests__/; two levels up = extension root
    const extensionUri = vscode.Uri.file(
      path.resolve(__dirname, "..", ".."),
    );
    const provider = new AgenticChatViewProvider(extensionUri);
    const { view, capturedHtml } = makeMockWebviewView();

    await provider.resolveWebviewView(
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

  test("resolveWebviewView rewrites /assets/ paths through asWebviewUri", async () => {
    const extensionUri = vscode.Uri.file(
      path.resolve(__dirname, "..", ".."),
    );
    const provider = new AgenticChatViewProvider(extensionUri);
    const { view, capturedHtml } = makeMockWebviewView();

    await provider.resolveWebviewView(
      view,
      {} as vscode.WebviewViewResolveContext,
      new vscode.CancellationTokenSource().token,
    );

    const html = capturedHtml();
    // After rewriting, no src or href attribute should still point to "/assets/"
    const rawAssetsPattern = /(?:src|href)="\/assets\//;
    if (rawAssetsPattern.test(html)) {
      throw new Error(
        `Expected all /assets/ references to be rewritten via asWebviewUri, ` +
          `but found raw '/assets/' in html:\n${html.slice(0, 500)}`,
      );
    }
  });

  test("viewType constant matches package.json contributes.views entry", () => {
    // Guard: the viewType string must stay in sync with package.json.
    // If it drifts, the sidebar will silently not open.
    if (AgenticChatViewProvider.viewType !== "agentic.chat") {
      throw new Error(
        `viewType should be "agentic.chat", got "${AgenticChatViewProvider.viewType}"`,
      );
    }
  });

  test("webview localResourceRoots includes web-ui dist directory", async () => {
    const extensionUri = vscode.Uri.file(
      path.resolve(__dirname, "..", ".."),
    );
    const provider = new AgenticChatViewProvider(extensionUri);
    const { view } = makeMockWebviewView();

    await provider.resolveWebviewView(
      view,
      {} as vscode.WebviewViewResolveContext,
      new vscode.CancellationTokenSource().token,
    );

    const roots = view.webview.options.localResourceRoots;
    if (!roots || roots.length === 0) {
      throw new Error("Expected localResourceRoots to be set but it was empty");
    }
    const rootPaths = roots.map((u) => u.fsPath);
    const hasDistDir = rootPaths.some((p) =>
      p.endsWith(path.join("web-ui", "dist")),
    );
    if (!hasDistDir) {
      throw new Error(
        `Expected localResourceRoots to include 'web-ui/dist', got: ${rootPaths.join(", ")}`,
      );
    }
  });
});
