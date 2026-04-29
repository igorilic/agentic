import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { ALL_COMMAND_IDS, makeTriageHandler } from "../commands";
import type { FindingsDecorator } from "../decorations";

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

  // ─── makeTriageHandler unit tests ───────────────────────────────────────
  // The handler is the most complex new code path in Step 14.6. Pure-fn
  // tests against stubbed deps (no extension host needed for this slice).

  function makeStubs() {
    const triageCalls: Array<{
      dataDir: string;
      runId: string;
      findingId: string;
      triage: string;
    }> = [];
    const clearedIds: string[] = [];
    const errors: string[] = [];
    let triageImpl: () => Promise<void> = async () => {};
    const node = {
      triageFinding: async (args: {
        dataDir: string;
        runId: string;
        findingId: string;
        triage: string;
      }) => {
        triageCalls.push(args);
        await triageImpl();
      },
    };
    const decorator = {
      clearFinding: (id: string) => {
        clearedIds.push(id);
      },
    } as unknown as FindingsDecorator;
    const handler = makeTriageHandler({
      node,
      decorator,
      getActiveEditor: () => undefined,
      showError: (msg: string) => {
        errors.push(msg);
      },
    });
    return {
      handler,
      triageCalls,
      clearedIds,
      errors,
      setTriageImpl: (impl: () => Promise<void>) => {
        triageImpl = impl;
      },
    };
  }

  test("triage handler shows error for missing args object", async () => {
    const s = makeStubs();
    await s.handler(undefined as never);
    if (s.triageCalls.length !== 0) {
      throw new Error("triageFinding must NOT be called for missing args");
    }
    if (s.errors.length !== 1 || !s.errors[0].includes("invalid triage")) {
      throw new Error(
        `expected one 'invalid triage' error, got: ${JSON.stringify(s.errors)}`,
      );
    }
  });

  test("triage handler shows error for invalid triage value", async () => {
    const s = makeStubs();
    await s.handler({
      dataDir: "/d",
      runId: "r",
      findingId: "f",
      triage: "bogus",
    });
    if (s.triageCalls.length !== 0) {
      throw new Error("triageFinding must NOT be called for invalid triage");
    }
    if (s.errors.length !== 1 || !s.errors[0].includes("bogus")) {
      throw new Error(
        `expected error mentioning 'bogus', got: ${JSON.stringify(s.errors)}`,
      );
    }
  });

  test("triage handler success path: calls triageFinding then clears decoration", async () => {
    const s = makeStubs();
    await s.handler({
      dataDir: "/d",
      runId: "r",
      findingId: "f1",
      triage: "fix",
    });
    if (s.triageCalls.length !== 1 || s.triageCalls[0].findingId !== "f1") {
      throw new Error(
        `expected one triageFinding call for 'f1', got: ${JSON.stringify(s.triageCalls)}`,
      );
    }
    if (s.clearedIds.length !== 1 || s.clearedIds[0] !== "f1") {
      throw new Error(
        `expected clearFinding('f1'), got: ${JSON.stringify(s.clearedIds)}`,
      );
    }
    if (s.errors.length !== 0) {
      throw new Error(
        `no errors expected on happy path, got: ${JSON.stringify(s.errors)}`,
      );
    }
  });

  test("triage handler error path: napi throws, decoration NOT cleared, error shown", async () => {
    const s = makeStubs();
    s.setTriageImpl(async () => {
      throw new Error("backend exploded");
    });
    await s.handler({
      dataDir: "/d",
      runId: "r",
      findingId: "f1",
      triage: "fix",
    });
    if (s.clearedIds.length !== 0) {
      throw new Error(
        `clearFinding must NOT be called on triage failure, got: ${JSON.stringify(s.clearedIds)}`,
      );
    }
    if (
      s.errors.length !== 1 ||
      !s.errors[0].includes("backend exploded") ||
      !s.errors[0].includes("triage failed")
    ) {
      throw new Error(
        `expected one 'triage failed' error mentioning 'backend exploded', got: ${JSON.stringify(s.errors)}`,
      );
    }
  });

  test("triage handler in-flight guard suppresses concurrent double-click", async () => {
    const s = makeStubs();
    let release: () => void = () => {};
    s.setTriageImpl(
      () =>
        new Promise<void>((r) => {
          release = r;
        }),
    );
    const args = {
      dataDir: "/d",
      runId: "r",
      findingId: "f1",
      triage: "fix",
    };
    // Fire two clicks; the first awaits forever until release().
    const first = s.handler(args);
    const second = s.handler(args);
    // Settle the second microtask — it should have hit the inflight guard
    // and resolved already without calling triageFinding twice.
    await second;
    if (s.triageCalls.length !== 1) {
      throw new Error(
        `expected exactly 1 triageFinding call (in-flight guard), got ${s.triageCalls.length}`,
      );
    }
    // Release the first so it resolves, then reset the impl so the
    // third call doesn't hang on the same pending promise.
    release();
    await first;
    s.setTriageImpl(async () => {});
    // After the first completes, a third click for the same id is allowed
    // again because the guard is cleared in finally. Capture the length
    // first to side-step TS narrowing from the earlier `!== 1` check.
    const beforeThird = s.triageCalls.length as number;
    await s.handler(args);
    if (s.triageCalls.length - beforeThird !== 1) {
      throw new Error(
        `expected one NEW triageFinding call after guard cleared, got ${s.triageCalls.length - beforeThird}`,
      );
    }
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
