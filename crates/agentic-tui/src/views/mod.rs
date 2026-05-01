//! Pane-level renderers. Each module here is a `pub fn render(...)`
//! that takes a sub-rect plus the relevant slice of `AppState` and
//! draws into the frame.

pub mod chat;
pub mod chat_pane;
pub mod cockpit;
pub mod diff;
pub mod findings;
pub mod issue_header;
pub mod logs_pane;
pub mod pipeline_bar;
pub mod tab_bar;
pub mod title_bar;
