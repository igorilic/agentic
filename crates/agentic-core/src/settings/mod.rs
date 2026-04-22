use std::collections::HashMap;

/// The set of supported configuration keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    DefaultsProfile,
    DefaultsBackend,
    DefaultsModel,
    UiTheme,
}

impl Key {
    /// Environment variable name for this key.
    pub fn env_var(self) -> &'static str {
        unimplemented!()
    }

    /// TOML dotted path as (section, field).
    pub fn toml_path(self) -> (&'static str, &'static str) {
        unimplemented!()
    }
}

/// Indicates where a resolved setting value originated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Value came from an environment variable.
    Env { var: &'static str },
    /// Value came from the workspace `.agentic/config.toml`.
    Workspace,
    /// Value came from the user-global `settings.toml`.
    User,
    /// Compiled-in default.
    Default,
}

/// A resolved setting value together with its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting<T> {
    pub value: T,
    pub source: Source,
}

/// Abstraction over environment variable lookup.
pub trait EnvProvider: Send + Sync {
    fn get(&self, var: &str) -> Option<String>;
}

/// Reads from the real process environment via `std::env::var`.
pub struct RealEnv;

impl EnvProvider for RealEnv {
    fn get(&self, var: &str) -> Option<String> {
        std::env::var(var).ok()
    }
}

/// Test-friendly in-memory environment provider.
pub struct MockEnv(HashMap<String, String>);

impl MockEnv {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn with(self, key: &str, value: &str) -> Self {
        let _ = (key, value);
        unimplemented!()
    }
}

impl Default for MockEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvProvider for MockEnv {
    fn get(&self, var: &str) -> Option<String> {
        let _ = var;
        unimplemented!()
    }
}

/// Resolves a setting key through env → workspace TOML → user TOML → default.
pub struct Resolver<E> {
    env: E,
    workspace: Option<toml::Table>,
    user: Option<toml::Table>,
    defaults: HashMap<Key, String>,
}

impl<E: EnvProvider> Resolver<E> {
    pub fn new(
        env: E,
        workspace: Option<toml::Table>,
        user: Option<toml::Table>,
        defaults: HashMap<Key, String>,
    ) -> Self {
        let _ = (env, workspace, user, defaults);
        unimplemented!()
    }

    /// Resolve `key` through: env var → workspace TOML → user TOML → default.
    /// Returns `None` if no source yields a value.
    pub fn resolve(&self, key: Key) -> Option<Setting<String>> {
        let _ = key;
        unimplemented!()
    }
}
