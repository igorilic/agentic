// Step 14.1 smoke test — round-trip a scripted run through the napi
// bridge: startRun → iterate subscribeEvents → triageFinding.
//
// This proves the three-function contract end-to-end against a real
// agentic-core (real bus, real DB) without needing a running VS Code
// extension host.

import { describe, expect, it, beforeEach } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync } from "fs";
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

    // Pre-triage: row exists with triage unset (napi maps Option::None
    // to undefined, not null).
    const before = await node.listFindings({ dataDir, runId });
    expect(before).toHaveLength(1);
    expect(before[0].id).toBe("smoke-f1");
    expect(before[0].triage).toBeUndefined();

    await node.triageFinding({
      dataDir,
      runId,
      findingId: "smoke-f1",
      triage: "tech-debt",
    });

    // Post-triage: same row, triage updated.
    const after = await node.listFindings({ dataDir, runId });
    expect(after).toHaveLength(1);
    expect(after[0].triage).toBe("tech-debt");
    expect(after[0].triagedAt).toBeGreaterThan(0);
  });

  it("listFindings returns an empty array for an unknown run", async () => {
    const findings = await node.listFindings({
      dataDir,
      runId: "nonexistent-run",
    });
    expect(findings).toEqual([]);
  });

  it("cancelRun returns true for an in-flight run, false afterwards", async () => {
    // Use a longer delay so the run is definitely still in flight
    // when we cancel — at delay=50ms with 5 events the loop is
    // alive for ~250ms.
    const { runId } = await node.startRun({
      dataDir,
      scriptPath: FIXTURE,
      delayMs: 50,
    });
    expect(node.cancelRun(runId)).toBe(true);

    // Drain to ensure the run actually winds down (publishes its
    // RunComplete after observing the cancel) before we re-query.
    const stream = node.subscribeEvents(runId);
    for await (const env of node.iterate(stream)) {
      if (env.event.type === "RunComplete") break;
    }

    // Map entry has been removed by the loop's cleanup; second
    // cancel returns false.
    expect(node.cancelRun(runId)).toBe(false);
  });

  it("cancelRun returns false for an unknown run id", () => {
    expect(node.cancelRun("never-existed")).toBe(false);
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

  it("getFileSnapshot returns bytes written to <dataDir>/snapshots/<hash>", async () => {
    // Write a known blob under the expected path.
    const snapshotsDir = join(dataDir, "snapshots");
    mkdirSync(snapshotsDir, { recursive: true });
    const hash = "deadbeefdeadbeef";
    const content = Buffer.from("hello snapshot");
    writeFileSync(join(snapshotsDir, hash), content);

    const result: Buffer = await node.getFileSnapshot({ dataDir, hash });
    expect(result).toBeInstanceOf(Buffer);
    expect(result.toString()).toBe("hello snapshot");
  });
});
