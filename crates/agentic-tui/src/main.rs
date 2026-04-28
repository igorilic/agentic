//! `agentic-tui` binary entry. Step 12.4: keys flow through
//! `AppState::handle_key`, which returns optional `AppCommand`s
//! (`:q` quits; `:plan <ticket>` and `:status` are placeholders until
//! the binary wires up a real bus subscription in a follow-up step).

use std::io;

use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use agentic_tui::modes::AppCommand;
use crossterm::event::{self, Event};
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
        if let Event::Key(key) = event::read()?
            && let Some(cmd) = state.handle_key(key.code)
        {
            match cmd {
                AppCommand::Quit => return Ok(()),
                // Plan + Status are accepted but no-op until a future
                // step wires the binary up to a real bus + backend.
                AppCommand::Plan { .. } | AppCommand::Status => {}
            }
        }
    }
}
