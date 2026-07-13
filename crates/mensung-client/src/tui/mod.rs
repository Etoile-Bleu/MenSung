//! Entry point for the interactive terminal interface: sets up the
//! terminal, runs the event loop, and always restores the terminal on the
//! way out, including on error, so a crash never leaves the user's shell in
//! raw mode.

mod app;
mod ui;

use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{self, Event};
use mensung_db::Database;

use app::App;

pub(crate) fn run(db: &Database) -> ExitCode {
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(err) => {
            eprintln!("Fatal: could not start the terminal interface: {err}");
            return ExitCode::from(70);
        }
    };

    let mut app = App::new(db);
    let result = event_loop(&mut terminal, &mut app);

    ratatui::restore();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Fatal: terminal interface error: {err}");
            ExitCode::from(70)
        }
    }
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    while !app.should_quit() {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
    }

    Ok(())
}
