// JS wrapper around the napi-generated bindings.
//
// The Rust side exposes an `EventStream` class with `async next()`
// returning a JSON string or null. This file re-exports that plus an
// `iterate(stream)` helper that turns it into a real AsyncIterable so
// callers can `for await (const env of iterate(stream)) { ... }`.
//
// Once VS Code extension code stabilises, this wrapper will likely be
// replaced by a typed TS shim with proper EventEnvelope types.

const native = require("./index.js");

async function* iterate(stream) {
  while (true) {
    const json = await stream.next();
    if (json === null || json === undefined) return;
    yield JSON.parse(json);
  }
}

module.exports = {
  ...native,
  iterate,
};
