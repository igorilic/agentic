//! Pane-level renderers. Each module here is a `pub fn render(...)`
//! that takes a sub-rect plus the relevant slice of `AppState` and
//! draws into the frame.

pub mod chat_pane;
pub mod diff;
pub mod findings;
pub mod help_overlay;
pub mod issue_header;
pub mod issue_pane;
pub mod logs_pane;
pub mod perm_card;
pub mod pipeline_bar;
pub mod status_line;
pub mod tab_bar;
pub mod title_bar;
