import { describe, it, expect } from "vitest";
import { reorderArray, insertAt } from "../utils/arrayMove";

describe("reorderArray", () => {
  it("forward move: moves element from index 0 to index 2", () => {
    const result = reorderArray(["a", "b", "c", "d"], 0, 2);
    expect(result).toEqual(["b", "c", "a", "d"]);
  });

  it("backward move: moves element from index 3 to index 1", () => {
    const result = reorderArray(["a", "b", "c", "d"], 3, 1);
    expect(result).toEqual(["a", "d", "b", "c"]);
  });

  it("no-op: same from and to returns a clone with unchanged order", () => {
    const arr = ["a", "b", "c"];
    const result = reorderArray(arr, 1, 1);
    expect(result).toEqual(["a", "b", "c"]);
    // Must be a new array (not the same reference)
    expect(result).not.toBe(arr);
  });

  it("out-of-bounds from index returns a clone unchanged", () => {
    const arr = ["a", "b", "c"];
    const result = reorderArray(arr, -1, 1);
    expect(result).toEqual(["a", "b", "c"]);
    expect(result).not.toBe(arr);
  });

  it("out-of-bounds from index (too large) returns a clone unchanged", () => {
    const arr = ["a", "b", "c"];
    const result = reorderArray(arr, 5, 1);
    expect(result).toEqual(["a", "b", "c"]);
    expect(result).not.toBe(arr);
  });

  it("out-of-bounds to index returns a clone unchanged", () => {
    const arr = ["a", "b", "c"];
    const result = reorderArray(arr, 0, -1);
    expect(result).toEqual(["a", "b", "c"]);
    expect(result).not.toBe(arr);
  });

  it("out-of-bounds to index (too large) returns a clone unchanged", () => {
    const arr = ["a", "b", "c"];
    const result = reorderArray(arr, 0, 10);
    expect(result).toEqual(["a", "b", "c"]);
    expect(result).not.toBe(arr);
  });

  it("empty array returns empty array", () => {
    const result = reorderArray([], 0, 1);
    expect(result).toEqual([]);
  });

  it("does not mutate the original array", () => {
    const arr = ["a", "b", "c", "d"];
    const original = [...arr];
    reorderArray(arr, 0, 2);
    expect(arr).toEqual(original);
  });
});

describe("insertAt", () => {
  it("inserts at start (index 0)", () => {
    const result = insertAt(["a", "b", "c"], 0, "x");
    expect(result).toEqual(["x", "a", "b", "c"]);
  });

  it("inserts in the middle (index 2)", () => {
    const result = insertAt(["a", "b", "c", "d"], 2, "x");
    expect(result).toEqual(["a", "b", "x", "c", "d"]);
  });

  it("inserts at end (index === length)", () => {
    const result = insertAt(["a", "b", "c"], 3, "x");
    expect(result).toEqual(["a", "b", "c", "x"]);
  });

  it("out-of-bounds negative index clamps to start", () => {
    const result = insertAt(["a", "b", "c"], -5, "x");
    expect(result).toEqual(["x", "a", "b", "c"]);
  });

  it("out-of-bounds large index clamps to end", () => {
    const result = insertAt(["a", "b", "c"], 100, "x");
    expect(result).toEqual(["a", "b", "c", "x"]);
  });

  it("empty array returns single-element array", () => {
    const result = insertAt([], 0, "x");
    expect(result).toEqual(["x"]);
  });

  it("does not mutate the original array", () => {
    const arr = ["a", "b", "c"];
    const original = [...arr];
    insertAt(arr, 1, "x");
    expect(arr).toEqual(original);
  });
});
