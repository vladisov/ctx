use crate::app::{App, Focus, InputMode, PreviewMode};
use ctx_core::RenderResult;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    draw_header(f, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    // Draw overlays
    match app.input_mode {
        InputMode::AddingArtifact => draw_add_artifact_dialog(f, app),
        InputMode::CreatingPack => draw_create_pack_dialog(f, app),
        InputMode::EditingBudget => draw_edit_budget_dialog(f, app),
        InputMode::ConfirmDeletePack => draw_confirm_delete_dialog(f, app),
        InputMode::ShowingHelp => draw_help_screen(f),
        InputMode::Normal => {}
    }
}

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("ctx - Interactive Pack Manager")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Pack list
            Constraint::Percentage(60), // Preview/Details
        ])
        .split(area);

    draw_pack_list(f, app, chunks[0]);
    draw_preview(f, app, chunks[1]);
}

fn draw_pack_list(f: &mut Frame, app: &App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    for (i, pack) in app.packs.iter().enumerate() {
        let is_expanded = app.is_expanded(&pack.id);
        let prefix = if is_expanded { "▾" } else { "▸" };

        // Get source count
        let source_count = app
            .pack_artifacts
            .get(&pack.id)
            .map(|v| v.len())
            .unwrap_or(0);

        // Format budget nicely
        let budget = pack.policies.budget_tokens;
        let budget_str = if budget >= 1000 {
            format!("{}k", budget / 1000)
        } else {
            budget.to_string()
        };

        // Main pack line
        let line = if source_count > 0 {
            format!(
                "{} {}  ({} sources, {})",
                prefix, pack.name, source_count, budget_str
            )
        } else {
            format!("{} {}  [{}]", prefix, pack.name, budget_str)
        };

        let is_pack_selected =
            i == app.selected_pack_index && app.selected_artifact_index.is_none();
        let style = if is_pack_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        items.push(ListItem::new(line).style(style));

        // If expanded, show sources
        if is_expanded && i == app.selected_pack_index {
            if let Some(artifacts) = app.pack_artifacts.get(&pack.id) {
                for (artifact_idx, artifact) in artifacts.iter().enumerate() {
                    let uri = &artifact.artifact.source_uri;
                    let source_line = format!("  ├─ {}", uri);

                    let is_artifact_selected = app.selected_artifact_index == Some(artifact_idx);
                    let artifact_style = if is_artifact_selected {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    items.push(ListItem::new(source_line).style(artifact_style));
                }
            }
        }
    }

    let title = match app.focus {
        Focus::PackList => format!(" Packs ({}) [FOCUSED] ", app.packs.len()),
        _ => format!(" Packs ({}) ", app.packs.len()),
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(list, area);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    // If artifact is selected and content is loaded, show artifact content
    if app.selected_artifact_index.is_some() && app.artifact_content.is_some() {
        draw_artifact_content(f, app, area);
        return;
    }

    // Otherwise show pack preview
    let title_suffix = match app.preview_mode {
        PreviewMode::Stats => " (stats) ",
        PreviewMode::Content => " (content) ",
    };

    let title = match app.focus {
        Focus::Preview => format!(" Preview{} [FOCUSED] ", title_suffix),
        _ => format!(" Preview{} ", title_suffix),
    };

    if let Some(preview) = &app.preview_result {
        match app.preview_mode {
            PreviewMode::Stats => draw_preview_stats(f, app, area, &title, preview),
            PreviewMode::Content => draw_preview_content(f, app, area, &title, preview),
        }
    } else {
        draw_preview_help(f, app, area, &title);
    }
}

fn draw_artifact_content(f: &mut Frame, app: &App, area: Rect) {
    let content_str = app.artifact_content.as_ref().unwrap();
    let total_lines = content_str.lines().count();
    let visible_lines = area.height.saturating_sub(2) as usize;
    let scroll_pos = app.content_scroll.min(total_lines.saturating_sub(1));

    let lines: Vec<&str> = content_str
        .lines()
        .skip(scroll_pos)
        .take(visible_lines)
        .collect();

    // Get artifact info
    let (artifact_name, token_estimate, size) =
        if let Some(pack) = app.packs.get(app.selected_pack_index) {
            if let Some(artifacts) = app.pack_artifacts.get(&pack.id) {
                if let Some(idx) = app.selected_artifact_index {
                    if let Some(item) = artifacts.get(idx) {
                        let name = item.artifact.source_uri.clone();
                        let tokens = item.artifact.token_estimate;
                        let bytes = item.artifact.metadata.size_bytes;
                        (name, tokens, bytes)
                    } else {
                        ("unknown".to_string(), 0, 0)
                    }
                } else {
                    ("unknown".to_string(), 0, 0)
                }
            } else {
                ("unknown".to_string(), 0, 0)
            }
        } else {
            ("unknown".to_string(), 0, 0)
        };

    let title = match app.focus {
        Focus::Preview => format!(
            " {} [FOCUSED] (line {}/{}, {} tokens, {} bytes) ",
            artifact_name,
            scroll_pos + 1,
            total_lines,
            token_estimate,
            size
        ),
        _ => format!(
            " {} (line {}/{}, {} tokens, {} bytes) ",
            artifact_name,
            scroll_pos + 1,
            total_lines,
            token_estimate,
            size
        ),
    };

    let content = lines.join("\n");
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_preview_stats(_f: &mut Frame, _app: &App, area: Rect, title: &str, preview: &RenderResult) {
    let utilization = (preview.token_estimate as f64 / preview.budget_tokens as f64) * 100.0;
    let status_icon = if preview.token_estimate > preview.budget_tokens {
        "⚠"
    } else if utilization > 90.0 {
        "⚡"
    } else {
        "✓"
    };

    let budget_str = format_tokens(preview.budget_tokens);
    let estimate_str = format_tokens(preview.token_estimate);

    let mut lines = vec![
        format!("📊 Token Usage"),
        format!("  Budget:    {}", budget_str),
        format!("  Estimated: {}", estimate_str),
        format!("  Usage:     {:.1}% {}", utilization, status_icon),
        String::new(),
        format!("📦 Artifacts"),
        format!("  Included:  {}", preview.included.len()),
        format!("  Excluded:  {}", preview.excluded.len()),
    ];

    if !preview.redactions.is_empty() {
        lines.push(format!("  Redacted:  {} secrets", preview.redactions.len()));
    }

    lines.push(String::new());
    lines.push("🔒 Render Hash".to_string());
    lines.push(preview.render_hash.clone());
    lines.push("".to_string());

    if !preview.excluded.is_empty() {
        lines.push("⚠ Excluded Artifacts:".to_string());
        for exc in preview.excluded.iter().take(5) {
            lines.push(format!("  • {}: {}", exc.source_uri, exc.reason));
        }
        if preview.excluded.len() > 5 {
            lines.push(format!("  ... and {} more", preview.excluded.len() - 5));
        }
    }

    let content = lines.join("\n");
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });

    _f.render_widget(paragraph, area);
}

fn draw_preview_content(
    _f: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    preview: &RenderResult,
) {
    let (content, title_with_scroll) = if let Some(payload) = &preview.payload {
        let total_lines = payload.lines().count();
        let visible_lines = area.height.saturating_sub(2) as usize;
        let scroll_pos = app.content_scroll.min(total_lines.saturating_sub(1));

        let lines: Vec<&str> = payload
            .lines()
            .skip(scroll_pos)
            .take(visible_lines)
            .collect();

        let scroll_info = format!(" (line {}/{}) ", scroll_pos + 1, total_lines);
        let title_with_scroll = title.replace(" ", &scroll_info);

        (lines.join("\n"), title_with_scroll)
    } else {
        (
            "No content rendered. Use 'p' to preview first.".to_string(),
            title.to_string(),
        )
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title_with_scroll),
        )
        .wrap(Wrap { trim: false });

    _f.render_widget(paragraph, area);
}

fn draw_preview_help(_f: &mut Frame, app: &App, area: Rect, title: &str) {
    let help_text = if let Some(pack) = app.packs.get(app.selected_pack_index) {
        format!(
            "Pack: {}\n\nKeyboard shortcuts:\n  p - Preview pack or load artifact content\n  v - Toggle stats/content view (pack)\n  j/k - Scroll content\n  space/enter - Expand/collapse\n  a - Add artifact\n  d - Delete selected artifact\n  D - Delete pack\n  c - Create pack\n  e - Edit budget\n  r - Refresh\n  ? - Help\n  q - Quit\n\nTip: Select an artifact and press 'p' to view its content!",
            pack.name
        )
    } else {
        "No packs found.\n\nCreate a pack with:\n  c - Create new pack\n  or: ctx pack create <name>".to_string()
    };

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });

    _f.render_widget(paragraph, area);
}

fn format_tokens(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{}k", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let status = if let Some(msg) = &app.status_message {
        msg.clone()
    } else {
        "Ready".to_string()
    };

    let help_text = vec![
        Span::raw(status),
        Span::raw(" | "),
        Span::styled("?", Style::default().fg(Color::Cyan)),
        Span::raw(":help "),
        Span::styled("c", Style::default().fg(Color::Yellow)),
        Span::raw(":create "),
        Span::styled("e", Style::default().fg(Color::Yellow)),
        Span::raw(":edit "),
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::raw(":add "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(":del "),
        Span::styled("p", Style::default().fg(Color::Yellow)),
        Span::raw(":preview "),
        Span::styled("v", Style::default().fg(Color::Yellow)),
        Span::raw(":view "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(":quit"),
    ];

    let footer =
        Paragraph::new(Line::from(help_text)).block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, area);
}

fn draw_add_artifact_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());

    let block = Block::default()
        .title(" Add Artifact ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let text = vec![
        Line::from("Enter artifact URI:"),
        Line::from("  file:path/to/file"),
        Line::from("  glob:src/**/*.rs"),
        Line::from("  text:Your inline text"),
        Line::from("  git:diff --base=main"),
        Line::from(""),
        Line::from(Span::styled(
            &app.input_buffer,
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter to confirm, Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_create_pack_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 18, f.area());

    let block = Block::default()
        .title(" Create Pack ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let text = vec![
        Line::from("Enter pack name or name:budget"),
        Line::from(""),
        Line::from("Examples:"),
        Line::from("  my-pack          (uses default 128k budget)"),
        Line::from("  my-pack:50000    (custom budget)"),
        Line::from(""),
        Line::from(Span::styled(
            &app.input_buffer,
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter to confirm, Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_edit_budget_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 15, f.area());

    let pack_name = app
        .packs
        .get(app.selected_pack_index)
        .map(|p| p.name.as_str())
        .unwrap_or("unknown");

    let current_budget = app
        .packs
        .get(app.selected_pack_index)
        .map(|p| p.policies.budget_tokens)
        .unwrap_or(0);

    let block = Block::default()
        .title(format!(" Edit Budget: {} ", pack_name))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let text = vec![
        Line::from(format!("Current budget: {}", current_budget)),
        Line::from(""),
        Line::from("New budget:"),
        Line::from(Span::styled(
            &app.input_buffer,
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter to confirm, Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_confirm_delete_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 15, f.area());

    let pack_name = app
        .packs
        .get(app.selected_pack_index)
        .map(|p| p.name.as_str())
        .unwrap_or("unknown");

    let block = Block::default()
        .title(" Confirm Delete ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::Red));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete pack '{}'?", pack_name),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("This action cannot be undone."),
        Line::from(""),
        Line::from(Span::styled(
            "Y to confirm, N/Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_help_screen(f: &mut Frame) {
    let area = centered_rect(80, 90, f.area());

    let block = Block::default()
        .title(" Help - Press ? or Esc to close ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let text = vec![
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  j/k or ↓/↑       Navigate packs and artifacts"),
        Line::from("  Space/Enter      Expand/collapse pack to show sources"),
        Line::from("  Tab              Switch focus between pack list and preview"),
        Line::from(""),
        Line::from(Span::styled(
            "Pack Management",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  c                Create new pack"),
        Line::from("  e                Edit pack budget"),
        Line::from("  D                Delete pack (with confirmation)"),
        Line::from("  r                Refresh pack list"),
        Line::from(""),
        Line::from(Span::styled(
            "Artifact Management",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  a                Add artifact to selected pack"),
        Line::from("  d                Delete selected artifact"),
        Line::from(""),
        Line::from(Span::styled(
            "Preview & Content",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  p                Preview pack OR load artifact content (context-aware)"),
        Line::from("  v                Toggle between stats view and content view (pack only)"),
        Line::from("  j/k              Scroll content line-by-line"),
        Line::from("  PageUp/PageDown  Scroll content page by page"),
        Line::from(""),
        Line::from(Span::styled(
            "Other",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ?                Show this help screen"),
        Line::from("  q                Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Tips:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  • Expand a pack to see and navigate its artifacts"),
        Line::from("  • Select an artifact (cyan highlight) and press 'p' to view its content"),
        Line::from("  • j/k scrolls content when viewing artifact or pack content"),
        Line::from("  • Pack preview shows token usage and excluded artifacts"),
        Line::from("  • Create pack format: 'name' or 'name:budget'"),
    ];

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
