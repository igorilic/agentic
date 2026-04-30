import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { describe, it, expect } from "vitest";

const SRC_ROOT = path.resolve(__dirname, "..");

const DELETED_FILES = [
  "components/Stepper.tsx",
  "components/EventList.tsx",
  "components/ActiveRunIndicator.tsx",
  "components/StartRunForm.tsx",
  "components/StartRunFormInner.tsx",
];

describe("dead code removal (W.8.5)", () => {
  for (const rel of DELETED_FILES) {
    it(`${rel} no longer exists`, () => {
      expect(existsSync(path.join(SRC_ROOT, rel))).toBe(false);
    });
  }

  it("App.tsx does not import any of the deleted modules", () => {
    const appSrc = readFileSync(path.join(SRC_ROOT, "App.tsx"), "utf8");
    for (const rel of DELETED_FILES) {
      const moduleName = rel.replace(/\.tsx$/, "").split("/").pop()!;
      expect(appSrc).not.toMatch(new RegExp(`from\\s+["'][^"']*${moduleName}["']`));
    }
  });
});
