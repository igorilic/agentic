//! Spec §4.2 title bar — fixed 1-row band at the top of the frame.
//!
//! Decorative traffic lights at columns 0/2/4 (red/amber/green);
//! centered status text `user@host — agentic — {cols}×{rows}` in DIM.

use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme;

const DOT: &str = "●";

/// `user@host` resolved at first call and cached for the process lifetime.
fn user_at_host() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED.get_or_init(|| {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string());
        let host = std::env::var("HOSTNAME")
            .ok()
            .or_else(|| std::env::var("COMPUTERNAME").ok())
            .or_else(|| {
                std::fs::read_to_string("/etc/hostname")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "host".to_string());
        format!("{user}@{host}")
    })
}

/// Render the title bar into `area`. `area.height` should be 1.
pub fn render(area: Rect, f: &mut Frame<'_>) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let bg = Style::default().bg(theme::HEADER_BG);
    let cols = f.area().width;
    let rows = f.area().height;
    let center_text = format!("{} — agentic — {cols}×{rows}", user_at_host());

    // Split the row: 6 cols for the traffic lights, remaining for centered title.
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    let dots = Line::from(vec![
        Span::styled(DOT, Style::default().fg(theme::RED).bg(theme::HEADER_BG)),
        Span::styled(" ", bg),
        Span::styled(DOT, Style::default().fg(theme::YELLOW).bg(theme::HEADER_BG)),
        Span::styled(" ", bg),
        Span::styled(DOT, Style::default().fg(theme::GREEN).bg(theme::HEADER_BG)),
        Span::styled(" ", bg),
    ]);
    f.render_widget(Paragraph::new(dots).style(bg), chunks[0]);

    let title = Paragraph::new(center_text)
        .style(Style::default().fg(theme::DIM).bg(theme::HEADER_BG))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[1]);
}
