use crate::app::{App, Focus};
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
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());

    draw_header(f, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("ctx - Interactive Pack Manager")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
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
        let source_count = app.pack_artifacts.get(&pack.id).map(|v| v.len()).unwrap_or(0);

        // Format budget nicely
        let budget = pack.policies.budget_tokens;
        let budget_str = if budget >= 1000 {
            format!("{}k", budget / 1000)
        } else {
            budget.to_string()
        };

        // Main pack line
        let line = if source_count > 0 {
            format!("{} {}  ({} sources, {})", prefix, pack.name, source_count, budget_str)
        } else {
            format!("{} {}  [{}]", prefix, pack.name, budget_str)
        };

        let style = if i == app.selected_pack_index {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        items.push(ListItem::new(line).style(style));

        // If expanded, show sources
        if is_expanded {
            if let Some(artifacts) = app.pack_artifacts.get(&pack.id) {
                for artifact in artifacts {
                    let uri = &artifact.artifact.source_uri;
                    let source_line = format!("  ├─ {}", uri);
                    items.push(ListItem::new(source_line).style(Style::default().fg(Color::DarkGray)));
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
    let title = match app.focus {
        Focus::Preview => " Preview [FOCUSED] ",
        _ => " Preview ",
    };

    if let Some(preview) = &app.preview_result {
        let utilization = (preview.token_estimate as f64 / preview.budget_tokens as f64) * 100.0;
        let status_icon = if preview.token_estimate > preview.budget_tokens {
            "⚠"
        } else if utilization > 90.0 {
            "⚡"
        } else {
            "✓"
        };

        // Format budget nicely
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
        lines.push(format!("🔒 Render Hash"));
        lines.push(format!("  {}...", &preview.render_hash[..16]));

        if !preview.excluded.is_empty() {
            lines.push(String::new());
            lines.push(format!("⚠ Excluded Artifacts:"));
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

        f.render_widget(paragraph, area);
    } else {
        let help_text = if let Some(pack) = app.packs.get(app.selected_pack_index) {
            format!(
                "Pack: {}\n\nKeyboard shortcuts:\n  p - Preview pack\n  space/enter - Expand/collapse\n  r - Refresh pack list\n  tab - Switch focus\n  q - Quit",
                pack.name
            )
        } else {
            "No packs found.\n\nCreate a pack with:\n  ctx pack create <name>".to_string()
        };

        let paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }
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
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(":quit "),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(":move "),
        Span::styled("p", Style::default().fg(Color::Yellow)),
        Span::raw(":preview "),
        Span::styled("space", Style::default().fg(Color::Yellow)),
        Span::raw(":toggle "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(":refresh"),
    ];

    let footer = Paragraph::new(Line::from(help_text))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, area);
}
