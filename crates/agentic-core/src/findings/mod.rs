//! Findings projection for ticket runs.
//!
//! The reviewer agent emits findings as a fenced markdown block in its
//! text output (see [`extractor`]). After the reviewer's step finishes,
//! the host parses that block, persists each entry via `FindingsRepo`,
//! and publishes a corresponding `Event::Finding` envelope so the
//! cockpit's live findings view picks it up.

pub mod extractor;
