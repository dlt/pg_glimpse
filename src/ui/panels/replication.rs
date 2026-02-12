use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;
use crate::ui::util::{empty_state, format_bytes, format_lag, truncate};

use super::panel_block;

pub fn render_replication(frame: &mut Frame, app: &mut App, area: Rect) {
    let emoji = if app.config.show_emojis { "🔄 " } else { "" };
    let title = format!("{emoji}Replication");
    let block = panel_block(&title);

    let Some(snap) = &app.snapshot else {
        frame.render_widget(Paragraph::new("No data").block(block), area);
        return;
    };

    let has_replication = !snap.replication.is_empty();
    let has_slots = !snap.replication_slots.is_empty();
    let has_subscriptions = !snap.subscriptions.is_empty();

    // If nothing to show, display empty state
    if !has_replication && !has_slots && !has_subscriptions {
        frame.render_widget(empty_state("No replication activity", block), area);
        return;
    }

    // Clone the data we need to avoid borrow conflicts
    let replication = snap.replication.clone();
    let replication_slots = snap.replication_slots.clone();
    let subscriptions = snap.subscriptions.clone();

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate section heights based on content
    let mut constraints = Vec::new();

    // Physical replication section (header + rows + margin)
    if has_replication {
        let repl_height = (replication.len() + 2).min(8) as u16;
        constraints.push(Constraint::Length(repl_height));
    }

    // Slots section
    if has_slots {
        let slots_height = (replication_slots.len() + 2).min(8) as u16;
        constraints.push(Constraint::Length(slots_height));
    }

    // Subscriptions section
    if has_subscriptions {
        let subs_height = (subscriptions.len() + 2).min(6) as u16;
        constraints.push(Constraint::Length(subs_height));
    }

    // Add a filler to take remaining space
    constraints.push(Constraint::Min(0));

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut section_idx = 0;

    // Render physical replication
    if has_replication {
        render_physical_replication(frame, app, &replication, sections[section_idx]);
        section_idx += 1;
    }

    // Render slots
    if has_slots {
        render_replication_slots(frame, &replication_slots, sections[section_idx]);
        section_idx += 1;
    }

    // Render subscriptions
    if has_subscriptions {
        render_subscriptions(frame, &subscriptions, sections[section_idx]);
    }
}

fn render_physical_replication(
    frame: &mut Frame,
    app: &mut App,
    replication: &[crate::db::models::ReplicationInfo],
    area: Rect,
) {
    let title_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);

    // Section header
    let header_area = Rect { height: 1, ..area };
    let table_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("Physical Replication", title_style))),
        header_area,
    );

    let header = Row::new(vec![
        "PID", "App", "Client", "State", "Replay LSN", "Write Lag", "Flush Lag", "Replay Lag", "Sync",
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let rows: Vec<Row> = replication
        .iter()
        .map(|r| {
            Row::new(vec![
                Cell::from(r.pid.to_string()),
                Cell::from(truncate(&r.application_name.clone().unwrap_or_else(|| "-".into()), 12)),
                Cell::from(r.client_addr.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.state.clone().unwrap_or_else(|| "-".into())),
                Cell::from(r.replay_lsn.clone().unwrap_or_else(|| "-".into())),
                Cell::from(format_lag(r.write_lag_secs)),
                Cell::from(format_lag(r.flush_lag_secs)),
                Cell::from(format_lag(r.replay_lag_secs))
                    .style(Style::default().fg(Theme::lag_color(r.replay_lag_secs))),
                Cell::from(r.sync_state.clone().unwrap_or_else(|| "-".into())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(7),   // PID
        Constraint::Length(12),  // App
        Constraint::Length(16),  // Client
        Constraint::Length(10),  // State
        Constraint::Length(14),  // Replay LSN
        Constraint::Length(10),  // Write Lag
        Constraint::Length(10),  // Flush Lag
        Constraint::Length(10),  // Replay Lag
        Constraint::Length(8),   // Sync
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Theme::highlight_bg())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25ba} ");

    frame.render_stateful_widget(table, table_area, &mut app.panels.replication);
}

fn render_replication_slots(
    frame: &mut Frame,
    slots: &[crate::db::models::ReplicationSlot],
    area: Rect,
) {
    let title_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);

    // Section header
    let header_area = Rect { height: 1, ..area };
    let table_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("Replication Slots", title_style))),
        header_area,
    );

    let header = Row::new(vec![
        "Slot Name", "Type", "Database", "Active", "WAL Retained", "Restart LSN",
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let rows: Vec<Row> = slots
        .iter()
        .map(|s| {
            let active_style = if s.active {
                Style::default().fg(Theme::border_ok())
            } else {
                Style::default().fg(Theme::border_warn())
            };

            // Color WAL retained based on size
            let retained_color = match s.wal_retained_bytes {
                Some(bytes) if bytes > 10 * 1024 * 1024 * 1024 => Theme::border_danger(), // >10GB
                Some(bytes) if bytes > 1024 * 1024 * 1024 => Theme::border_warn(),        // >1GB
                _ => Theme::fg(),
            };

            Row::new(vec![
                Cell::from(truncate(&s.slot_name, 20)),
                Cell::from(s.slot_type.clone()),
                Cell::from(s.database.clone().unwrap_or_else(|| "-".into())),
                Cell::from(if s.active { "yes" } else { "no" }).style(active_style),
                Cell::from(s.wal_retained_bytes.map_or_else(|| "-".into(), format_bytes))
                    .style(Style::default().fg(retained_color)),
                Cell::from(s.restart_lsn.clone().unwrap_or_else(|| "-".into())),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(20),  // Slot Name
        Constraint::Length(10),  // Type
        Constraint::Length(14),  // Database
        Constraint::Length(8),   // Active
        Constraint::Length(12),  // WAL Retained
        Constraint::Length(16),  // Restart LSN
    ];

    let table = Table::new(rows, widths).header(header);
    frame.render_widget(table, table_area);
}

fn render_subscriptions(
    frame: &mut Frame,
    subscriptions: &[crate::db::models::Subscription],
    area: Rect,
) {
    let title_style = Style::default()
        .fg(Theme::fg())
        .add_modifier(Modifier::BOLD);

    // Section header
    let header_area = Rect { height: 1, ..area };
    let table_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled("Subscriptions (Logical Replication)", title_style))),
        header_area,
    );

    let header = Row::new(vec![
        "Name", "Enabled", "Worker PID", "Tables", "Received LSN", "Last Msg",
    ])
    .style(Theme::title_style())
    .bottom_margin(0);

    let rows: Vec<Row> = subscriptions
        .iter()
        .map(|s| {
            let enabled_style = if s.enabled {
                Style::default().fg(Theme::border_ok())
            } else {
                Style::default().fg(Theme::border_warn())
            };

            // Format "last message" time as relative
            let last_msg = s.last_msg_receipt_time.map(|t| {
                let elapsed = chrono::Utc::now() - t;
                let secs = elapsed.num_seconds();
                if secs < 60 {
                    format!("{secs}s ago")
                } else if secs < 3600 {
                    format!("{}m ago", secs / 60)
                } else {
                    format!("{}h ago", secs / 3600)
                }
            }).unwrap_or_else(|| "-".into());

            Row::new(vec![
                Cell::from(truncate(&s.subname, 20)),
                Cell::from(if s.enabled { "yes" } else { "no" }).style(enabled_style),
                Cell::from(s.pid.map_or_else(|| "-".into(), |p| p.to_string())),
                Cell::from(s.relcount.to_string()),
                Cell::from(s.received_lsn.clone().unwrap_or_else(|| "-".into())),
                Cell::from(last_msg),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(20),  // Name
        Constraint::Length(8),   // Enabled
        Constraint::Length(12),  // Worker PID
        Constraint::Length(8),   // Tables
        Constraint::Length(16),  // Received LSN
        Constraint::Length(12),  // Last Msg
    ];

    let table = Table::new(rows, widths).header(header);
    frame.render_widget(table, table_area);
}
