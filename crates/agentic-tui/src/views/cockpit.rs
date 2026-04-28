//! Cockpit-pane renderer. Step 12.3: a four-row stepper with a
//! per-row icon and the agent's name. Mirrors the visual contract of
//! `apps/web-ui/src/components/Stepper.tsx` but stacked vertically —
//! more idiomatic for a TTY column.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{AppState, Pane};
use crate::run::StepRunStatus;
use crate::views::findings;

pub fn render(area: Rect, state: &AppState, frame: &mut Frame<'_>) {
    let title = if state.focus == Pane::Cockpit {
        "Cockpit *"
    } else {
        "Cockpit"
    };
    let title_style = if state.focus == Pane::Cockpit {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL);

    // Render the bordered block once, then carve its inner area into a
    // stepper section (4 rows for the canonical agents) and a findings
    // section that takes the rest. A blank divider row keeps the two
    // visually separated without drawing a second border.
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(state.run.steps.len() as u16),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let stepper_lines: Vec<Line<'_>> = state
        .run
        .steps
        .iter()
        .map(|row| {
            let style = status_style(row.status);
            Line::from(vec![
                Span::styled(format!("{} ", row.status.icon()), style),
                Span::raw(row.agent.clone()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(stepper_lines), chunks[0]);
    findings::render(chunks[2], state, frame);
}

fn status_style(status: StepRunStatus) -> Style {
    match status {
        StepRunStatus::Pending => Style::default().fg(Color::DarkGray),
        StepRunStatus::Running => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
        StepRunStatus::Passed => Style::default().fg(Color::Green),
        StepRunStatus::Failed => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        StepRunStatus::NeedsTriage => Style::default().fg(Color::Yellow),
        StepRunStatus::Skipped => Style::default().fg(Color::Yellow),
    }
}
