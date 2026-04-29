import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { ALL_COMMAND_IDS } from "../commands";

suite("Command Registration", () => {
  async function ensureActivated(): Promise<void> {
    const ext = vscode.extensions.getExtension("agentic.agentic");
    if (!ext) {
      throw new Error(
        "extension not found by id 'agentic.agentic' — verify package.json `publisher` and `name` fields",
      );
    }
    if (!ext.isActive) {
      await ext.activate();
    }
  }

  test("all MVP agentic.* commands are registered after activation", async () => {
    await ensureActivated();

    const registered = await vscode.commands.getCommands(true);
    const missing = ALL_COMMAND_IDS.filter((id) => !registered.includes(id));

    if (missing.length > 0) {
      throw new Error(
        `Expected the following commands to be registered but they were missing:\n  ${missing.join("\n  ")}\n` +
          `Registered 'agentic.*' commands: ${registered.filter((c) => c.startsWith("agentic.")).join(", ")}`,
      );
    }
  });

  test("agentic.plan command executes without throwing", async () => {
    await ensureActivated();

    // Stub handler shows an info message and returns undefined. We
    // only assert that executeCommand resolves without rejecting.
    await vscode.commands.executeCommand("agentic.plan");
  });

  test("manifest contributes.commands matches runtime registrations exactly", () => {
    // __dirname at runtime = out/__tests__; package.json sits two up.
    const pkgPath = path.resolve(__dirname, "..", "..", "package.json");
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
    const manifestIds: string[] = pkg.contributes.commands.map(
      (c: { command: string }) => c.command,
    );

    const inManifestNotRuntime = manifestIds.filter(
      (id) => !ALL_COMMAND_IDS.includes(id),
    );
    const inRuntimeNotManifest = ALL_COMMAND_IDS.filter(
      (id) => !manifestIds.includes(id),
    );

    if (
      inManifestNotRuntime.length > 0 ||
      inRuntimeNotManifest.length > 0
    ) {
      throw new Error(
        `package.json contributes.commands and runtime registrations have drifted.\n` +
          `  In manifest only: ${inManifestNotRuntime.join(", ") || "(none)"}\n` +
          `  In runtime only:  ${inRuntimeNotManifest.join(", ") || "(none)"}`,
      );
    }
  });
});
