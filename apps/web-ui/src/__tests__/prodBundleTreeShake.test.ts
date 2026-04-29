// @vitest-environment node
/**
 * Asserts that the production bundle does not ship the StartRunForm body.
 *
 * This test runs `pnpm build` (full Vite production build) and then greps
 * the output for the data-testid strings that would only appear if the
 * StartRunFormInner chunk leaked into production.
 *
 * It is slow (~1s build) so it is skipped by default when running locally
 * with SKIP_BUILD_TEST=1. Always runs in CI (SKIP_BUILD_TEST is not set).
 *
 * Usage:
 *   SKIP_BUILD_TEST=1 pnpm test   # skip this test (reported as "skipped")
 *   pnpm test                      # run all tests including this one
 */

import { execSync } from "child_process";
import { readFileSync, readdirSync } from "fs";
import { join } from "path";

const SKIP = process.env.SKIP_BUILD_TEST === "1";
const UI_ROOT = new URL("../../", import.meta.url).pathname.replace(/\/$/, "");
const DIST_ASSETS = join(UI_ROOT, "dist", "assets");

// Strings that appear ONLY inside the form body — their presence in dist
// means the StartRunFormInner chunk was not tree-shaken.
const FORM_STRINGS = ["start-run-form", "script-path-input"];

// describe.skipIf marks the whole suite as "skipped" in the Vitest report
// when SKIP_BUILD_TEST=1, instead of silently passing via an early return.
describe.skipIf(SKIP)("prod bundle tree-shake", () => {
  beforeAll(() => {
    // Build the production bundle. Throws if the build fails.
    execSync("pnpm build", {
      cwd: UI_ROOT,
      stdio: "pipe",
    });
  }, 120_000); // 2-minute timeout for the build step

  it("does not ship StartRunFormInner strings in the production bundle", () => {
    const jsFiles = readdirSync(DIST_ASSETS).filter((f) => f.endsWith(".js"));
    expect(jsFiles.length).toBeGreaterThan(0);

    for (const file of jsFiles) {
      const content = readFileSync(join(DIST_ASSETS, file), "utf-8");
      for (const str of FORM_STRINGS) {
        expect(content).not.toContain(str);
      }
    }
  });
});
