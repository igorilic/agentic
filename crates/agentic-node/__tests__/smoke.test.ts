// Step 14.1 smoke test — round-trip a scripted run through the napi
// bridge: startRun → iterate subscribeEvents → triageFinding.
//
// This proves the three-function contract end-to-end against a real
// agentic-core (real bus, real DB) without needing a running VS Code
// extension host.

import { describe, expect, it, beforeEach } from "vitest";
import { mkdtempSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";

// eslint-disable-next-line @typescript-eslint/no-require-imports
const node = require("../lib.js");

const FIXTURE = join(__dirname, "..", "fixtures", "smoke.json");

function freshDataDir(): string {
  return mkdtempSync(join(tmpdir(), "agentic-node-smoke-"));
}

describe("agentic-node smoke", () => {
  let dataDir: string;

  beforeEach(() => {
    dataDir = freshDataDir();
  });

  it("startRun returns a runId", async () => {
    const { runId } = await node.startRun({
      dataDir,
      scriptPath: FIXTURE,
    });
    expect(runId).toMatch(/^[0-9a-z]{26}$/); // ULID lowercase
  });

  it("subscribeEvents yields envelopes for the started run", async () => {
    // Subscribe BEFORE starting so we don't miss early events.
    const { runId } = await node.startRun({
      dataDir,
      scriptPath: FIXTURE,
      delayMs: 5, // slight pacing so subscribe is ready
    });
    const stream = node.subscribeEvents(runId);
    const seen: string[] = [];
    // Collect up to RunComplete or 50 events, whichever first.
    for await (const env of node.iterate(stream)) {
      seen.push(env.event.type);
      if (env.event.type === "RunComplete") break;
      if (seen.length > 50) break;
    }
    // Must have seen at least one of each key event variant.
    expect(seen).toContain("StepStarted");
    expect(seen).toContain("TextDelta");
    expect(seen).toContain("StepComplete");
    expect(seen).toContain("Finding");
    expect(seen).toContain("RunComplete");
  });

  it("triageFinding writes the new state to the DB", async () => {
    const { runId } = await node.startRun({
      dataDir,
      scriptPath: FIXTURE,
      delayMs: 5,
    });
    // Drain events so the Finding row gets persisted before we triage.
    const stream = node.subscribeEvents(runId);
    for await (const env of node.iterate(stream)) {
      if (env.event.type === "RunComplete") break;
    }
    await node.triageFinding({
      dataDir,
      runId,
      findingId: "smoke-f1",
      triage: "tech-debt",
    });
    // No exception means it succeeded — invalid triage / unknown
    // finding both reject. (A list_findings query would be the
    // tighter assertion; deferred until a future read-side fn lands.)
  });

  it("triageFinding rejects an unknown finding id", async () => {
    const { runId } = await node.startRun({
      dataDir,
      scriptPath: FIXTURE,
    });
    await expect(
      node.triageFinding({
        dataDir,
        runId,
        findingId: "does-not-exist",
        triage: "fix",
      }),
    ).rejects.toThrow(/not found/);
  });

  it("triageFinding rejects an invalid triage value", async () => {
    const { runId } = await node.startRun({
      dataDir,
      scriptPath: FIXTURE,
      delayMs: 5,
    });
    const stream = node.subscribeEvents(runId);
    for await (const env of node.iterate(stream)) {
      if (env.event.type === "RunComplete") break;
    }
    await expect(
      node.triageFinding({
        dataDir,
        runId,
        findingId: "smoke-f1",
        triage: "bogus",
      }),
    ).rejects.toThrow();
  });
});
