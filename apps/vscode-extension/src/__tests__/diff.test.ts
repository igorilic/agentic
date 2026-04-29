// Step 14.5 — diff.ts unit tests
//
// Tests the two pure functions in diff.ts without loading the real napi
// binary or starting a vscode extension host:
//
//   1. AgenticDiffProvider.provideTextDocumentContent — resolves an
//      agentic://before/<hash> URI by calling the injected snapshot
//      fetcher and returns the bytes as a UTF-8 string.
//
//   2. openDiffForFileChange — calls vscode.commands.executeCommand
//      with 'vscode.diff' and the expected arg shape.
//
// Both functions accept their external dependencies through constructor
// injection so they can be tested with plain stubs here.

import * as path from "path";
import * as vscode from "vscode";
import { AgenticDiffProvider, openDiffForFileChange } from "../diff";

// ---------------------------------------------------------------------------
// Stub for the napi getFileSnapshot call
// ---------------------------------------------------------------------------

function makeStubFetcher(
  returnBytes: Buffer,
): (opts: { dataDir: string; hash: string }) => Promise<Buffer> {
  return async (_opts) => returnBytes;
}

// ---------------------------------------------------------------------------
// Test 1: provideTextDocumentContent calls the fetcher with the correct hash
//         and returns the bytes as a UTF-8 string.
// ---------------------------------------------------------------------------

suite("AgenticDiffProvider", () => {
  test("provideTextDocumentContent resolves agentic://before/<hash> to UTF-8 string", async () => {
    const expectedContent = "before file content\nline two\n";
    const fetcher = makeStubFetcher(Buffer.from(expectedContent));

    const provider = new AgenticDiffProvider("/fake/data-dir", fetcher);
    const uri = vscode.Uri.parse("agentic://before/abc123");
    const result = await provider.provideTextDocumentContent(uri);

    if (result !== expectedContent) {
      throw new Error(
        `Expected provider to return "${expectedContent}" but got "${result}"`,
      );
    }
  });

  test("provideTextDocumentContent passes the correct hash and dataDir to fetcher", async () => {
    const calls: Array<{ dataDir: string; hash: string }> = [];

    const recordingFetcher = async (opts: {
      dataDir: string;
      hash: string;
    }): Promise<Buffer> => {
      calls.push(opts);
      return Buffer.from("content");
    };

    const dataDir = "/workspace/.agentic";
    const provider = new AgenticDiffProvider(dataDir, recordingFetcher);
    const uri = vscode.Uri.parse("agentic://before/deadbeef1234");
    await provider.provideTextDocumentContent(uri);

    if (calls.length !== 1) {
      throw new Error(`Expected fetcher to be called once, got ${calls.length}`);
    }
    if (calls[0].hash !== "deadbeef1234") {
      throw new Error(
        `Expected hash "deadbeef1234" but fetcher received "${calls[0].hash}"`,
      );
    }
    if (calls[0].dataDir !== dataDir) {
      throw new Error(
        `Expected dataDir "${dataDir}" but fetcher received "${calls[0].dataDir}"`,
      );
    }
  });
});

// ---------------------------------------------------------------------------
// Test 2: openDiffForFileChange invokes vscode.commands.executeCommand
//         with the expected arguments.
// ---------------------------------------------------------------------------

suite("openDiffForFileChange", () => {
  test("calls vscode.diff with before URI, workspace file URI, and label containing 'Agentic'", async () => {
    const executedCommands: Array<{
      command: string;
      args: unknown[];
    }> = [];

    // Monkey-patch executeCommand for the duration of this test.
    const originalExecute = vscode.commands.executeCommand;
    (
      vscode.commands as unknown as {
        executeCommand: (...args: unknown[]) => Promise<unknown>;
      }
    ).executeCommand = async (
      command: unknown,
      ...args: unknown[]
    ): Promise<unknown> => {
      executedCommands.push({
        command: command as string,
        args,
      });
      return undefined;
    };

    const workspaceRoot = "/home/user/project";
    const filePath = "src/main.rs";
    const beforeHash = "cafebabe0000";

    const envelope = {
      run_id: "run-1",
      step_id: "step-1",
      event: {
        type: "FileChange" as const,
        path: filePath,
        before_hash: beforeHash,
        after_hash: "newhashnew1234",
      },
    };

    await openDiffForFileChange(envelope, workspaceRoot);

    // Restore original
    (
      vscode.commands as unknown as {
        executeCommand: (...args: unknown[]) => Promise<unknown>;
      }
    ).executeCommand = originalExecute as unknown as (
      ...args: unknown[]
    ) => Promise<unknown>;

    if (executedCommands.length !== 1) {
      throw new Error(
        `Expected executeCommand to be called once, got ${executedCommands.length}`,
      );
    }

    const { command, args } = executedCommands[0];

    if (command !== "vscode.diff") {
      throw new Error(
        `Expected command "vscode.diff" but got "${command}"`,
      );
    }

    // args[0]: before URI (agentic://before/<hash>)
    const beforeUri = args[0] as vscode.Uri;
    if (beforeUri.scheme !== "agentic") {
      throw new Error(
        `Expected before URI scheme "agentic" but got "${beforeUri.scheme}"`,
      );
    }
    if (!beforeUri.toString().includes(beforeHash)) {
      throw new Error(
        `Expected before URI to contain hash "${beforeHash}" but got "${beforeUri.toString()}"`,
      );
    }

    // args[1]: workspace file URI (file://...)
    const workspaceUri = args[1] as vscode.Uri;
    if (workspaceUri.scheme !== "file") {
      throw new Error(
        `Expected workspace URI scheme "file" but got "${workspaceUri.scheme}"`,
      );
    }
    const expectedFsPath = path.join(workspaceRoot, filePath);
    if (workspaceUri.fsPath !== expectedFsPath) {
      throw new Error(
        `Expected workspace URI fsPath "${expectedFsPath}" but got "${workspaceUri.fsPath}"`,
      );
    }

    // args[2]: label containing "Agentic"
    const label = args[2] as string;
    if (!label.includes("Agentic")) {
      throw new Error(
        `Expected diff label to contain "Agentic" but got "${label}"`,
      );
    }
  });
});
