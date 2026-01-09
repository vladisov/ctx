use crate::app::{App, Focus, InputMode, PreviewMode};
use ctx_core::RenderResult;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

// Style helpers
fn bold(color: Color) -> Style { Style::default().fg(color).add_modifier(Modifier::BOLD) }
fn dim() -> Style { Style::default().fg(Color::DarkGray) }
fn separator() -> Line<'static> { Line::from(Span::styled("─".repeat(50), dim())) }
fn input_line(buffer: &str) -> Line<'_> { Line::from(vec![
    Span::styled("> ", bold(Color::Green)),
    Span::styled(buffer.to_string(), Style::default().fg(Color::Yellow)),
    Span::styled("█", Style::default().fg(Color::Yellow)),
])}

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    draw_header(f, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    match app.input_mode {
        InputMode::BrowsingFiles => draw_file_browser(f, app),
        InputMode::AddingArtifact => draw_add_artifact_dialog(f, app),
        InputMode::CreatingPack => draw_create_pack_dialog(f, app),
        InputMode::EditingBudget => draw_edit_budget_dialog(f, app),
        InputMode::ConfirmDeletePack => draw_confirm_delete_dialog(f, app),
        InputMode::ShowingHelp => draw_help_screen(f),
        InputMode::Normal => {}
    }

    if app.loading_message.is_some() {
        draw_loading_indicator(f, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("ctx - Interactive Pack Manager")
        .style(bold(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);
    draw_pack_list(f, app, chunks[0]);
    draw_preview(f, app, chunks[1]);
}

fn draw_pack_list(f: &mut Frame, app: &App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    for (i, pack) in app.packs.iter().enumerate() {
        let is_expanded = app.is_expanded(&pack.id);
        let prefix = if is_expanded { "▾" } else { "▸" };
        let source_count = app.pack_artifacts.get(&pack.id).map(|v| v.len()).unwrap_or(0);
        let budget_str = format_tokens(pack.policies.budget_tokens);

        let line = if source_count > 0 {
            format!("{} {}  ({} sources, {})", prefix, pack.name, source_count, budget_str)
        } else {
            format!("{} {}  [{}]", prefix, pack.name, budget_str)
        };

        let is_selected = i == app.selected_pack_index && app.selected_artifact_index.is_none();
        let style = if is_selected { bold(Color::Yellow) } else { Style::default() };
        items.push(ListItem::new(line).style(style));

        if is_expanded && i == app.selected_pack_index {
            if let Some(artifacts) = app.pack_artifacts.get(&pack.id) {
                for (idx, artifact) in artifacts.iter().enumerate() {
                    let is_artifact_selected = app.selected_artifact_index == Some(idx);
                    let style = if is_artifact_selected { bold(Color::Cyan) } else { dim() };
                    items.push(ListItem::new(format!("  ├─ {}", artifact.artifact.source_uri)).style(style));
                }
            }
        }
    }

    let title = match app.focus {
        Focus::PackList => format!(" Packs ({}) [FOCUSED] ", app.packs.len()),
        _ => format!(" Packs ({}) ", app.packs.len()),
    };

    f.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    if app.selected_artifact_index.is_some() && app.artifact_content.is_some() {
        return draw_artifact_content(f, app, area);
    }

    let mode_str = match app.preview_mode { PreviewMode::Stats => "stats", PreviewMode::Content => "content" };
    let title = match app.focus {
        Focus::Preview => format!(" Preview ({}) [FOCUSED] ", mode_str),
        _ => format!(" Preview ({}) ", mode_str),
    };

    if let Some(preview) = &app.preview_result {
        match app.preview_mode {
            PreviewMode::Stats => draw_preview_stats(f, area, &title, preview),
            PreviewMode::Content => draw_preview_content(f, app, area, &title, preview),
        }
    } else {
        draw_preview_help(f, app, area, &title);
    }
}

fn draw_artifact_content(f: &mut Frame, app: &App, area: Rect) {
    let content_str = app.artifact_content.as_ref().unwrap();
    let total_lines = content_str.lines().count();
    let visible = area.height.saturating_sub(2) as usize;
    let scroll = app.content_scroll.min(total_lines.saturating_sub(1));

    let (name, tokens, bytes) = app.packs.get(app.selected_pack_index)
        .and_then(|p| app.pack_artifacts.get(&p.id))
        .and_then(|a| app.selected_artifact_index.and_then(|i| a.get(i)))
        .map(|item| (item.artifact.source_uri.clone(), item.artifact.token_estimate, item.artifact.metadata.size_bytes))
        .unwrap_or_else(|| ("unknown".into(), 0, 0));

    let focus = if matches!(app.focus, Focus::Preview) { " [FOCUSED]" } else { "" };
    let title = format!(" {}{} (line {}/{}, {} tokens, {} bytes) ", name, focus, scroll + 1, total_lines, tokens, bytes);

    let content: String = content_str.lines().skip(scroll).take(visible).collect::<Vec<_>>().join("\n");
    f.render_widget(
        Paragraph::new(content).block(Block::default().borders(Borders::ALL).title(title)).wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_preview_stats(f: &mut Frame, area: Rect, title: &str, preview: &RenderResult) {
    let util = (preview.token_estimate as f64 / preview.budget_tokens as f64) * 100.0;
    let icon = if preview.token_estimate > preview.budget_tokens { "⚠" } else if util > 90.0 { "⚡" } else { "✓" };

    let mut lines = vec![
        "📊 Token Usage".into(),
        format!("  Budget:    {}", format_tokens(preview.budget_tokens)),
        format!("  Estimated: {}", format_tokens(preview.token_estimate)),
        format!("  Usage:     {:.1}% {}", util, icon),
        String::new(),
        "📦 Artifacts".into(),
        format!("  Included:  {}", preview.included.len()),
        format!("  Excluded:  {}", preview.excluded.len()),
    ];

    if !preview.redactions.is_empty() {
        lines.push(format!("  Redacted:  {} secrets", preview.redactions.len()));
    }
    lines.extend(["".into(), "🔒 Render Hash".into(), preview.render_hash.clone(), "".into()]);

    if !preview.excluded.is_empty() {
        lines.push("⚠ Excluded:".into());
        for exc in preview.excluded.iter().take(5) {
            lines.push(format!("  • {}: {}", exc.source_uri, exc.reason));
        }
        if preview.excluded.len() > 5 {
            lines.push(format!("  ... and {} more", preview.excluded.len() - 5));
        }
    }

    f.render_widget(
        Paragraph::new(lines.join("\n")).block(Block::default().borders(Borders::ALL).title(title)).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_preview_content(f: &mut Frame, app: &App, area: Rect, title: &str, preview: &RenderResult) {
    let (content, title) = preview.payload.as_ref().map(|payload| {
        let total = payload.lines().count();
        let visible = area.height.saturating_sub(2) as usize;
        let scroll = app.content_scroll.min(total.saturating_sub(1));
        let lines: String = payload.lines().skip(scroll).take(visible).collect::<Vec<_>>().join("\n");
        (lines, format!("{} (line {}/{}) ", title, scroll + 1, total))
    }).unwrap_or_else(|| ("No content. Press 'p' to preview.".into(), title.into()));

    f.render_widget(
        Paragraph::new(content).block(Block::default().borders(Borders::ALL).title(title)).wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_preview_help(f: &mut Frame, app: &App, area: Rect, title: &str) {
    let help = app.packs.get(app.selected_pack_index)
        .map(|p| format!("Pack: {}\n\nKeys: p=preview v=view j/k=scroll a=add d=delete c=create ?=help q=quit\n\nTip: Select artifact and press 'p' to view content!", p.name))
        .unwrap_or_else(|| "No packs. Press 'c' to create.".into());

    f.render_widget(
        Paragraph::new(help).block(Block::default().borders(Borders::ALL).title(title)).wrap(Wrap { trim: true }),
        area,
    );
}

fn format_tokens(tokens: usize) -> String {
    if tokens >= 1_000_000 { format!("{:.1}M", tokens as f64 / 1_000_000.0) }
    else if tokens >= 1_000 { format!("{}k", tokens / 1_000) }
    else { tokens.to_string() }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let status = app.status_message.as_deref().unwrap_or("Ready");
    let spans = vec![
        Span::raw(status), Span::raw(" | "),
        Span::styled("?", Style::default().fg(Color::Cyan)), Span::raw(":help "),
        Span::styled("c", Style::default().fg(Color::Yellow)), Span::raw(":create "),
        Span::styled("a", Style::default().fg(Color::Yellow)), Span::raw(":add "),
        Span::styled("p", Style::default().fg(Color::Yellow)), Span::raw(":preview "),
        Span::styled("q", Style::default().fg(Color::Yellow)), Span::raw(":quit"),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL)), area);
}

fn draw_add_artifact_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 22, f.area());
    let lines = vec![
        Line::from(Span::styled("Enter artifact URI:", bold(Color::Cyan))),
        Line::from(""),
        Line::from(Span::styled("Examples:", dim())),
        Line::from("  file:path/to/file"),
        Line::from("  glob:src/**/*.rs"),
        Line::from("  text:Your inline text"),
        Line::from("  git:diff --base=main"),
        Line::from(""), separator(), input_line(&app.input_buffer), separator(), Line::from(""),
        Line::from(Span::styled("Enter to confirm, Esc to cancel", dim())),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::default().title(" Add Artifact ").borders(Borders::ALL).style(Style::default().bg(Color::Black))).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_create_pack_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 18, f.area());
    let lines = vec![
        Line::from(Span::styled("Enter pack name or name:budget", bold(Color::Cyan))),
        Line::from(""),
        Line::from(Span::styled("Examples:", dim())),
        Line::from("  my-pack          (default 128k)"),
        Line::from("  my-pack:50000    (custom)"),
        Line::from(""), separator(), input_line(&app.input_buffer), separator(), Line::from(""),
        Line::from(Span::styled("Enter to confirm, Esc to cancel", dim())),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::default().title(" Create Pack ").borders(Borders::ALL).style(Style::default().bg(Color::Black))).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_edit_budget_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 15, f.area());
    let (name, budget) = app.packs.get(app.selected_pack_index)
        .map(|p| (p.name.as_str(), p.policies.budget_tokens))
        .unwrap_or(("unknown", 0));

    let lines = vec![
        Line::from(vec![Span::raw("Current: "), Span::styled(budget.to_string(), Style::default().fg(Color::Cyan))]),
        Line::from(""),
        Line::from(Span::styled("Enter new budget:", bold(Color::Cyan))),
        Line::from(""), separator(), input_line(&app.input_buffer), separator(), Line::from(""),
        Line::from(Span::styled("Enter to confirm, Esc to cancel", dim())),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::default().title(format!(" Edit Budget: {} ", name)).borders(Borders::ALL).style(Style::default().bg(Color::Black))).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_confirm_delete_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 12, f.area());
    let name = app.packs.get(app.selected_pack_index).map(|p| p.name.as_str()).unwrap_or("unknown");
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(format!("Delete pack '{}'?", name), bold(Color::Red))),
        Line::from(""),
        Line::from("This cannot be undone."),
        Line::from(""),
        Line::from(Span::styled("Y to confirm, N/Esc to cancel", dim())),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::default().title(" Confirm Delete ").borders(Borders::ALL).style(Style::default().bg(Color::Black).fg(Color::Red))).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_help_screen(f: &mut Frame) {
    let area = centered_rect(80, 85, f.area());
    let sections = [
        ("Navigation", vec!["j/k ↓/↑  Navigate", "Space/Enter  Expand pack", "Tab  Switch focus"]),
        ("Pack", vec!["c  Create", "e  Edit budget", "D  Delete", "r  Refresh"]),
        ("Artifact", vec!["a  Add artifact", "d  Delete artifact"]),
        ("Preview", vec!["p  Preview/load content", "v  Toggle stats/content", "j/k  Scroll", "PageUp/Down  Page scroll"]),
        ("Other", vec!["?  Help", "q  Quit"]),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (title, items) in sections {
        lines.push(Line::from(Span::styled(title, bold(Color::Yellow))));
        for item in items { lines.push(Line::from(format!("  {}", item))); }
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled("Tips:", bold(Color::Cyan))));
    lines.push(Line::from("  • Select artifact (cyan) and press 'p' to view"));
    lines.push(Line::from("  • Pack format: 'name' or 'name:budget'"));

    f.render_widget(
        Paragraph::new(lines).block(Block::default().title(" Help - ? or Esc to close ").borders(Borders::ALL).style(Style::default().bg(Color::Black))).wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_loading_indicator(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 10, f.area());
    let msg = app.loading_message.as_deref().unwrap_or("");
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("⏳ Loading...", bold(Color::Yellow))),
        Line::from(""),
        Line::from(msg),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Black).fg(Color::Yellow))).alignment(Alignment::Center),
        area,
    );
}

fn draw_file_browser(f: &mut Frame, app: &App) {
    let area = centered_rect(90, 85, f.area());
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let Some(browser) = &app.file_browser else { return };
    let visible = chunks[0].height.saturating_sub(2) as usize;

    // File list
    if browser.entries.is_empty() {
        f.render_widget(
            Paragraph::new("Directory is empty").block(Block::default().borders(Borders::ALL).title(" Files ")).style(dim()),
            chunks[0],
        );
    } else {
        let items: Vec<ListItem> = browser.entries.iter().enumerate()
            .skip(browser.scroll_offset)
            .take(visible)
            .map(|(i, e)| {
                let icon = if e.name == ".." { "⬆ " } else if e.is_dir { "📁 " } else { "📄 " };
                let name = if e.name.len() > 50 { format!("{}...", &e.name[..47]) } else { e.name.clone() };
                let style = if i == browser.selected_index { bold(Color::Yellow).bg(Color::DarkGray) }
                    else if e.is_dir { Style::default().fg(Color::Cyan) }
                    else if e.is_hidden { dim() }
                    else { Style::default() };
                ListItem::new(format!("{}{}", icon, name)).style(style)
            }).collect();

        let scroll_info = if browser.entries.len() > visible { format!(" [{}/{}]", browser.selected_index + 1, browser.entries.len()) } else { String::new() };
        let title = format!(" Files: {}{} ", browser.current_dir.display(), scroll_info);
        f.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).title(title)), chunks[0]);
    }

    // Info pane
    let mut info = vec![
        Line::from(Span::styled("Type:", bold(Color::Cyan))),
        Line::from(Span::styled(browser.artifact_type.label(), Style::default().fg(Color::Yellow))),
        Line::from(""),
    ];

    if browser.is_text_mode() {
        info.extend([
            Line::from(Span::styled("Text Mode", bold(Color::Yellow))),
            Line::from("Press Space for text input"),
        ]);
    } else if let Some(e) = browser.selected_entry() {
        info.extend([
            Line::from(Span::styled("Selected:", bold(Color::Cyan))),
            Line::from(e.name.clone()),
            Line::from(""),
        ]);
        if let Some(uri) = browser.get_selected_uri() {
            info.extend([Line::from(Span::styled("Will add:", bold(Color::Cyan))), Line::from(Span::styled(uri, Style::default().fg(Color::Green)))]);
        } else if e.name == ".." {
            info.push(Line::from(Span::styled("Enter to go up", dim())));
        }
    }

    info.extend([Line::from(""), separator(), Line::from("")]);
    for (key, desc) in [("j/k", "Navigate"), ("Enter/l", "Enter"), ("h/Back", "Up"), ("Tab", "Type"), (".", "Hidden"), ("Space", "Confirm"), ("i", "Text"), ("Esc", "Cancel")] {
        info.push(Line::from(vec![Span::styled(key, Style::default().fg(Color::Yellow)), Span::raw(format!(" {}", desc))]));
    }

    f.render_widget(Paragraph::new(info).block(Block::default().borders(Borders::ALL).title(" Info ")).wrap(Wrap { trim: true }), chunks[1]);
}

fn centered_rect(px: u16, py: u16, r: Rect) -> Rect {
    let v = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Percentage((100 - py) / 2), Constraint::Percentage(py), Constraint::Percentage((100 - py) / 2)])
        .split(r);
    Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Percentage((100 - px) / 2), Constraint::Percentage(px), Constraint::Percentage((100 - px) / 2)])
        .split(v[1])[1]
}
