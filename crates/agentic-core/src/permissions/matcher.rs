//! Pattern grammar for `permissions.toml` rules (v1).
//!
//! # Pattern shapes
//!
//! Two shapes are supported:
//!
//! ## 1. Tool wildcard — matches any arg for the named tool
//!
//! ```text
//! <tool>:*
//! ```
//!
//! - `<tool>` is an exact, case-sensitive tool name.
//! - `:*` is a literal suffix — no other content after the colon is allowed.
//!
//! Examples: `Bash:*`, `Read:*`, `bash:*`
//!
//! ## 2. Arg glob — shell-glob on the full argument string
//!
//! ```text
//! <tool>(<arg-glob>)
//! ```
//!
//! - `<tool>` is an exact, case-sensitive tool name.
//! - `<arg-glob>` uses **standard shell-glob syntax**: `*` (any chars), `?`
//!   (exactly one char), `[abc]` / `[a-z]` (character sets).
//! - The pattern is anchored to the ENTIRE arg string — the parentheses are
//!   the implicit start/end anchors. There is no implicit prefix/suffix wildcard.
//!
//! Examples: `Bash(rm -rf *)`, `Read(/tmp/?.txt)`, `Bash(git [pf]*)`
//!
//! # Non-features (explicit)
//!
//! - No regex syntax: slashes and backslashes are treated as literal characters.
//!   (A pattern like `Bash(/.+/)` is syntactically valid but matches only the
//!   literal string `/.+/`.)
//! - No negation patterns.
//! - No captures or back-references.
//! - No implicit shell tokenization: the arg is matched as a flat string.
//!   `*` crosses spaces, quotes, and all other characters freely.
//!
//! # Flat-string matching
//!
//! The arg passed to `Pattern::matches` is matched as a single flat string.
//! There is no shell word-splitting, quoting, or escaping — the glob sees
//! exactly the bytes of the arg. For example, `Bash(rm * /tmp)` will match
//! the arg `"rm -rf /tmp"` because `*` spans the intervening characters.

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A compiled permission pattern.
///
/// Created via [`Pattern::parse`].
#[derive(Debug)]
pub struct Pattern {
    /// Original input string, for round-tripping.
    raw: String,
    /// Exact, case-sensitive tool name.
    tool: String,
    /// How to match the argument.
    kind: PatternKind,
}

#[derive(Debug)]
enum PatternKind {
    /// `<tool>:*` — matches any argument.
    Wildcard,
    /// `<tool>(<arg-glob>)` — shell-glob matched against the full arg.
    Glob(glob::Pattern),
}

/// Errors returned by [`Pattern::parse`].
#[derive(Debug, PartialEq, Eq)]
pub enum PatternParseError {
    /// Pattern is not one of the two supported shapes.
    ///
    /// Examples: `Bash` (no parens, no `:*`), `Bash(unclosed`.
    Malformed,
    /// Pattern has no tool name before the `(` or `:`.
    ///
    /// Example: `(arg)`.
    EmptyToolName,
    /// The arg-glob portion contains syntax the underlying glob library rejects.
    InvalidGlob(String),
}

impl std::fmt::Display for PatternParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => write!(
                f,
                "malformed pattern: must be `<tool>:*` or `<tool>(<arg-glob>)`"
            ),
            Self::EmptyToolName => {
                write!(f, "empty tool name: pattern must start with a tool name")
            }
            Self::InvalidGlob(msg) => write!(f, "invalid glob syntax: {msg}"),
        }
    }
}

impl std::error::Error for PatternParseError {}

impl Pattern {
    /// Parse a pattern string into a compiled [`Pattern`].
    ///
    /// # Errors
    ///
    /// Returns [`PatternParseError`] if the input is not a valid pattern.
    pub fn parse(input: &str) -> Result<Pattern, PatternParseError> {
        // Shape 1: `<tool>:*`
        if let Some(colon_pos) = input.find(':') {
            // Only `:*` is valid after the colon (must be last char).
            if &input[colon_pos..] == ":*" {
                let tool = &input[..colon_pos];
                if tool.is_empty() {
                    return Err(PatternParseError::EmptyToolName);
                }
                return Ok(Pattern {
                    raw: input.to_string(),
                    tool: tool.to_string(),
                    kind: PatternKind::Wildcard,
                });
            }
            // Colon present but not `:*` suffix — fall through to check `(`.
        }

        // Shape 2: `<tool>(<arg-glob>)`
        if let Some(open_pos) = input.find('(') {
            let tool = &input[..open_pos];
            if tool.is_empty() {
                return Err(PatternParseError::EmptyToolName);
            }
            // Must end with `)`.
            if !input.ends_with(')') {
                return Err(PatternParseError::Malformed);
            }
            let glob_str = &input[open_pos + 1..input.len() - 1];
            let compiled = glob::Pattern::new(glob_str)
                .map_err(|e| PatternParseError::InvalidGlob(e.to_string()))?;
            return Ok(Pattern {
                raw: input.to_string(),
                tool: tool.to_string(),
                kind: PatternKind::Glob(compiled),
            });
        }

        Err(PatternParseError::Malformed)
    }

    /// Returns `true` if this pattern matches the given `(tool, arg)` pair.
    ///
    /// Tool name matching is case-sensitive and exact. Argument matching
    /// depends on the pattern shape — see the module-level documentation.
    pub fn matches(&self, tool: &str, arg: &str) -> bool {
        if tool != self.tool {
            return false;
        }
        match &self.kind {
            PatternKind::Wildcard => true,
            PatternKind::Glob(pat) => pat.matches(arg),
        }
    }

    /// Returns the original input string, for round-tripping.
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // 1. tool_wildcard_matches_any_arg
    // -----------------------------------------------------------------------

    #[test]
    fn tool_wildcard_matches_any_arg() {
        let p = Pattern::parse("Bash:*").unwrap();
        assert!(
            p.matches("Bash", "ls -la"),
            "Bash:* should match ('Bash', 'ls -la')"
        );
        assert!(
            p.matches("Bash", ""),
            "Bash:* should match ('Bash', '') (empty arg)"
        );
        assert!(
            !p.matches("Read", "/tmp/x"),
            "Bash:* should NOT match ('Read', '/tmp/x')"
        );
    }

    // -----------------------------------------------------------------------
    // 2. arg_glob_basic
    // -----------------------------------------------------------------------

    #[test]
    fn arg_glob_basic() {
        let p = Pattern::parse("Bash(rm -rf *)").unwrap();
        assert!(
            p.matches("Bash", "rm -rf node_modules"),
            "Bash(rm -rf *) should match ('Bash', 'rm -rf node_modules')"
        );
        assert!(
            !p.matches("Bash", "ls"),
            "Bash(rm -rf *) should NOT match ('Bash', 'ls')"
        );
        assert!(
            !p.matches("Read", "rm -rf node_modules"),
            "Bash(rm -rf *) should NOT match ('Read', 'rm -rf node_modules') — tool mismatch"
        );
    }

    // -----------------------------------------------------------------------
    // 3. arg_glob_question_mark
    // -----------------------------------------------------------------------

    #[test]
    fn arg_glob_question_mark() {
        let p = Pattern::parse("Read(/tmp/?.txt)").unwrap();
        assert!(
            p.matches("Read", "/tmp/a.txt"),
            "Read(/tmp/?.txt) should match ('Read', '/tmp/a.txt')"
        );
        assert!(
            !p.matches("Read", "/tmp/ab.txt"),
            "Read(/tmp/?.txt) should NOT match ('Read', '/tmp/ab.txt') — ? is exactly 1 char"
        );
        assert!(
            !p.matches("Read", "/tmp/.txt"),
            "Read(/tmp/?.txt) should NOT match ('Read', '/tmp/.txt') — no char in ? position"
        );
    }

    // -----------------------------------------------------------------------
    // 4. arg_glob_charset
    // -----------------------------------------------------------------------

    #[test]
    fn arg_glob_charset() {
        let p = Pattern::parse("Bash(git [pf]*)").unwrap();
        assert!(
            p.matches("Bash", "git push"),
            "Bash(git [pf]*) should match ('Bash', 'git push')"
        );
        assert!(
            p.matches("Bash", "git fetch"),
            "Bash(git [pf]*) should match ('Bash', 'git fetch')"
        );
        assert!(
            !p.matches("Bash", "git status"),
            "Bash(git [pf]*) should NOT match ('Bash', 'git status')"
        );
    }

    // -----------------------------------------------------------------------
    // 5. arg_glob_no_shell_tokenization
    // -----------------------------------------------------------------------

    #[test]
    fn arg_glob_no_shell_tokenization() {
        // The arg is matched as a FLAT string — the glob matcher sees one string
        // with no shell tokenization, quote processing, or word-splitting.
        // `*` in the glob spans spaces and any other characters freely.
        // `Bash(rm * /tmp)` matches `"rm -rf /tmp"` because the `*` in the glob
        // spans `"-rf "`.
        let p = Pattern::parse("Bash(rm * /tmp)").unwrap();
        assert!(
            p.matches("Bash", "rm -rf /tmp"),
            "Bash(rm * /tmp) should match ('Bash', 'rm -rf /tmp') via flat-string glob — * spans spaces"
        );
    }

    // -----------------------------------------------------------------------
    // 6. unknown_pattern_shape_errors
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_pattern_shape_errors() {
        // No parens, no `:*` — Malformed.
        assert_eq!(
            Pattern::parse("Bash").unwrap_err(),
            PatternParseError::Malformed,
            "Bash (no parens, no :*) should be Malformed"
        );

        // Unclosed paren — Malformed.
        assert_eq!(
            Pattern::parse("Bash(unclosed").unwrap_err(),
            PatternParseError::Malformed,
            "Bash(unclosed should be Malformed (missing close paren)"
        );

        // No tool name — EmptyToolName.
        assert_eq!(
            Pattern::parse("(arg)").unwrap_err(),
            PatternParseError::EmptyToolName,
            "(arg) with no tool name should be EmptyToolName"
        );

        // Slashes are LITERAL characters — Bash(/.*/) PARSES successfully.
        // It matches only the exact arg string "/.*/" and nothing else.
        // This is INTENTIONAL per the P.1.3 plan: the old heuristic falsely
        // rejected URL-containing patterns; slashes are now treated as literals.
        let p = Pattern::parse("Bash(/.*/)")
            .expect("Bash(/.*/) should parse successfully — slashes are literal chars");
        assert!(
            p.matches("Bash", "/.*/"),
            "Bash(/.*/) should match ('Bash', '/.*/')"
        );
        assert!(
            !p.matches("Bash", "anything else"),
            "Bash(/.*/) should NOT match ('Bash', 'anything else')"
        );
    }

    // -----------------------------------------------------------------------
    // 7. tool_name_is_case_sensitive
    // -----------------------------------------------------------------------

    #[test]
    fn tool_name_is_case_sensitive() {
        let lower = Pattern::parse("bash:*").unwrap();
        assert!(
            !lower.matches("Bash", "ls"),
            "bash:* should NOT match ('Bash', 'ls') — case-sensitive"
        );

        let upper = Pattern::parse("Bash:*").unwrap();
        assert!(
            !upper.matches("bash", "ls"),
            "Bash:* should NOT match ('bash', 'ls') — case-sensitive"
        );
    }

    // -----------------------------------------------------------------------
    // 8. (bonus) empty_arg_glob_matches_only_empty_arg
    // -----------------------------------------------------------------------

    #[test]
    fn empty_arg_glob_matches_only_empty_arg() {
        let p = Pattern::parse("Read()").unwrap();
        assert!(
            p.matches("Read", ""),
            "Read() should match ('Read', '') — empty glob matches empty arg"
        );
        assert!(
            !p.matches("Read", "x"),
            "Read() should NOT match ('Read', 'x')"
        );
    }

    // -----------------------------------------------------------------------
    // 9. (bonus) round_trip_via_raw
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_via_raw() {
        let p = Pattern::parse("Bash(rm -rf *)").unwrap();
        assert_eq!(p.raw(), "Bash(rm -rf *)");
    }
}
