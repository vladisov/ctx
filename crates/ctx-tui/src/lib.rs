mod app;
mod ui;

pub use app::App;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub async fn run(storage: ctx_storage::Storage) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(storage).await?;

    // Run the app
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                use app::InputMode;

                match app.input_mode {
                    InputMode::Normal => {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('j') | KeyCode::Down => app.next(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous(),
                            KeyCode::Char(' ') | KeyCode::Enter => app.toggle_expand().await?,
                            KeyCode::Char('p') => app.preview().await?,
                            KeyCode::Char('v') => app.toggle_preview_mode(),
                            KeyCode::Char('r') => app.refresh().await?,
                            KeyCode::Char('a') => app.start_add_artifact(),
                            KeyCode::Char('d') => app.delete_artifact().await?,
                            KeyCode::Char('D') => app.start_delete_pack(),
                            KeyCode::Tab => app.cycle_focus(),
                            KeyCode::PageUp => app.scroll_content_up(),
                            KeyCode::PageDown => app.scroll_content_down(),
                            _ => {}
                        }
                    }
                    InputMode::AddingArtifact => {
                        match key.code {
                            KeyCode::Enter => app.confirm_add_artifact().await?,
                            KeyCode::Esc => app.cancel_input(),
                            KeyCode::Backspace => app.input_backspace(),
                            KeyCode::Char(c) => app.input_char(c),
                            _ => {}
                        }
                    }
                    InputMode::ConfirmDeletePack => {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_pack().await?,
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_input(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
