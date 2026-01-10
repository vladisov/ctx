mod app;
mod file_browser;
mod ui;

pub use app::App;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
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

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            use app::InputMode;

            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => app.toggle_help(),
                    KeyCode::Esc => app.exit_content_view(),
                    KeyCode::Char('j') | KeyCode::Down => app.navigate_or_scroll_down(),
                    KeyCode::Char('k') | KeyCode::Up => app.navigate_or_scroll_up(),
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
                },
                InputMode::BrowsingFiles => {
                    let h = (terminal.size()?.height as usize * 85 / 100).saturating_sub(4);
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.browser_next(h),
                        KeyCode::Char('k') | KeyCode::Up => app.browser_previous(),
                        KeyCode::Enter | KeyCode::Char('l') => app.browser_enter()?,
                        KeyCode::Char('h') | KeyCode::Backspace => app.browser_go_up()?,
                        KeyCode::Char('.') => app.browser_toggle_hidden()?,
                        KeyCode::Tab => app.browser_cycle_type(),
                        KeyCode::Char(' ') => app.browser_confirm_selection().await?,
                        KeyCode::Esc => app.cancel_input(),
                        _ => {}
                    }
                }
                InputMode::AddingArtifact | InputMode::CreatingPack | InputMode::EditingBudget => {
                    match key.code {
                        KeyCode::Enter => match app.input_mode {
                            InputMode::AddingArtifact => app.confirm_add_artifact().await?,
                            InputMode::CreatingPack => app.confirm_create_pack().await?,
                            InputMode::EditingBudget => app.confirm_edit_budget().await?,
                            _ => {}
                        },
                        KeyCode::Esc => app.cancel_input(),
                        KeyCode::Backspace => app.input_backspace(),
                        KeyCode::Char(c) => app.input_char(c),
                        _ => {}
                    }
                }
                InputMode::ConfirmDeletePack => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_pack().await?,
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_input(),
                    _ => {}
                },
                InputMode::ShowingHelp => match key.code {
                    KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => app.toggle_help(),
                    _ => {}
                },
            }
        }
    }
}
