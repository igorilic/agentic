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
/// Only shell-glob characters (`*` and `?`) are supported. Patterns containing
/// regex syntax are rejected:
/// - Slash-delimited regex:  a segment like `/.../` inside the parens is invalid.
/// - Backslash-escaped char classes: `\d`, `\w`, `\s`, etc. are invalid.
///
/// Matching logic lives in P.1.3; this module is the config carrier only.
use std::path::Path;

use serde::{Deserialize, Serialize};

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

/// A single permission rule containing a glob pattern string.
///
/// The pattern syntax is:
/// - Tool name (exact or `*`) followed by optional `(<arg-glob>)`.
/// - Examples: `Read(*)`, `Bash(cargo:*)`, `Bash(rm -rf /*)`.
/// - Regex syntax (slash-delimited or `\`-escaped) is rejected on load.
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
    /// A pattern contains unsupported regex syntax.
    ///
    /// Rejected patterns: slash-delimited (`/pattern/`) or backslash-escaped
    /// char classes (`\d`, `\w`, `\s`, etc.). Shell-glob chars `*` and `?`
    /// are fine.
    InvalidPattern(String),
}

impl std::fmt::Display for PermissionsConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "permissions.toml I/O error: {e}"),
            Self::Parse(e) => write!(f, "permissions.toml parse error: {e}"),
            Self::InvalidPattern(p) => {
                write!(f, "invalid pattern (regex syntax not supported): {p}")
            }
        }
    }
}

impl std::error::Error for PermissionsConfigError {}

// ---------------------------------------------------------------------------
// TOML wire shapes for `[allowlist]` / `[denylist]` sections
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
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
    /// Source: `crates/agentic-core/src/pipeline/tool_use_observer.rs:113`
    ///         `crates/agentic-core/src/backends/claude_code/mod.rs:378`
    ///
    /// # Copilot CLI tool names
    /// Tool names are lowercase as emitted by Copilot's JSON stream.
    /// Source: `crates/agentic-core/src/backends/copilot_cli/parser.rs`
    ///         `crates/agentic-core/tests/fixtures/copilot/tool_use_bash.jsonl`
    ///         `crates/agentic-core/src/pipeline/tool_use_observer.rs:193`
    pub fn builtin_default() -> Self {
        let allowlist = vec![
            // --- Claude Code: read-only / navigation tools ---
            // from: crates/agentic-core/src/backends/claude_code/mod.rs:378
            rule("Read(*)"),
            // from: crates/agentic-core/src/backends/claude_code/mod.rs:378
            rule("Edit(*)"),
            // from: crates/agentic-core/src/backends/claude_code/mod.rs:378
            rule("Bash(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:114
            rule("Write(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:114
            rule("MultiEdit(*)"),
            // Claude navigation tools (PascalCase) — verified from claude parser ToolUse blocks
            rule("LS(*)"),
            rule("Grep(*)"),
            rule("Glob(*)"),
            // --- Copilot CLI: read-only / navigation tools (lowercase) ---
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:205 ("view")
            rule("view(*)"),
            // from: step description P.1.2 / copilot tool baseline
            rule("ls(*)"),
            rule("grep(*)"),
            rule("find(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:195 ("create")
            rule("create(*)"),
            // from: crates/agentic-core/src/pipeline/tool_use_observer.rs:196 ("str_replace")
            rule("str_replace(*)"),
            // from: crates/agentic-core/tests/fixtures/copilot/tool_use_bash.jsonl:15 ("bash")
            rule("bash(*)"),
        ];

        let denylist = vec![
            // High-risk shell destructor patterns — all backends
            rule("Bash(rm -rf /*)"),
            rule("bash(rm -rf /*)"),
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

    fn validate_patterns(&self) -> Result<(), PermissionsConfigError> {
        for rule in self.allowlist.iter().chain(self.denylist.iter()) {
            if pattern_has_regex_syntax(&rule.pattern) {
                return Err(PermissionsConfigError::InvalidPattern(
                    rule.pattern.clone(),
                ));
            }
        }
        Ok(())
    }
}

/// Returns `true` if `pattern` contains regex-like syntax that v1 does not support.
///
/// Rejection rules:
/// 1. The argument portion (inside parentheses, or the whole string) starts and
///    ends with `/` — this is the slash-delimited regex convention.
/// 2. The pattern contains a backslash followed by a letter (e.g. `\d`, `\w`,
///    `\s`, `\b`) — these are regex escape sequences.
///
/// Shell-glob characters `*` and `?` are NOT rejected.
fn pattern_has_regex_syntax(pattern: &str) -> bool {
    // Rule 2: backslash followed by any ASCII letter.
    if pattern.contains('\\')
        && pattern
            .chars()
            .zip(pattern.chars().skip(1))
            .any(|(a, b)| a == '\\' && b.is_ascii_alphabetic())
    {
        return true;
    }

    // Rule 1: argument portion (inside parens) is slash-delimited.
    // Look for `(/.../)` anywhere in the pattern.
    if let Some(open) = pattern.find('(') {
        let args = &pattern[open + 1..];
        // Strip trailing `)` if present.
        let args = args.trim_end_matches(')');
        let args = args.trim();
        if args.starts_with('/') && args.ends_with('/') && args.len() >= 2 {
            return true;
        }
    }

    false
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

        // Spot-check denylist
        let has_rm = cfg.denylist.iter().any(|r| r.pattern == "Bash(rm -rf /*)");
        assert!(has_rm, "denylist should contain Bash(rm -rf /*)");

        // Default timeout policy
        assert_eq!(cfg.settings.default_on_timeout, OnTimeout::Deny);
    }

    // -----------------------------------------------------------------------
    // 3. rejects_invalid_pattern — slash-delimited regex
    // -----------------------------------------------------------------------

    #[test]
    fn rejects_slash_delimited_regex_pattern() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = ["Bash(/.*/)"  ]

[denylist]
patterns = []
"#,
        );

        let err = PermissionsConfig::load(&path).expect_err("Bash(/.*/) should be rejected");
        assert!(
            matches!(err, PermissionsConfigError::InvalidPattern(_)),
            "expected InvalidPattern, got: {err}"
        );
    }

    #[test]
    fn rejects_backslash_escaped_class_pattern() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[allowlist]
patterns = []

[denylist]
patterns = ["Bash(\\d+)"]
"#,
        );

        let err =
            PermissionsConfig::load(&path).expect_err("Bash(\\d+) should be rejected as regex");
        assert!(
            matches!(err, PermissionsConfigError::InvalidPattern(_)),
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
