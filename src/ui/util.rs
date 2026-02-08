use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Paragraph, Row, Table};

use super::theme::Theme;

/// Create a styled table with consistent highlight behavior
pub fn styled_table<'a>(
    rows: Vec<Row<'a>>,
    widths: impl IntoIterator<Item = Constraint>,
    header: Row<'a>,
    block: Block<'a>,
) -> Table<'a> {
    Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Theme::highlight_bg())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25ba} ")
}

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

/// Truncate string to max length, adding ellipsis if truncated
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

/// Format duration in seconds to human-readable compact form (e.g., "1.5s", "2m30s", "1h15m")
pub fn format_duration(secs: f64) -> String {
    if secs < 0.001 {
        "0s".into()
    } else if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        format!("{:.0}m{:.0}s", secs / 60.0, secs % 60.0)
    } else {
        format!("{:.0}h{:.0}m", secs / 3600.0, (secs % 3600.0) / 60.0)
    }
}

/// Format duration in milliseconds for statement stats (spaced format)
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

/// Format a rate (per second) with appropriate unit suffix
pub fn format_rate(rate: f64) -> String {
    if rate >= 1_000_000.0 {
        format!("{:.1}M/s", rate / 1_000_000.0)
    } else if rate >= 1_000.0 {
        format!("{:.1}K/s", rate / 1_000.0)
    } else if rate >= 1.0 {
        format!("{:.0}/s", rate)
    } else if rate > 0.0 {
        format!("{:.1}/s", rate)
    } else {
        "0/s".into()
    }
}

/// Format a byte rate (bytes per second) with appropriate unit suffix
pub fn format_byte_rate(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    if bytes_per_sec >= GB {
        format!("{:.1} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.1} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:.0} KB/s", bytes_per_sec / KB)
    } else if bytes_per_sec > 0.0 {
        format!("{:.0} B/s", bytes_per_sec)
    } else {
        "0 B/s".into()
    }
}

/// Highlight matching characters in a string based on fuzzy match indices.
/// The `match_indices` are character positions from nucleo.
/// Returns owned Spans to avoid lifetime issues.
pub fn highlight_matches(
    text: &str,
    match_indices: &[u32],
    base_style: Style,
) -> Vec<Span<'static>> {
    if match_indices.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    let highlight_style = base_style
        .fg(Theme::border_active())
        .add_modifier(Modifier::BOLD);

    let match_set: std::collections::HashSet<usize> = match_indices
        .iter()
        .map(|&idx| idx as usize)
        .collect();

    let mut spans = Vec::new();
    let mut current_span = String::new();
    let mut current_is_match = false;

    for (char_idx, ch) in text.chars().enumerate() {
        let is_match = match_set.contains(&char_idx);

        if is_match != current_is_match && !current_span.is_empty() {
            let style = if current_is_match {
                highlight_style
            } else {
                base_style
            };
            spans.push(Span::styled(std::mem::take(&mut current_span), style));
        }

        current_span.push(ch);
        current_is_match = is_match;
    }

    if !current_span.is_empty() {
        let style = if current_is_match {
            highlight_style
        } else {
            base_style
        };
        spans.push(Span::styled(current_span, style));
    }

    spans
}

/// Compute fuzzy match indices for a text against a filter pattern.
/// Returns None if no match, Some(indices) if match.
pub fn compute_match_indices(text: &str, filter: &str) -> Option<Vec<u32>> {
    use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
    use nucleo_matcher::{Config as MatcherConfig, Matcher};

    if filter.is_empty() {
        return None;
    }

    let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
    let pattern = Pattern::parse(filter, CaseMatching::Ignore, Normalization::Smart);
    let mut buf = Vec::new();
    let utf32 = nucleo_matcher::Utf32Str::new(text, &mut buf);

    if pattern.score(utf32.slice(..), &mut matcher).is_some() {
        let mut indices = Vec::new();
        pattern.indices(utf32.slice(..), &mut matcher, &mut indices);
        Some(indices)
    } else {
        None
    }
}

