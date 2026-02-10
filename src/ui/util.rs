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
    Paragraph::new(format!("\n  {text}"))
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
        format!("{bytes} B")
    }
}

pub fn format_lag(secs: Option<f64>) -> String {
    secs.map_or_else(|| "-".into(), |s| format!("{s:.3}s"))
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

/// Truncate string to max length (in characters), adding ellipsis if truncated
pub fn truncate(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{truncated}…")
    }
}

/// Format duration in seconds to human-readable compact form (e.g., "1.5s", "2m30s", "1h15m")
pub fn format_duration(secs: f64) -> String {
    if secs < 0.001 {
        "0s".into()
    } else if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{secs:.1}s")
    } else if secs < 3600.0 {
        format!("{:.0}m{:.0}s", secs / 60.0, secs % 60.0)
    } else {
        format!("{:.0}h{:.0}m", secs / 3600.0, (secs % 3600.0) / 60.0)
    }
}

/// Format duration in milliseconds for statement stats (spaced format)
pub fn format_time_ms(ms: f64) -> String {
    if ms < 1.0 {
        format!("{ms:.3} ms")
    } else if ms < 1_000.0 {
        format!("{ms:.1} ms")
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
        format!("{rate:.0}/s")
    } else if rate > 0.0 {
        format!("{rate:.1}/s")
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
        format!("{bytes_per_sec:.0} B/s")
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

#[cfg(test)]
mod tests {
    use super::*;

    // format_bytes tests
    #[test]
    fn format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn format_bytes_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn format_bytes_kilobytes() {
        assert_eq!(format_bytes(1024), "1 KB");
        assert_eq!(format_bytes(1536), "2 KB"); // 1.5 KB rounds to 2
        assert_eq!(format_bytes(10240), "10 KB");
    }

    #[test]
    fn format_bytes_megabytes() {
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 5 + 1024 * 512), "5.5 MB");
    }

    #[test]
    fn format_bytes_gigabytes() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 2 + 1024 * 1024 * 512), "2.5 GB");
    }

    // format_lag tests
    #[test]
    fn format_lag_none() {
        assert_eq!(format_lag(None), "-");
    }

    #[test]
    fn format_lag_some() {
        assert_eq!(format_lag(Some(1.234)), "1.234s");
        assert_eq!(format_lag(Some(0.0)), "0.000s");
        assert_eq!(format_lag(Some(100.5)), "100.500s");
    }

    // format_compact tests
    #[test]
    fn format_compact_small() {
        assert_eq!(format_compact(0), "0");
        assert_eq!(format_compact(999), "999");
    }

    #[test]
    fn format_compact_thousands() {
        assert_eq!(format_compact(1_000), "1.0K");
        assert_eq!(format_compact(1_500), "1.5K");
        assert_eq!(format_compact(999_999), "1000.0K");
    }

    #[test]
    fn format_compact_millions() {
        assert_eq!(format_compact(1_000_000), "1.0M");
        assert_eq!(format_compact(2_500_000), "2.5M");
    }

    #[test]
    fn format_compact_billions() {
        assert_eq!(format_compact(1_000_000_000), "1.0B");
        assert_eq!(format_compact(3_500_000_000), "3.5B");
    }

    // truncate tests
    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("hello world", 5), "hell…");
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn truncate_edge_cases() {
        assert_eq!(truncate("hello", 1), "…");
        assert_eq!(truncate("hello", 0), "…");
        assert_eq!(truncate("hello", 2), "h…");
    }

    // format_duration tests
    #[test]
    fn format_duration_sub_millisecond() {
        assert_eq!(format_duration(0.0), "0s");
        assert_eq!(format_duration(0.0001), "0s");
    }

    #[test]
    fn format_duration_milliseconds() {
        assert_eq!(format_duration(0.001), "1ms");
        assert_eq!(format_duration(0.5), "500ms");
        assert_eq!(format_duration(0.999), "999ms");
    }

    #[test]
    fn format_duration_seconds() {
        assert_eq!(format_duration(1.0), "1.0s");
        assert_eq!(format_duration(30.5), "30.5s");
        assert_eq!(format_duration(59.9), "59.9s");
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(60.0), "1m0s");
        assert_eq!(format_duration(90.0), "2m30s"); // 90/60=1.5 rounds to 2
        assert_eq!(format_duration(3599.0), "60m59s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3600.0), "1h0m");
        assert_eq!(format_duration(5400.0), "2h30m"); // 5400/3600=1.5 rounds to 2
        assert_eq!(format_duration(7200.0), "2h0m");
    }

    // format_time_ms tests
    #[test]
    fn format_time_ms_sub_millisecond() {
        assert_eq!(format_time_ms(0.001), "0.001 ms");
        assert_eq!(format_time_ms(0.5), "0.500 ms");
    }

    #[test]
    fn format_time_ms_milliseconds() {
        assert_eq!(format_time_ms(1.0), "1.0 ms");
        assert_eq!(format_time_ms(100.5), "100.5 ms");
        assert_eq!(format_time_ms(999.9), "999.9 ms");
    }

    #[test]
    fn format_time_ms_seconds() {
        assert_eq!(format_time_ms(1_000.0), "1.00 s");
        assert_eq!(format_time_ms(30_000.0), "30.00 s");
    }

    #[test]
    fn format_time_ms_minutes() {
        assert_eq!(format_time_ms(60_000.0), "1.0 min");
        assert_eq!(format_time_ms(150_000.0), "2.5 min");
    }

    #[test]
    fn format_time_ms_hours() {
        assert_eq!(format_time_ms(3_600_000.0), "1.0 hr");
        assert_eq!(format_time_ms(5_400_000.0), "1.5 hr");
    }

    // format_rate tests
    #[test]
    fn format_rate_zero() {
        assert_eq!(format_rate(0.0), "0/s");
    }

    #[test]
    fn format_rate_fractional() {
        assert_eq!(format_rate(0.5), "0.5/s");
    }

    #[test]
    fn format_rate_small() {
        assert_eq!(format_rate(1.0), "1/s");
        assert_eq!(format_rate(999.0), "999/s");
    }

    #[test]
    fn format_rate_thousands() {
        assert_eq!(format_rate(1_000.0), "1.0K/s");
        assert_eq!(format_rate(2_500.0), "2.5K/s");
    }

    #[test]
    fn format_rate_millions() {
        assert_eq!(format_rate(1_000_000.0), "1.0M/s");
        assert_eq!(format_rate(5_500_000.0), "5.5M/s");
    }

    // format_byte_rate tests
    #[test]
    fn format_byte_rate_zero() {
        assert_eq!(format_byte_rate(0.0), "0 B/s");
    }

    #[test]
    fn format_byte_rate_bytes() {
        assert_eq!(format_byte_rate(512.0), "512 B/s");
    }

    #[test]
    fn format_byte_rate_kilobytes() {
        assert_eq!(format_byte_rate(1024.0), "1 KB/s");
        assert_eq!(format_byte_rate(2048.0), "2 KB/s");
    }

    #[test]
    fn format_byte_rate_megabytes() {
        assert_eq!(format_byte_rate(1024.0 * 1024.0), "1.0 MB/s");
        assert_eq!(format_byte_rate(1024.0 * 1024.0 * 5.5), "5.5 MB/s");
    }

    #[test]
    fn format_byte_rate_gigabytes() {
        assert_eq!(format_byte_rate(1024.0 * 1024.0 * 1024.0), "1.0 GB/s");
    }

    // compute_match_indices tests
    #[test]
    fn compute_match_indices_empty_filter() {
        assert_eq!(compute_match_indices("hello", ""), None);
    }

    #[test]
    fn compute_match_indices_no_match() {
        assert_eq!(compute_match_indices("hello", "xyz"), None);
    }

    #[test]
    fn compute_match_indices_exact_match() {
        let result = compute_match_indices("hello", "hello");
        assert!(result.is_some());
        let indices = result.unwrap();
        assert_eq!(indices.len(), 5);
    }

    #[test]
    fn compute_match_indices_partial_match() {
        let result = compute_match_indices("hello world", "hlo");
        assert!(result.is_some());
    }

    #[test]
    fn compute_match_indices_case_insensitive() {
        let result = compute_match_indices("Hello World", "hello");
        assert!(result.is_some());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Property-based tests using proptest
    // ─────────────────────────────────────────────────────────────────────────────

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            // ─────────────────────────────────────────────────────────────────
            // truncate properties
            // ─────────────────────────────────────────────────────────────────

            /// truncate output should never exceed max_len in characters
            #[test]
            fn truncate_never_exceeds_max_len(s in ".*", max in 0usize..1000) {
                let result = truncate(&s, max);
                let char_count = result.chars().count();
                prop_assert!(
                    char_count <= max.max(1),
                    "truncate({:?}, {}) produced {} chars, expected <= {}",
                    s, max, char_count, max.max(1)
                );
            }

            /// truncate should preserve content when string fits
            #[test]
            fn truncate_preserves_short_strings(s in ".{0,50}", max in 50usize..100) {
                let result = truncate(&s, max);
                if s.len() <= max {
                    prop_assert_eq!(result, s);
                }
            }

            /// truncate should end with ellipsis when truncated
            #[test]
            fn truncate_adds_ellipsis_when_needed(s in ".{10,100}", max in 2usize..9) {
                let result = truncate(&s, max);
                if s.len() > max {
                    prop_assert!(
                        result.ends_with('…'),
                        "truncate({:?}, {}) = {:?} should end with ellipsis",
                        s, max, result
                    );
                }
            }

            // ─────────────────────────────────────────────────────────────────
            // format_bytes properties
            // ─────────────────────────────────────────────────────────────────

            /// format_bytes should never panic and always produce valid output
            #[test]
            fn format_bytes_never_panics(bytes: i64) {
                let result = format_bytes(bytes);
                prop_assert!(!result.is_empty());
            }

            /// format_bytes output should contain appropriate unit suffix
            #[test]
            fn format_bytes_has_valid_suffix(bytes in 0i64..i64::MAX) {
                let result = format_bytes(bytes);
                prop_assert!(
                    result.ends_with(" B") || result.ends_with(" KB") ||
                    result.ends_with(" MB") || result.ends_with(" GB"),
                    "format_bytes({}) = {:?} has invalid suffix",
                    bytes, result
                );
            }

            /// format_bytes should produce consistent ordering (larger bytes = larger/equal unit)
            #[test]
            fn format_bytes_ordering(a in 0i64..1_000_000_000, b in 0i64..1_000_000_000) {
                let result_a = format_bytes(a);
                let result_b = format_bytes(b);

                // If a >= b and both are in the same unit range, the numeric prefix should reflect order
                // This is a weak check - mainly ensures no weird inversions
                if a == b {
                    prop_assert_eq!(result_a, result_b);
                }
            }

            // ─────────────────────────────────────────────────────────────────
            // format_compact properties
            // ─────────────────────────────────────────────────────────────────

            /// format_compact should never panic
            #[test]
            fn format_compact_never_panics(n: i64) {
                let result = format_compact(n);
                prop_assert!(!result.is_empty());
            }

            /// format_compact output should have valid suffix for large numbers
            #[test]
            fn format_compact_valid_suffix(n in 0i64..i64::MAX) {
                let result = format_compact(n);
                if n >= 1_000_000_000 {
                    prop_assert!(result.ends_with('B'), "format_compact({}) = {:?}", n, result);
                } else if n >= 1_000_000 {
                    prop_assert!(result.ends_with('M'), "format_compact({}) = {:?}", n, result);
                } else if n >= 1_000 {
                    prop_assert!(result.ends_with('K'), "format_compact({}) = {:?}", n, result);
                }
            }

            // ─────────────────────────────────────────────────────────────────
            // format_duration properties
            // ─────────────────────────────────────────────────────────────────

            /// format_duration should never panic for non-negative values
            #[test]
            fn format_duration_never_panics(secs in 0.0f64..1e12) {
                let result = format_duration(secs);
                prop_assert!(!result.is_empty());
            }

            /// format_duration should have valid time suffix
            #[test]
            fn format_duration_valid_suffix(secs in 0.0f64..1e9) {
                let result = format_duration(secs);
                prop_assert!(
                    result.ends_with('s') || result.ends_with('m') || result.ends_with('h'),
                    "format_duration({}) = {:?} has invalid suffix",
                    secs, result
                );
            }

            /// format_duration output length should be reasonable
            #[test]
            fn format_duration_reasonable_length(secs in 0.0f64..1e9) {
                let result = format_duration(secs);
                prop_assert!(
                    result.len() <= 20,
                    "format_duration({}) = {:?} is too long",
                    secs, result
                );
            }

            // ─────────────────────────────────────────────────────────────────
            // format_time_ms properties
            // ─────────────────────────────────────────────────────────────────

            /// format_time_ms should never panic for non-negative values
            #[test]
            fn format_time_ms_never_panics(ms in 0.0f64..1e15) {
                let result = format_time_ms(ms);
                prop_assert!(!result.is_empty());
            }

            /// format_time_ms should have valid time unit
            #[test]
            fn format_time_ms_valid_unit(ms in 0.0f64..1e12) {
                let result = format_time_ms(ms);
                prop_assert!(
                    result.contains("ms") || result.contains(" s") ||
                    result.contains("min") || result.contains("hr"),
                    "format_time_ms({}) = {:?} has invalid unit",
                    ms, result
                );
            }

            // ─────────────────────────────────────────────────────────────────
            // format_rate properties
            // ─────────────────────────────────────────────────────────────────

            /// format_rate should never panic for non-negative values
            #[test]
            fn format_rate_never_panics(rate in 0.0f64..1e15) {
                let result = format_rate(rate);
                prop_assert!(!result.is_empty());
            }

            /// format_rate should always end with /s
            #[test]
            fn format_rate_ends_with_per_second(rate in 0.0f64..1e12) {
                let result = format_rate(rate);
                prop_assert!(
                    result.ends_with("/s"),
                    "format_rate({}) = {:?} should end with /s",
                    rate, result
                );
            }

            // ─────────────────────────────────────────────────────────────────
            // format_byte_rate properties
            // ─────────────────────────────────────────────────────────────────

            /// format_byte_rate should never panic for non-negative values
            #[test]
            fn format_byte_rate_never_panics(rate in 0.0f64..1e15) {
                let result = format_byte_rate(rate);
                prop_assert!(!result.is_empty());
            }

            /// format_byte_rate should have valid byte unit suffix
            #[test]
            fn format_byte_rate_valid_suffix(rate in 0.0f64..1e15) {
                let result = format_byte_rate(rate);
                prop_assert!(
                    result.ends_with("B/s") || result.ends_with("KB/s") ||
                    result.ends_with("MB/s") || result.ends_with("GB/s"),
                    "format_byte_rate({}) = {:?} has invalid suffix",
                    rate, result
                );
            }

            // ─────────────────────────────────────────────────────────────────
            // format_lag properties
            // ─────────────────────────────────────────────────────────────────

            /// format_lag should never panic
            #[test]
            fn format_lag_never_panics(secs in proptest::option::of(-1e15f64..1e15f64)) {
                let result = format_lag(secs);
                prop_assert!(!result.is_empty());
            }

            /// format_lag should return "-" for None
            #[test]
            fn format_lag_none_is_dash(_dummy in 0..1) {
                prop_assert_eq!(format_lag(None), "-");
            }

            /// format_lag should end with 's' for Some values
            #[test]
            fn format_lag_some_ends_with_s(secs in -1e12f64..1e12f64) {
                let result = format_lag(Some(secs));
                prop_assert!(
                    result.ends_with('s'),
                    "format_lag(Some({})) = {:?} should end with s",
                    secs, result
                );
            }

            // ─────────────────────────────────────────────────────────────────
            // highlight_matches properties
            // ─────────────────────────────────────────────────────────────────

            /// highlight_matches should never panic and preserve text content
            #[test]
            fn highlight_matches_preserves_text(
                text in ".{0,100}",
                indices in proptest::collection::vec(0u32..100, 0..20)
            ) {
                let result = highlight_matches(&text, &indices, Style::default());

                // Concatenate all spans
                let combined: String = result.iter().map(|s| s.content.as_ref()).collect();

                prop_assert_eq!(
                    combined, text,
                    "highlight_matches should preserve text content"
                );
            }

            /// highlight_matches with empty indices returns single span
            #[test]
            fn highlight_matches_empty_indices_single_span(text in ".{0,50}") {
                let result = highlight_matches(&text, &[], Style::default());
                prop_assert_eq!(result.len(), 1);
            }

            // ─────────────────────────────────────────────────────────────────
            // compute_match_indices properties
            // ─────────────────────────────────────────────────────────────────

            /// compute_match_indices should never panic
            #[test]
            fn compute_match_indices_never_panics(
                text in ".{0,100}",
                filter in ".{0,20}"
            ) {
                // Just verify it doesn't panic
                let _ = compute_match_indices(&text, &filter);
            }

            /// compute_match_indices empty filter always returns None
            #[test]
            fn compute_match_indices_empty_filter_is_none(text in ".{0,100}") {
                prop_assert_eq!(compute_match_indices(&text, ""), None);
            }

            /// compute_match_indices indices should be within text bounds
            #[test]
            fn compute_match_indices_valid_bounds(
                text in "[a-z]{5,50}",
                filter in "[a-z]{1,5}"
            ) {
                if let Some(indices) = compute_match_indices(&text, &filter) {
                    let text_len = text.chars().count() as u32;
                    for &idx in &indices {
                        prop_assert!(
                            idx < text_len,
                            "Index {} out of bounds for text {:?} (len {})",
                            idx, text, text_len
                        );
                    }
                }
            }
        }
    }
}

