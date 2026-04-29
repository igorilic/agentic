import * as vscode from "vscode";

suite("Extension Activation", () => {
  test("activates and registers the MVP command surface", async () => {
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

    // Pick one well-known command from the MVP set as a smoke for
    // "the registerCommands call ran". The full set is asserted in
    // commands.test.ts; this test exists only to prove activation
    // succeeded at all (i.e. the onStartupFinished hook fired and
    // extension.ts didn't throw).
    const commands = await vscode.commands.getCommands(true);
    if (!commands.includes("agentic.plan")) {
      throw new Error(
        `Expected 'agentic.plan' to be registered after activation. ` +
          `Registered 'agentic.*' commands: ${commands.filter((c) => c.startsWith("agentic.")).join(", ")}`,
      );
    }
  });
});
