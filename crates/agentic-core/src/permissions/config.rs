/// Permissions config carrier loaded from `permissions.toml`.
///
/// # TOML schema
///
/// ```toml
/// [allowlist]
/// patterns = ["Read(*)", "LS(*)", "bash(cargo:*)"]
///
/// [denylist]
/// patterns = ["Bash(rm -rf /*)", "Bash(sudo *)"]
///
/// [settings]
/// default_on_timeout = "deny"   # or "allow"; missing → "deny"
/// ```
///
/// # Pattern syntax (v1)
///
/// Two shapes are supported:
/// - `<tool>:*` — matches any argument for the named tool.
/// - `<tool>(<arg-glob>)` — matches using shell-glob syntax (`*`, `?`, `[abc]`)
///   against the entire argument string.
///
/// Tool names are case-sensitive. Slashes and backslashes are literal characters
/// (no regex syntax is supported). Matching logic lives in `permissions::matcher`.
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::permissions::matcher::{Pattern, PatternParseError};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Top-level permissions configuration loaded from `permissions.toml`.
///
/// # Fields
/// - `allowlist`: rules whose patterns are approved (matcher in P.1.3)
/// - `denylist`:  rules whose patterns are blocked  (matcher in P.1.3)
/// - `settings`:  global knobs (e.g. what to do on approval timeout)
///
/// `allowlist` and `denylist` are `Vec<PermissionRule>` — matcher in P.1.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionsConfig {
    /// Patterns that are unconditionally allowed. // matcher in P.1.3
    #[serde(default)]
    pub allowlist: Vec<PermissionRule>,
    /// Patterns that are unconditionally denied. // matcher in P.1.3
    #[serde(default)]
    pub denylist: Vec<PermissionRule>,
    /// Global settings.
    #[serde(default)]
    pub settings: PermissionsSettings,
}

/// A single permission rule containing a pattern string.
///
/// Pattern syntax (see `permissions/matcher.rs` for the grammar):
/// - `<tool>:*` — matches any argument for the named tool.
/// - `<tool>(<arg-glob>)` — shell-glob (`*`, `?`, `[abc]`) on the full arg.
/// - Examples: `Read(*)`, `Bash(rm -rf /*)`, `Bash:*`.
/// - Tool names are case-sensitive. Slashes and backslashes are literal arg chars.
/// - Patterns are validated by `Pattern::parse` at config-load time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRule {
    pub pattern: String,
}

/// Global settings block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionsSettings {
    /// What to do when a user does not respond to a permission prompt in time.
    #[serde(default)]
    pub default_on_timeout: OnTimeout,
}

impl Default for PermissionsSettings {
    fn default() -> Self {
        Self {
            default_on_timeout: OnTimeout::Deny,
        }
    }
}

/// Action to take when an interactive permission prompt times out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnTimeout {
    /// Let the tool call through (permissive).
    Allow,
    /// Block the tool call (safe default).
    #[default]
    Deny,
}

/// Errors returned by [`PermissionsConfig::load`].
#[derive(Debug)]
pub enum PermissionsConfigError {
    /// I/O error reading the config file.
    Io(std::io::Error),
    /// TOML parse error.
    Parse(toml::de::Error),
    /// A pattern string could not be compiled by the tool matcher.
    ///
    /// The inner [`PatternParseError`] gives the specific reason: `Malformed`
    /// (unrecognised shape), `EmptyToolName`, or `InvalidGlob` (bad glob
    /// syntax). The outer `String` is the raw pattern string from the config
    /// file, for error-message context.
    InvalidPattern(String, PatternParseError),
}

impl std::fmt::Display for PermissionsConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "permissions.toml I/O error: {e}"),
            Self::Parse(e) => write!(f, "permissions.toml parse error: {e}"),
            Self::InvalidPattern(p, reason) => {
                write!(f, "invalid pattern `{p}`: {reason}")
            }
        }
    }
}

impl std::error::Error for PermissionsConfigError {}

// ---------------------------------------------------------------------------
// TOML wire shapes for `[allowlist]` / `[denylist]` sections
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize)]
struct RawSection {
    #[serde(default)]
    patterns: Vec<String>,
}

#[derive(Deserialize)]
struct RawConfig {
    #[serde(default)]
    allowlist: RawSection,
    #[serde(default)]
    denylist: RawSection,
    #[serde(default)]
    settings: PermissionsSettings,
}

// ---------------------------------------------------------------------------
// Impl
// ---------------------------------------------------------------------------

impl PermissionsConfig {
    /// Load from `path`.
    ///
    /// Returns `Ok(builtin_default())` if the file does not exist.
    /// Returns `Err` if the file exists but cannot be read or parsed, or if
    /// any pattern contains unsupported regex syntax.
    pub fn load(path: &Path) -> Result<Self, PermissionsConfigError> {
        if !path.exists() {
            return Ok(Self::builtin_default());
        }
        let raw = std::fs::read_to_string(path).map_err(PermissionsConfigError::Io)?;
        let parsed: RawConfig = toml::from_str(&raw).map_err(PermissionsConfigError::Parse)?;

        let allowlist = parsed
            .allowlist
            .patterns
            .into_iter()
            .map(|p| PermissionRule { pattern: p })
            .collect();
        let denylist = parsed
            .denylist
            .patterns
            .into_iter()
            .map(|p| PermissionRule { pattern: p })
            .collect();

        let config = Self {
            allowlist,
            denylist,
            settings: parsed.settings,
        };
        config.validate_patterns()?;
        Ok(config)
    }

    /// Built-in baseline populated with per-backend tool defaults.
    ///
    /// # Claude Code tool names
    /// Tool names are PascalCase as emitted by the Claude CLI JSON stream.
    /// Verified sources:
    /// - `crates/agentic-core/src/backends/claude_code/mod.rs:378-380` — "Read", "Edit", "Bash"
    /// - `crates/agentic-core/src/pipeline/tool_use_observer.rs:114`   — "Edit", "Write", "MultiEdit"
    /// - `crates/agentic-core/tests/fixtures/agents/architect.md:5`     — "Glob", "Grep"
    ///
    /// # Copilot CLI tool names
    /// Tool names are lowercase as emitted by Copilot's JSON stream.
    /// Verified sources:
    /// - `crates/agentic-core/tests/fixtures/copilot/tool_use_bash.jsonl:15` — "bash"
    /// - `crates/agentic-core/src/pipeline/tool_use_observer.rs:115`          — "create", "str_replace"
    /// - Remaining Copilot names (view, ls, grep, find): unverified — no fixture
    ///   currently exercises them. Assumed lowercase from Copilot tool naming
    ///   convention; verify when a fixture is captured.
    pub fn builtin_default() -> Self {
        let allowlist = vec![
            // --- Claude Code: read-only / navigation tools (PascalCase) ---
            // from: crates/agentic-core/src/backends/claude_code/mod.rs:378
            rule("Read(*)"),
            // from: crates/agentic-core/src/backends/claude_code/mod.rs:379
            rule("Edit(*)"),
            // from: crates/agentic-core/src/backends/claude_code/mod.rs:380
            rule("Bash(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:114
            rule("Write(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:114
            rule("MultiEdit(*)"),
            // from: crates/agentic-core/tests/fixtures/agents/architect.md:5
            rule("Glob(*)"),
            // from: crates/agentic-core/tests/fixtures/agents/architect.md:5
            rule("Grep(*)"),
            // unverified: no fixture shows LS(*); assumed PascalCase from Claude CLI naming
            rule("LS(*)"),
            // --- Copilot CLI: tools (lowercase) ---
            // from: crates/agentic-core/tests/fixtures/copilot/tool_use_bash.jsonl:15
            rule("bash(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:115
            rule("create(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:115
            rule("str_replace(*)"),
            // unverified: no fixture shows view/ls/grep/find; assumed lowercase from Copilot naming
            rule("view(*)"),
            rule("ls(*)"),
            rule("grep(*)"),
            rule("find(*)"),
        ];

        let denylist = vec![
            // High-risk shell destructor patterns — all backends
            // absolute-path variant
            rule("Bash(rm -rf /*)"),
            rule("bash(rm -rf /*)"),
            // home-dir variant (per P.1.2 plan)
            rule("Bash(rm -rf ~*)"),
            rule("bash(rm -rf ~*)"),
            rule("Bash(sudo *)"),
            rule("bash(sudo *)"),
            // Kubernetes irreversible operations
            rule("Bash(kubectl delete *)"),
            rule("bash(kubectl delete *)"),
            // Dangerous git operations
            rule("Bash(git reset --hard *)"),
            rule("bash(git reset --hard *)"),
            rule("Bash(git push --force *)"),
            rule("bash(git push --force *)"),
        ];

        Self {
            allowlist,
            denylist,
            settings: PermissionsSettings::default(),
        }
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate all patterns by attempting to compile them with the real matcher.
    ///
    /// This replaces the P.1.2 heuristic (`pattern_has_regex_syntax`) which
    /// falsely rejected patterns containing slashes (e.g. URL paths). The real
    /// matcher treats slashes as literal characters, so `Bash(/tmp/*)` is valid.
    fn validate_patterns(&self) -> Result<(), PermissionsConfigError> {
        for rule in self.allowlist.iter().chain(self.denylist.iter()) {
            Pattern::parse(&rule.pattern)
                .map_err(|e| PermissionsConfigError::InvalidPattern(rule.pattern.clone(), e))?;
        }
        Ok(())
    }
}

fn rule(pattern: &str) -> PermissionRule {
    PermissionRule {
        pattern: pattern.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    fn write_toml(dir: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
        let path = dir.path().join("permissions.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    // -----------------------------------------------------------------------
    // 1. loads_minimal_config
    // -----------------------------------------------------------------------

    #[test]
    fn loads_minimal_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = ["Read(*)", "LS(*)"]

[denylist]
patterns = ["Bash(rm -rf /*)"]

[settings]
default_on_timeout = "deny"
"#,
        );

        let cfg = PermissionsConfig::load(&path).expect("should parse");
        assert_eq!(cfg.allowlist.len(), 2, "expected 2 allowlist patterns");
        assert_eq!(cfg.denylist.len(), 1, "expected 1 denylist pattern");
        assert_eq!(cfg.allowlist[0].pattern, "Read(*)");
        assert_eq!(cfg.allowlist[1].pattern, "LS(*)");
        assert_eq!(cfg.denylist[0].pattern, "Bash(rm -rf /*)");
        assert_eq!(cfg.settings.default_on_timeout, OnTimeout::Deny);
    }

    // -----------------------------------------------------------------------
    // 2. defaults_when_file_missing
    // -----------------------------------------------------------------------

    #[test]
    fn defaults_when_file_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("permissions.toml"); // does not exist

        let cfg = PermissionsConfig::load(&path).expect("missing file should return default");
        let default = PermissionsConfig::builtin_default();
        assert_eq!(cfg, default);

        // Spot-check Claude baseline
        let has_read = cfg.allowlist.iter().any(|r| r.pattern == "Read(*)");
        assert!(has_read, "allowlist should contain Read(*)");

        // Spot-check Copilot baseline
        let has_view = cfg.allowlist.iter().any(|r| r.pattern == "view(*)");
        assert!(has_view, "allowlist should contain view(*)");

        // Spot-check denylist — absolute-path variant
        let has_rm = cfg.denylist.iter().any(|r| r.pattern == "Bash(rm -rf /*)");
        assert!(has_rm, "denylist should contain Bash(rm -rf /*)");

        // F-2: home-dir variant must also be present
        let has_rm_home = cfg.denylist.iter().any(|r| r.pattern == "Bash(rm -rf ~*)");
        assert!(
            has_rm_home,
            "denylist should contain Bash(rm -rf ~*) per P.1.2 plan"
        );
        let has_rm_home_lower = cfg.denylist.iter().any(|r| r.pattern == "bash(rm -rf ~*)");
        assert!(
            has_rm_home_lower,
            "denylist should contain bash(rm -rf ~*) per P.1.2 plan"
        );

        // Default timeout policy
        assert_eq!(cfg.settings.default_on_timeout, OnTimeout::Deny);
    }

    // -----------------------------------------------------------------------
    // S-4. rejects_invalid_toml_syntax
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_invalid_toml_syntax() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(&dir, "not valid toml ===");
        let err = PermissionsConfig::load(&path).expect_err("malformed toml should error");
        assert!(
            matches!(err, PermissionsConfigError::Parse(_)),
            "expected Parse error for malformed TOML, got: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // 3. rejects_invalid_pattern — truly malformed patterns
    // -----------------------------------------------------------------------

    /// Patterns with slashes are VALID in v1 — slashes are literal characters.
    /// `Bash(/.*/)` now parses successfully and matches the literal arg `/.*/`.
    /// (The old P.1.2 heuristic falsely rejected this; the real matcher does not.)
    #[test]
    fn slash_in_pattern_is_accepted_as_literal() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = ["Bash(/.*/)"]

[denylist]
patterns = []
"#,
        );

        // Must load successfully — slashes are literal chars, not regex.
        let cfg = PermissionsConfig::load(&path)
            .expect("Bash(/.*/) should be accepted — slashes are literal characters");
        assert_eq!(cfg.allowlist.len(), 1);
        assert_eq!(cfg.allowlist[0].pattern, "Bash(/.*/)")
    }

    #[test]
    fn rejects_missing_close_paren_pattern() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = ["Bash(unclosed"]

[denylist]
patterns = []
"#,
        );

        let err = PermissionsConfig::load(&path)
            .expect_err("Bash(unclosed should be rejected — missing close paren");
        assert!(
            matches!(err, PermissionsConfigError::InvalidPattern(_, _)),
            "expected InvalidPattern, got: {err}"
        );
    }

    #[test]
    fn rejects_no_parens_no_wildcard_pattern() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = []

[denylist]
patterns = ["Bash"]
"#,
        );

        let err = PermissionsConfig::load(&path)
            .expect_err("Bash (no parens, no :*) should be rejected as malformed");
        assert!(
            matches!(err, PermissionsConfigError::InvalidPattern(_, _)),
            "expected InvalidPattern, got: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // 4. default_on_timeout_round_trips
    // -----------------------------------------------------------------------

    #[test]
    fn default_on_timeout_allow_round_trips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = []

[denylist]
patterns = []

[settings]
default_on_timeout = "allow"
"#,
        );

        let cfg = PermissionsConfig::load(&path).expect("should parse");
        assert_eq!(cfg.settings.default_on_timeout, OnTimeout::Allow);
    }

    #[test]
    fn default_on_timeout_deny_round_trips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = []

[denylist]
patterns = []

[settings]
default_on_timeout = "deny"
"#,
        );

        let cfg = PermissionsConfig::load(&path).expect("should parse");
        assert_eq!(cfg.settings.default_on_timeout, OnTimeout::Deny);
    }

    #[test]
    fn default_on_timeout_missing_settings_block_defaults_to_deny() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = ["Read(*)"]

[denylist]
patterns = []
"#,
        );

        let cfg = PermissionsConfig::load(&path).expect("should parse without [settings]");
        assert_eq!(
            cfg.settings.default_on_timeout,
            OnTimeout::Deny,
            "missing [settings] should default to Deny"
        );
    }

    // -----------------------------------------------------------------------
    // 5. parses_glob_chars_in_patterns (bonus)
    // -----------------------------------------------------------------------

    #[test]
    fn glob_star_patterns_are_accepted() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = ["bash(cargo:*)", "Read(*)"]

[denylist]
patterns = ["Bash(rm -rf ?ome)"]
"#,
        );

        let cfg = PermissionsConfig::load(&path).expect("glob patterns should be accepted");
        assert_eq!(cfg.allowlist.len(), 2);
        assert_eq!(cfg.denylist.len(), 1);
        assert_eq!(cfg.allowlist[0].pattern, "bash(cargo:*)");
        assert_eq!(cfg.denylist[0].pattern, "Bash(rm -rf ?ome)");
    }
}
