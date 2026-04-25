use std::collections::HashMap;

use agentic_core::settings::{Key, MockEnv, Resolver, Setting, SettingsError, Source};

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
    let Setting { value, source } = r
        .resolve(Key::UiTheme)
        .expect("no type error")
        .expect("resolved");
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
    let Setting { value, source } = r
        .resolve(Key::UiTheme)
        .expect("no type error")
        .expect("resolved");
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
    let Setting { value, source } = r
        .resolve(Key::UiTheme)
        .expect("no type error")
        .expect("resolved");
    assert_eq!(value, "dark");
    assert_eq!(source, Source::User);
}

#[test]
fn default_is_used_when_no_other_source_has_the_key() {
    let env = MockEnv::new();
    let r = Resolver::new(env, None, None, defaults_with_theme("auto"));
    let Setting { value, source } = r
        .resolve(Key::UiTheme)
        .expect("no type error")
        .expect("resolved");
    assert_eq!(value, "auto");
    assert_eq!(source, Source::Default);
}

#[test]
fn missing_key_without_default_returns_none() {
    let env = MockEnv::new();
    let r = Resolver::new(env, None, None, HashMap::new());
    assert!(r.resolve(Key::UiTheme).expect("no type error").is_none());
}

#[test]
fn resolver_errors_on_non_string_value_for_string_key() {
    // The workspace TOML has [ui] theme as an integer, not a string.
    let workspace = parse_toml(
        r#"[ui]
theme = 8080
"#,
    );
    let r = Resolver::new(MockEnv::new(), Some(workspace), None, HashMap::new());
    let result = r.resolve(Key::UiTheme);
    assert!(
        matches!(result, Err(SettingsError::WrongType { .. })),
        "expected WrongType error, got: {result:?}"
    );
    if let Err(SettingsError::WrongType { actual, .. }) = result {
        assert_eq!(actual, "integer");
    }
}

#[test]
fn resolver_returns_none_for_missing_key() {
    let r = Resolver::new(
        MockEnv::new(),
        Some(toml::Table::new()),
        None,
        HashMap::new(),
    );
    assert!(
        matches!(r.resolve(Key::UiTheme), Ok(None)),
        "expected Ok(None) for missing key"
    );
}

#[test]
fn every_key_variant_has_expected_env_var_and_toml_path() {
    // Parametric regression guard: catches copy-paste errors in Key::env_var
    // and Key::toml_path for any variant. Expand this table when adding new
    // keys.
    let table: &[(Key, &str, (&str, &str))] = &[
        (
            Key::DefaultsProfile,
            "AGENTIC_PROFILE",
            ("defaults", "profile"),
        ),
        (
            Key::DefaultsBackend,
            "AGENTIC_BACKEND",
            ("defaults", "backend"),
        ),
        (Key::DefaultsModel, "AGENTIC_MODEL", ("defaults", "model")),
        (Key::UiTheme, "AGENTIC_THEME", ("ui", "theme")),
    ];
    for (key, expected_env_var, expected_toml_path) in table {
        assert_eq!(key.env_var(), *expected_env_var, "{key:?} env_var mismatch");
        assert_eq!(
            key.toml_path(),
            *expected_toml_path,
            "{key:?} toml_path mismatch"
        );
    }
}
