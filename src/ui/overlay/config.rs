use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, ViewMode};
use crate::config::ConfigItem;
use crate::recorder::Recorder;
use crate::ui::theme::Theme;

use super::{centered_rect, overlay_block, section_header};

pub fn render_config(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(70, 75, area);
    frame.render_widget(Clear, popup);

    let block = overlay_block("🔧 Configuration  [←→] change  [q/Esc] save & close", Theme::border_active());

    let logo_style = Style::default().fg(Theme::border_active());
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(" ██████╗  ██████╗     ██████╗ ██╗     ██╗███╗   ███╗██████╗ ███████╗███████╗", logo_style)),
        Line::from(Span::styled(" ██╔══██╗██╔════╝    ██╔════╝ ██║     ██║████╗ ████║██╔══██╗██╔════╝██╔════╝", logo_style)),
        Line::from(Span::styled(" ██████╔╝██║  ███╗   ██║  ███╗██║     ██║██╔████╔██║██████╔╝███████╗█████╗  ", logo_style)),
        Line::from(Span::styled(" ██╔═══╝ ██║   ██║   ██║   ██║██║     ██║██║╚██╔╝██║██╔═══╝ ╚════██║██╔══╝  ", logo_style)),
        Line::from(Span::styled(" ██║     ╚██████╔╝   ╚██████╔╝███████╗██║██║ ╚═╝ ██║██║     ███████║███████╗", logo_style)),
        Line::from(Span::styled(" ╚═╝      ╚═════╝     ╚═════╝ ╚══════╝╚═╝╚═╝     ╚═╝╚═╝     ╚══════╝╚══════╝", logo_style)),
        Line::from(""),
        section_header("Settings"),
    ];

    let is_editing_dir = matches!(app.view_mode, ViewMode::ConfigEditRecordingsDir);

    for (i, item) in ConfigItem::ALL.iter().enumerate() {
        let selected = i == app.config_overlay.selected;
        let indicator = if selected { "▸ " } else { "  " };

        // Check if this item is being edited
        let is_editing_this = is_editing_dir && *item == ConfigItem::RecordingsDir;

        let value_str = match item {
            ConfigItem::GraphMarker => app.config.graph_marker.label().to_string(),
            ConfigItem::ColorTheme => app.config.color_theme.label().to_string(),
            ConfigItem::RefreshInterval => format!("{}s", app.config.refresh_interval_secs),
            ConfigItem::WarnDuration => format!("{:.1}s", app.config.warn_duration_secs),
            ConfigItem::DangerDuration => format!("{:.1}s", app.config.danger_duration_secs),
            ConfigItem::RecordingRetention => {
                let secs = app.config.recording_retention_secs;
                if secs >= 3600 {
                    format!("{}h", secs / 3600)
                } else {
                    format!("{}m", secs / 60)
                }
            }
            ConfigItem::RecordingsDir => {
                if is_editing_this {
                    format!("{}█", app.config_overlay.input_buffer)
                } else {
                    app.config
                        .recordings_dir
                        .clone()
                        .unwrap_or_else(|| Recorder::default_recordings_dir().to_string_lossy().into_owned())
                }
            }
        };

        let label_style = if selected {
            Style::default()
                .fg(Theme::border_active())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::fg())
        };

        let value_style = if is_editing_this {
            // Editing mode: different style
            Style::default()
                .fg(Theme::fg())
                .bg(Theme::highlight_bg())
        } else if selected {
            Style::default()
                .fg(Theme::overlay_bg())
                .bg(Theme::border_active())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::fg_dim())
        };

        let arrow_style = if selected && !is_editing_this {
            Style::default().fg(Theme::border_active())
        } else {
            Style::default().fg(Theme::border_dim())
        };

        // For RecordingsDir, show Enter hint instead of arrows when selected
        if *item == ConfigItem::RecordingsDir && selected && !is_editing_this {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}{:<20}", indicator, item.label()), label_style),
                Span::styled("[Enter] ", arrow_style),
                Span::styled(format!(" {value_str} "), value_style),
            ]));
        } else if is_editing_this {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}{:<20}", indicator, item.label()), label_style),
                Span::styled(format!(" {value_str} "), value_style),
                Span::styled(" [Enter] save  [Esc] cancel", Style::default().fg(Theme::fg_dim())),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}{:<20}", indicator, item.label()), label_style),
                Span::styled("◀ ", arrow_style),
                Span::styled(format!(" {value_str} "), value_style),
                Span::styled(" ▶", arrow_style),
            ]));
        }
    }

    // About section
    let label_style = Style::default().fg(Theme::fg_dim());
    let value_style = Style::default().fg(Theme::fg());
    let link_style = Style::default().fg(Theme::border_active());

    lines.push(Line::from(""));
    lines.push(section_header("About"));
    lines.push(Line::from(vec![
        Span::styled("    Version:    ", label_style),
        Span::styled(env!("CARGO_PKG_VERSION"), value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    License:    ", label_style),
        Span::styled("MIT", value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    Built with: ", label_style),
        Span::styled("Rust + ratatui + tokio-postgres", value_style),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    GitHub:     ", label_style),
        Span::styled("github.com/dlt/pg_glimpse", link_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    Issues:     ", label_style),
        Span::styled("github.com/dlt/pg_glimpse/issues", link_style),
    ]));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup);
}
