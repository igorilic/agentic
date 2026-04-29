import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, it, expect } from "vitest";

const html = readFileSync(
  resolve(__dirname, "../../index.html"),
  "utf-8"
);

describe("index.html Inter font links", () => {
  it("contains preconnect to fonts.googleapis.com", () => {
    expect(html).toContain(
      '<link rel="preconnect" href="https://fonts.googleapis.com">'
    );
  });

  it("contains preconnect to fonts.gstatic.com with crossorigin", () => {
    expect(html).toContain(
      '<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>'
    );
  });

  it("contains Google Fonts stylesheet link for Inter with display=swap", () => {
    // Verify we have a stylesheet link that points to the Inter family and has display=swap
    const stylesheetPattern =
      /href="https:\/\/fonts\.googleapis\.com\/css2\?family=Inter[^"]*display=swap"/;
    expect(html).toMatch(stylesheetPattern);

    // Also verify it is a rel="stylesheet" link
    expect(html).toContain('rel="stylesheet"');
  });
});
