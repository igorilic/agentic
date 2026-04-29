import { isTauriDense } from "../utils/isTauriDense";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("isTauriDense", () => {
  beforeEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
    vi.unstubAllEnvs();
  });

  afterEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
    vi.unstubAllEnvs();
  });

  it("returns false when neither flag is set", () => {
    expect(isTauriDense()).toBe(false);
  });

  it("returns true when `window.__TAURI_INTERNALS__` is truthy", () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ =
      {};
    expect(isTauriDense()).toBe(true);
  });

  it("returns true when `import.meta.env.TAURI === '1'`", () => {
    vi.stubEnv("TAURI", "1");
    expect(isTauriDense()).toBe(true);
  });

  it("returns false when `TAURI` env is set to something other than '1'", () => {
    vi.stubEnv("TAURI", "0");
    expect(isTauriDense()).toBe(false);
  });
});
