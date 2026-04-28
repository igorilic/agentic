//! `agentic-tui` binary entry. Step 12.2: two-pane layout, Tab toggles
//! focus, `[`/`]` resize the cockpit, q/Esc quits.

use std::io;

use agentic_tui::app::{AppEvent, AppState};
use agentic_tui::draw_app;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal);

    // Always restore the terminal, even on error.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut state = AppState::default();
    loop {
        terminal.draw(|f| draw_app(f, &state))?;
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Tab => state.handle(AppEvent::ToggleFocus),
                KeyCode::Char(']') => state.handle(AppEvent::WidenCockpit),
                KeyCode::Char('[') => state.handle(AppEvent::NarrowCockpit),
                _ => {}
            }
        }
    }
}
