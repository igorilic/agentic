pub mod device_code;
pub mod gh_delegate;
pub mod loopback;
pub mod oauth_github;
pub mod oauth_gitlab;
pub mod pkce;
pub mod refresh;
pub mod secrets;

pub use device_code::{DeviceAuthorization, DeviceCodeClient, DeviceCodeError};
pub use gh_delegate::{GhDelegate, GhDelegateError};
pub use loopback::{CallbackQuery, LoopbackError, LoopbackListener, start as start_loopback};
pub use oauth_github::{AccessToken, GithubOauthClient, GithubOauthError, validate_state};
pub use oauth_gitlab::{GitlabOauthClient, GitlabOauthError};
pub use pkce::{PkceChallenge, generate_state};
pub use refresh::{
    AccountStatus, GithubRefreshStrategy, RefreshError, RefreshScheduler, RefreshStrategy,
};
#[cfg(any(test, feature = "testing"))]
pub use secrets::MemSecretStore;
pub use secrets::{KeyringSecretStore, SecretStore, SecretStoreError};
