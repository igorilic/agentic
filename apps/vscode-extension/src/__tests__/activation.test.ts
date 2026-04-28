import * as vscode from "vscode";

suite("Extension Activation", () => {
  test("registers the agentic.hello command after activation", async () => {
    // Marketplace ID is `<publisher>.<name>` from package.json — see
    // spec.md §20.2. A future rename of either field must update this
    // string AND package.json together.
    const ext = vscode.extensions.getExtension("agentic.agentic");
    if (!ext) {
      throw new Error(
        "extension not found by id 'agentic.agentic' — verify package.json `publisher` and `name` fields",
      );
    }
    if (!ext.isActive) {
      await ext.activate();
    }

    const commands = await vscode.commands.getCommands(true);
    if (!commands.includes("agentic.hello")) {
      throw new Error(
        `Expected 'agentic.hello' to be registered. Registered 'agentic' commands: ${commands.filter((c) => c.startsWith("agentic")).join(", ")}`,
      );
    }
  });
});
