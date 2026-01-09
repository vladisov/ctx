mod app;
mod ui;
mod file_browser;

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
                            KeyCode::Char('?') => app.toggle_help(),
                            KeyCode::Esc => {
                                // Clear artifact content view, go back to navigation
                                if app.artifact_content.is_some() {
                                    app.artifact_content = None;
                                    app.content_scroll = 0;
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                // If viewing artifact content or pack content, scroll
                                if app.artifact_content.is_some() || app.preview_mode == app::PreviewMode::Content {
                                    app.scroll_content_down();
                                } else {
                                    app.next();
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                // If viewing artifact content or pack content, scroll
                                if app.artifact_content.is_some() || app.preview_mode == app::PreviewMode::Content {
                                    app.scroll_content_up();
                                } else {
                                    app.previous();
                                }
                            }
                            KeyCode::Char(' ') | KeyCode::Enter => app.toggle_expand().await?,
                            KeyCode::Char('p') => app.preview().await?,
                            KeyCode::Char('v') => app.toggle_preview_mode(),
                            KeyCode::Char('r') => app.refresh().await?,
                            KeyCode::Char('a') => app.start_add_artifact(),
                            KeyCode::Char('c') => app.start_create_pack(),
                            KeyCode::Char('e') => app.start_edit_budget(),
                            KeyCode::Char('d') => app.delete_artifact().await?,
                            KeyCode::Char('D') => app.start_delete_pack(),
                            KeyCode::Tab => app.cycle_focus(),
                            KeyCode::PageUp => app.scroll_page_up(),
                            KeyCode::PageDown => app.scroll_page_down(),
                            _ => {}
                        }
                    }
                    InputMode::BrowsingFiles => {
                        // Calculate visible height for scrolling (85% of terminal height, minus borders)
                        let visible_height = (terminal.size()?.height as usize * 85 / 100).saturating_sub(4);
                        match key.code {
                            KeyCode::Char('j') | KeyCode::Down => app.browser_next(visible_height),
                            KeyCode::Char('k') | KeyCode::Up => app.browser_previous(),
                            KeyCode::Enter | KeyCode::Char('l') => app.browser_enter()?,
                            KeyCode::Char('h') | KeyCode::Backspace => app.browser_go_up()?,
                            KeyCode::Char('.') => app.browser_toggle_hidden()?,
                            KeyCode::Tab => app.browser_cycle_type(),
                            KeyCode::Char(' ') => app.browser_confirm_selection().await?,
                            KeyCode::Char('i') => app.browser_switch_to_text_input(),
                            KeyCode::Esc => app.cancel_input(),
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
                    InputMode::CreatingPack => {
                        match key.code {
                            KeyCode::Enter => app.confirm_create_pack().await?,
                            KeyCode::Esc => app.cancel_input(),
                            KeyCode::Backspace => app.input_backspace(),
                            KeyCode::Char(c) => app.input_char(c),
                            _ => {}
                        }
                    }
                    InputMode::EditingBudget => {
                        match key.code {
                            KeyCode::Enter => app.confirm_edit_budget().await?,
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
                    InputMode::ShowingHelp => {
                        match key.code {
                            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => app.toggle_help(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
