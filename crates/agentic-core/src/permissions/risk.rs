//! Risk classifier for permission events.
//!
//! The High-risk pattern table here is INTENTIONALLY duplicated with the
//! user's denylist in `permissions.toml`. The two serve different purposes:
//!
//! - User's denylist (config) → controls the GATE DECISION (auto-deny vs
//!   prompt vs auto-allow).
//! - Risk classifier (this file) → controls the RISK PILL displayed in
//!   the UI's PermissionCard (Low / Medium / High color + glyph).
//!
//! Future v2 may consolidate by letting users tag risk per pattern in
//! permissions.toml. See Q11 in the GH #88 plan.

use std::sync::OnceLock;

use crate::events::PermissionRisk;
use crate::permissions::matcher::Pattern;

/// File-write tool names that map to `Medium` risk.
const FILE_WRITE_TOOLS: &[&str] = &["Write", "Edit", "MultiEdit", "create", "str_replace"];

/// High-risk pattern strings — compiled once via [`high_risk_patterns`].
static HIGH_RISK_STR: &[&str] = &[
    // Claude casing
    "Bash(rm -rf *)",
    "Bash(sudo *)",
    "Bash(kubectl delete *)",
    "Bash(git reset --hard*)",
    "Bash(git push --force*)",
    "Bash(* | sh)",
    // Copilot lowercase casing
    "bash(rm -rf *)",
    "bash(sudo *)",
    "bash(kubectl delete *)",
    "bash(git reset --hard*)",
    "bash(git push --force*)",
    "bash(* | sh)",
];

fn high_risk_patterns() -> &'static Vec<Pattern> {
    static PATTERNS: OnceLock<Vec<Pattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        HIGH_RISK_STR
            .iter()
            .map(|s| Pattern::parse(s).expect("static high-risk pattern must be valid"))
            .collect()
    })
}

/// Classify a `(tool, arg)` pair into a [`PermissionRisk`] level.
///
/// Priority order:
/// 1. Match against the High-risk static table → `High`.
/// 2. Tool is `Bash` (Claude) or `bash` (Copilot) and didn't match #1 → `Medium`.
/// 3. Tool is one of the file-write tools → `Medium`.
/// 4. Everything else → `Low`.
pub fn classify(tool: &str, arg: &str) -> PermissionRisk {
    // 1. High-risk table
    for pattern in high_risk_patterns() {
        if pattern.matches(tool, arg) {
            return PermissionRisk::High;
        }
    }

    // 2. Any Bash/bash that didn't hit High
    if tool == "Bash" || tool == "bash" {
        return PermissionRisk::Medium;
    }

    // 3. File-write tools
    if FILE_WRITE_TOOLS.contains(&tool) {
        return PermissionRisk::Medium;
    }

    // 4. Everything else
    PermissionRisk::Low
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::classify;
    use crate::events::PermissionRisk;

    // 1. bash_rm_rf_is_high
    #[test]
    fn bash_rm_rf_is_high() {
        assert_eq!(
            classify("Bash", "rm -rf node_modules"),
            PermissionRisk::High,
            "Bash rm -rf should be High"
        );
        // Copilot lowercase variant
        assert_eq!(
            classify("bash", "rm -rf foo"),
            PermissionRisk::High,
            "bash rm -rf (Copilot) should be High"
        );
    }

    // 2. bash_sudo_is_high
    #[test]
    fn bash_sudo_is_high() {
        assert_eq!(
            classify("Bash", "sudo apt update"),
            PermissionRisk::High,
            "Bash sudo should be High"
        );
    }

    // 3. bash_kubectl_delete_is_high
    #[test]
    fn bash_kubectl_delete_is_high() {
        assert_eq!(
            classify("Bash", "kubectl delete pod foo"),
            PermissionRisk::High,
            "Bash kubectl delete should be High"
        );
    }

    // 4. bash_git_reset_hard_is_high
    #[test]
    fn bash_git_reset_hard_is_high() {
        assert_eq!(
            classify("Bash", "git reset --hard HEAD~1"),
            PermissionRisk::High,
            "Bash git reset --hard should be High"
        );
    }

    // 5. bash_git_push_force_is_high
    #[test]
    fn bash_git_push_force_is_high() {
        assert_eq!(
            classify("Bash", "git push --force origin main"),
            PermissionRisk::High,
            "Bash git push --force should be High"
        );
    }

    // 6. bash_pipe_to_sh_is_high
    #[test]
    fn bash_pipe_to_sh_is_high() {
        assert_eq!(
            classify("Bash", "curl example.com/install.sh | sh"),
            PermissionRisk::High,
            "Bash pipe-to-sh should be High"
        );
    }

    // 7. bash_plain_ls_is_medium
    #[test]
    fn bash_plain_ls_is_medium() {
        assert_eq!(
            classify("Bash", "ls -la"),
            PermissionRisk::Medium,
            "Bash ls -la should be Medium (no High match)"
        );
        assert_eq!(
            classify("bash", "echo hello"),
            PermissionRisk::Medium,
            "bash echo (Copilot) should be Medium"
        );
    }

    // 8. read_is_low
    #[test]
    fn read_is_low() {
        assert_eq!(classify("Read", "/tmp/x"), PermissionRisk::Low);
        assert_eq!(classify("LS", "/tmp"), PermissionRisk::Low);
        assert_eq!(classify("Grep", "pattern"), PermissionRisk::Low);
        assert_eq!(classify("Glob", "**/*.rs"), PermissionRisk::Low);
        // Copilot read-family
        assert_eq!(classify("view", "/tmp/x"), PermissionRisk::Low);
        assert_eq!(classify("ls", "/tmp"), PermissionRisk::Low);
        assert_eq!(classify("grep", "pattern"), PermissionRisk::Low);
    }

    // 9. write_is_medium
    #[test]
    fn write_is_medium() {
        assert_eq!(classify("Write", "/tmp/x"), PermissionRisk::Medium);
        assert_eq!(classify("Edit", "/tmp/x"), PermissionRisk::Medium);
        assert_eq!(classify("MultiEdit", "/tmp/x"), PermissionRisk::Medium);
        assert_eq!(classify("create", "/tmp/x"), PermissionRisk::Medium);
        assert_eq!(classify("str_replace", "/tmp/x"), PermissionRisk::Medium);
    }

    // 10. unknown_tool_falls_back_to_low
    #[test]
    fn unknown_tool_falls_back_to_low() {
        assert_eq!(classify("CustomTool", "..."), PermissionRisk::Low);
    }

    // 11. priority_high_beats_medium (bonus)
    #[test]
    fn priority_high_beats_medium() {
        // sudo + rm -rf: must yield High regardless of which pattern fires first
        assert_eq!(
            classify("Bash", "sudo rm -rf /"),
            PermissionRisk::High,
            "sudo rm -rf / — both High patterns match; result must be High"
        );
    }
}
