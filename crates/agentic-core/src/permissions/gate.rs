//! Synchronous permission gate for `PermissionsConfig`-driven decisions.
//!
//! # Role
//!
//! [`ConfigGate`] is the first layer of the permission system. It evaluates
//! every `(tool, arg)` pair against the loaded [`PermissionsConfig`] and
//! returns a [`GateOutcome`] synchronously.
//!
//! # What this module is NOT
//!
//! - No async. P.2.2 adds `evaluate_async` + a decision channel for the
//!   `Prompt` branch.
//! - No session allowlist. P.2.3 adds in-memory `AllowSession` tracking.
//! - No orchestrator wiring. P.2.4 connects the gate to `Event::ToolUseStart`.

use crate::events::{PermissionRisk, PermissionSource};
use crate::permissions::config::{PermissionsConfig, PermissionsSettings};
use crate::permissions::matcher::Pattern;
use crate::permissions::risk;

// ---------------------------------------------------------------------------
// Public types (stubs — GREEN phase fills in the implementation)
// ---------------------------------------------------------------------------

/// The outcome of a permission gate evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// The `(tool, arg)` matched an allowlist rule — proceed without prompting.
    AnnotateAllow { source: PermissionSource },
    /// The `(tool, arg)` matched a denylist rule — block without prompting.
    AnnotateDeny { source: PermissionSource },
    /// Neither list matched — caller should present a user prompt.
    Prompt { risk: PermissionRisk },
}

/// Synchronous permission gate trait.
pub trait PermissionGate {
    fn evaluate(&self, tool: &str, arg: &str) -> GateOutcome;
}

/// A static gate backed by [`PermissionsConfig`].
pub struct ConfigGate {
    allow: Vec<Pattern>,
    deny: Vec<Pattern>,
    #[allow(dead_code)]
    settings: PermissionsSettings,
}

impl ConfigGate {
    /// Construct a gate from a validated [`PermissionsConfig`].
    ///
    /// # Panics
    ///
    /// Panics if any pattern string fails to parse. This should never happen
    /// because `PermissionsConfig::load` validates all patterns at load time.
    /// A panic here indicates an internal invariant violation.
    pub fn new(config: PermissionsConfig) -> Self {
        let allow = config
            .allowlist
            .into_iter()
            .map(|rule| {
                Pattern::parse(&rule.pattern).expect("config patterns validated at load time")
            })
            .collect();

        let deny = config
            .denylist
            .into_iter()
            .map(|rule| {
                Pattern::parse(&rule.pattern).expect("config patterns validated at load time")
            })
            .collect();

        Self {
            allow,
            deny,
            settings: config.settings,
        }
    }
}

impl PermissionGate for ConfigGate {
    /// Evaluate a `(tool, arg)` pair against the config.
    ///
    /// Priority order:
    /// 1. **Denylist** — any match → `AnnotateDeny { DenylistConfig }`.
    /// 2. **Allowlist** — any match → `AnnotateAllow { AllowlistConfig }`.
    /// 3. **Neither** → `Prompt { risk }` where risk comes from the classifier.
    fn evaluate(&self, tool: &str, arg: &str) -> GateOutcome {
        // 1. Denylist takes precedence over allowlist.
        for pattern in &self.deny {
            if pattern.matches(tool, arg) {
                return GateOutcome::AnnotateDeny {
                    source: PermissionSource::DenylistConfig,
                };
            }
        }

        // 2. Allowlist.
        for pattern in &self.allow {
            if pattern.matches(tool, arg) {
                return GateOutcome::AnnotateAllow {
                    source: PermissionSource::AllowlistConfig,
                };
            }
        }

        // 3. Neither matched — classify risk and ask the user.
        GateOutcome::Prompt {
            risk: risk::classify(tool, arg),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{PermissionRisk, PermissionSource};
    use crate::permissions::config::{PermissionRule, PermissionsConfig, PermissionsSettings};

    fn cfg_with(allow: &[&str], deny: &[&str]) -> PermissionsConfig {
        PermissionsConfig {
            allowlist: allow
                .iter()
                .map(|s| PermissionRule {
                    pattern: (*s).into(),
                })
                .collect(),
            denylist: deny
                .iter()
                .map(|s| PermissionRule {
                    pattern: (*s).into(),
                })
                .collect(),
            settings: PermissionsSettings::default(),
        }
    }

    // 1. allowlist_hit_returns_allow_once
    #[test]
    fn allowlist_hit_returns_allow_once() {
        let gate = ConfigGate::new(cfg_with(&["Read(*)"], &[]));
        assert_eq!(
            gate.evaluate("Read", "/tmp/x"),
            GateOutcome::AnnotateAllow {
                source: PermissionSource::AllowlistConfig
            },
            "Read(*) allowlisted — evaluate should return AnnotateAllow"
        );
    }

    // 2. denylist_hit_returns_deny
    #[test]
    fn denylist_hit_returns_deny() {
        let gate = ConfigGate::new(cfg_with(&[], &["Bash(rm -rf *)"]));
        assert_eq!(
            gate.evaluate("Bash", "rm -rf node_modules"),
            GateOutcome::AnnotateDeny {
                source: PermissionSource::DenylistConfig
            },
            "Bash(rm -rf *) denylisted — evaluate should return AnnotateDeny"
        );
    }

    // 3. unknown_tool_returns_prompt_with_low_risk
    #[test]
    fn unknown_tool_returns_prompt_with_low_risk() {
        let gate = ConfigGate::new(cfg_with(&[], &[]));
        assert_eq!(
            gate.evaluate("CustomTool", "x"),
            GateOutcome::Prompt {
                risk: PermissionRisk::Low
            },
            "unknown tool with no matching rules should return Prompt with Low risk"
        );
    }

    // 4. unknown_bash_returns_prompt_with_medium_risk
    #[test]
    fn unknown_bash_returns_prompt_with_medium_risk() {
        let gate = ConfigGate::new(cfg_with(&[], &[]));
        assert_eq!(
            gate.evaluate("Bash", "echo hello"),
            GateOutcome::Prompt {
                risk: PermissionRisk::Medium
            },
            "Bash with no matching rules should return Prompt with Medium risk"
        );
    }

    // 5. unknown_high_risk_bash_returns_prompt_with_high_risk
    #[test]
    fn unknown_high_risk_bash_returns_prompt_with_high_risk() {
        let gate = ConfigGate::new(cfg_with(&[], &[]));
        assert_eq!(
            gate.evaluate("Bash", "sudo apt update"),
            GateOutcome::Prompt {
                risk: PermissionRisk::High
            },
            "Bash sudo with no matching rules should return Prompt with High risk"
        );
    }

    // 6. denylist_takes_precedence_over_allowlist
    #[test]
    fn denylist_takes_precedence_over_allowlist() {
        let gate = ConfigGate::new(cfg_with(&["Bash(*)"], &["Bash(rm -rf *)"]));
        assert_eq!(
            gate.evaluate("Bash", "rm -rf foo"),
            GateOutcome::AnnotateDeny {
                source: PermissionSource::DenylistConfig
            },
            "denylist must win over allowlist when both patterns match"
        );
    }

    // 7. unrelated_tool_with_allow_returns_prompt
    #[test]
    fn unrelated_tool_with_allow_returns_prompt() {
        let gate = ConfigGate::new(cfg_with(&["Read(*)"], &[]));
        assert_eq!(
            gate.evaluate("Bash", "ls"),
            GateOutcome::Prompt {
                risk: PermissionRisk::Medium
            },
            "Read(*) allowlist does not match Bash — should fall through to Prompt"
        );
    }

    // 8. multiple_allow_patterns_first_match_wins_for_source_only
    #[test]
    fn multiple_allow_patterns_first_match_wins_for_source_only() {
        let gate = ConfigGate::new(cfg_with(&["Read(*)", "Read(/tmp/*)"], &[]));
        // Both patterns match /tmp/x — result must be AnnotateAllow with AllowlistConfig.
        assert_eq!(
            gate.evaluate("Read", "/tmp/x"),
            GateOutcome::AnnotateAllow {
                source: PermissionSource::AllowlistConfig
            },
            "multiple overlapping allow patterns — first match wins, same source"
        );
    }
}
