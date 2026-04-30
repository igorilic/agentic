//! TUI dark-only color palette. Matches spec §4.1 (`docs/redesign/spec.md`).
//!
//! ```
//! use agentic_tui::theme;
//! use ratatui::style::Color;
//! assert_eq!(theme::ACCENT, Color::Rgb(0x5e, 0xea, 0xd4));
//! ```

use ratatui::style::Color;

pub const BG: Color = Color::Rgb(0x0d, 0x0e, 0x10);
pub const FG: Color = Color::Rgb(0xe6, 0xe6, 0xe6);
pub const DIM: Color = Color::Rgb(0x7d, 0x7d, 0x8a);
pub const BORDER: Color = Color::Rgb(0x2a, 0x2b, 0x30);
pub const ACCENT: Color = Color::Rgb(0x5e, 0xea, 0xd4);
pub const BLUE: Color = Color::Rgb(0x7d, 0xd3, 0xfc);
pub const YELLOW: Color = Color::Rgb(0xfd, 0xe6, 0x8a);
pub const GREEN: Color = Color::Rgb(0xa7, 0xf3, 0xd0);
pub const RED: Color = Color::Rgb(0xfc, 0xa5, 0xa5);
pub const PURPLE: Color = Color::Rgb(0xc4, 0xb5, 0xfd);
pub const HEADER_BG: Color = Color::Rgb(0x16, 0x17, 0x1b);
