use std::collections::HashMap;

/// Errors that can occur during settings resolution.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SettingsError {
    /// A TOML key exists but holds a value of the wrong type.
    #[error("settings key '{key}' has wrong type: expected {expected}, got {actual}")]
    WrongType {
        key: String,
        expected: &'static str,
        actual: &'static str,
    },
}

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
    ///
    /// Per spec §14.2 for the three `AGENTIC_*` keys; UI theme gets
    /// `AGENTIC_THEME` for consistency.
    pub fn env_var(self) -> &'static str {
        match self {
            Self::DefaultsProfile => "AGENTIC_PROFILE",
            Self::DefaultsBackend => "AGENTIC_BACKEND",
            Self::DefaultsModel => "AGENTIC_MODEL",
            Self::UiTheme => "AGENTIC_THEME",
        }
    }

    /// TOML dotted path as `(section, field)`.
    ///
    /// For example, `("defaults", "profile")` maps to `[defaults]\nprofile = "..."`.
    pub fn toml_path(self) -> (&'static str, &'static str) {
        match self {
            Self::DefaultsProfile => ("defaults", "profile"),
            Self::DefaultsBackend => ("defaults", "backend"),
            Self::DefaultsModel => ("defaults", "model"),
            Self::UiTheme => ("ui", "theme"),
        }
    }
}

/// Indicates where a resolved setting value originated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Value came from an environment variable. The variable name is included
    /// so the UI can show e.g. "Source: env: AGENTIC_PROFILE".
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
///
/// Allows tests to inject a fake environment without touching the real
/// process environment.
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
        Self(HashMap::new())
    }

    /// Insert a key-value pair and return `self` for chaining.
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.0.insert(key.to_string(), value.to_string());
        self
    }
}

impl Default for MockEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvProvider for MockEnv {
    fn get(&self, var: &str) -> Option<String> {
        self.0.get(var).cloned()
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
        Self {
            env,
            workspace,
            user,
            defaults,
        }
    }

    /// Resolve `key` through: env var → workspace TOML → user TOML → default.
    ///
    /// Returns `Ok(None)` if no source yields a value. Returns
    /// `Err(SettingsError::WrongType)` if a TOML source contains the key but
    /// with a non-string value — callers can distinguish "missing" from
    /// "misconfigured".
    pub fn resolve(&self, key: Key) -> Result<Option<Setting<String>>, SettingsError> {
        // 1. Env
        if let Some(value) = self.env.get(key.env_var()) {
            return Ok(Some(Setting {
                value,
                source: Source::Env { var: key.env_var() },
            }));
        }
        // 2. Workspace TOML
        if let Some(ws) = &self.workspace
            && let Some(value) = lookup_in_toml(ws, key)?
        {
            return Ok(Some(Setting {
                value,
                source: Source::Workspace,
            }));
        }
        // 3. User TOML
        if let Some(user) = &self.user
            && let Some(value) = lookup_in_toml(user, key)?
        {
            return Ok(Some(Setting {
                value,
                source: Source::User,
            }));
        }
        // 4. Default
        if let Some(value) = self.defaults.get(&key) {
            return Ok(Some(Setting {
                value: value.clone(),
                source: Source::Default,
            }));
        }
        Ok(None)
    }
}

fn toml_type_name(v: &toml::Value) -> &'static str {
    match v {
        toml::Value::String(_) => "string",
        toml::Value::Integer(_) => "integer",
        toml::Value::Float(_) => "float",
        toml::Value::Boolean(_) => "boolean",
        toml::Value::Datetime(_) => "datetime",
        toml::Value::Array(_) => "array",
        toml::Value::Table(_) => "table",
    }
}

fn lookup_in_toml(table: &toml::Table, key: Key) -> Result<Option<String>, SettingsError> {
    let (section, field) = key.toml_path();
    let Some(section_val) = table.get(section) else {
        return Ok(None);
    };
    let Some(field_val) = section_val.as_table().and_then(|t| t.get(field)) else {
        return Ok(None);
    };
    match field_val {
        toml::Value::String(s) => Ok(Some(s.clone())),
        other => Err(SettingsError::WrongType {
            key: format!("{section}.{field}"),
            expected: "string",
            actual: toml_type_name(other),
        }),
    }
}
