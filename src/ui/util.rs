use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Paragraph};

use super::theme::Theme;

/// Create a styled empty state message for panels with no data
pub fn empty_state<'a>(text: &'a str, block: Block<'a>) -> Paragraph<'a> {
    Paragraph::new(format!("\n  {}", text))
        .style(
            Style::default()
                .fg(Theme::border_ok())
                .add_modifier(Modifier::ITALIC),
        )
        .block(block)
}

pub fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = 1024 * 1024;
    const GB: i64 = 1024 * 1024 * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_lag(secs: Option<f64>) -> String {
    match secs {
        Some(s) => format!("{:.3}s", s),
        None => "-".into(),
    }
}

pub fn lag_color(secs: Option<f64>) -> Color {
    match secs {
        Some(s) if s > 10.0 => Theme::border_danger(),
        Some(s) if s > 1.0 => Theme::border_warn(),
        _ => Theme::fg(),
    }
}

/// Format large numbers compactly (e.g., 1.5K, 2.3M, 1.0B)
pub fn format_compact(n: i64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

pub fn format_time_ms(ms: f64) -> String {
    if ms < 1.0 {
        format!("{:.3} ms", ms)
    } else if ms < 1_000.0 {
        format!("{:.1} ms", ms)
    } else if ms < 60_000.0 {
        format!("{:.2} s", ms / 1_000.0)
    } else if ms < 3_600_000.0 {
        format!("{:.1} min", ms / 60_000.0)
    } else {
        format!("{:.1} hr", ms / 3_600_000.0)
    }
}

