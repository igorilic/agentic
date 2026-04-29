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

  it("contains a single <link> with rel=stylesheet, href starting with Google Fonts Inter, and display=swap", () => {
    // Match a single <link> element that contains BOTH rel="stylesheet" and
    // the Inter Google Fonts href with display=swap.
    // Allow either attribute order (rel first or href first).
    const linkWithRelFirst =
      /<link[^>]*rel="stylesheet"[^>]*href="https:\/\/fonts\.googleapis\.com\/css2\?family=Inter[^"]*display=swap"[^>]*>/;
    const linkWithHrefFirst =
      /<link[^>]*href="https:\/\/fonts\.googleapis\.com\/css2\?family=Inter[^"]*display=swap"[^>]*rel="stylesheet"[^>]*>/;

    const matchesRelFirst = html.match(linkWithRelFirst);
    const matchesHrefFirst = html.match(linkWithHrefFirst);

    const totalMatches =
      (matchesRelFirst ? matchesRelFirst.length : 0) +
      (matchesHrefFirst ? matchesHrefFirst.length : 0);

    expect(totalMatches).toBe(1);
  });
});
