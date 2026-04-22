use std::collections::HashMap;

use agentic_core::settings::{Key, MockEnv, Resolver, Setting, Source};

fn parse_toml(s: &str) -> toml::Table {
    toml::from_str(s).expect("valid TOML")
}

fn defaults_with_theme(default_value: &str) -> HashMap<Key, String> {
    let mut m = HashMap::new();
    m.insert(Key::UiTheme, default_value.to_string());
    m
}

#[test]
fn env_wins_over_workspace() {
    let env = MockEnv::new().with("AGENTIC_THEME", "dark");
    let workspace = parse_toml(
        r#"[ui]
theme = "light"
"#,
    );
    let r = Resolver::new(env, Some(workspace), None, defaults_with_theme("auto"));
    let Setting { value, source } = r.resolve(Key::UiTheme).expect("resolved");
    assert_eq!(value, "dark");
    assert_eq!(
        source,
        Source::Env {
            var: "AGENTIC_THEME"
        }
    );
}

#[test]
fn workspace_wins_over_user() {
    let env = MockEnv::new();
    let workspace = parse_toml(
        r#"[ui]
theme = "light"
"#,
    );
    let user = parse_toml(
        r#"[ui]
theme = "dark"
"#,
    );
    let r = Resolver::new(
        env,
        Some(workspace),
        Some(user),
        defaults_with_theme("auto"),
    );
    let Setting { value, source } = r.resolve(Key::UiTheme).expect("resolved");
    assert_eq!(value, "light");
    assert_eq!(source, Source::Workspace);
}

#[test]
fn user_wins_over_default() {
    let env = MockEnv::new();
    let user = parse_toml(
        r#"[ui]
theme = "dark"
"#,
    );
    let r = Resolver::new(env, None, Some(user), defaults_with_theme("auto"));
    let Setting { value, source } = r.resolve(Key::UiTheme).expect("resolved");
    assert_eq!(value, "dark");
    assert_eq!(source, Source::User);
}

#[test]
fn default_is_used_when_no_other_source_has_the_key() {
    let env = MockEnv::new();
    let r = Resolver::new(env, None, None, defaults_with_theme("auto"));
    let Setting { value, source } = r.resolve(Key::UiTheme).expect("resolved");
    assert_eq!(value, "auto");
    assert_eq!(source, Source::Default);
}

#[test]
fn missing_key_without_default_returns_none() {
    let env = MockEnv::new();
    let r = Resolver::new(env, None, None, HashMap::new());
    assert!(r.resolve(Key::UiTheme).is_none());
}
