import * as vscode from "vscode";

suite("Extension Activation", () => {
  test("registers the agentic.hello command after activation", async () => {
    // Wait for the extension to activate (onStartupFinished)
    const ext = vscode.extensions.getExtension("agentic.vscode-extension");
    if (ext && !ext.isActive) {
      await ext.activate();
    }

    const commands = await vscode.commands.getCommands(true);
    const hasHello = commands.includes("agentic.hello");

    if (!hasHello) {
      throw new Error(
        `Expected 'agentic.hello' to be registered, but it was not found. ` +
          `Registered commands starting with 'agentic': ${commands.filter((c) => c.startsWith("agentic")).join(", ")}`,
      );
    }
  });
});
