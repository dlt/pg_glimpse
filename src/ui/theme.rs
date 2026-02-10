use ratatui::style::{Color, Modifier, Style};
use std::sync::RwLock;

use crate::config::ThemeColors;

static ACTIVE_THEME: RwLock<ThemeColors> = RwLock::new(ThemeColors::TOKYO_NIGHT);
static DURATION_THRESHOLDS: RwLock<(f64, f64)> = RwLock::new((1.0, 10.0));

pub fn set_theme(colors: ThemeColors) {
    *ACTIVE_THEME.write().unwrap() = colors;
}

pub fn set_duration_thresholds(warn: f64, danger: f64) {
    *DURATION_THRESHOLDS.write().unwrap() = (warn, danger);
}

pub struct Theme;

impl Theme {
    pub fn header_bg() -> Color {
        ACTIVE_THEME.read().unwrap().header_bg
    }

    pub fn fg() -> Color {
        ACTIVE_THEME.read().unwrap().fg
    }

    pub fn fg_dim() -> Color {
        ACTIVE_THEME.read().unwrap().fg_dim
    }

    pub fn border_active() -> Color {
        ACTIVE_THEME.read().unwrap().border_active
    }

    pub fn border_warn() -> Color {
        ACTIVE_THEME.read().unwrap().border_warn
    }

    pub fn border_danger() -> Color {
        ACTIVE_THEME.read().unwrap().border_danger
    }

    pub fn border_ok() -> Color {
        ACTIVE_THEME.read().unwrap().border_ok
    }

    pub fn border_dim() -> Color {
        ACTIVE_THEME.read().unwrap().border_dim
    }

    pub fn graph_connections() -> Color {
        ACTIVE_THEME.read().unwrap().graph_connections
    }

    pub fn graph_cache() -> Color {
        ACTIVE_THEME.read().unwrap().graph_cache
    }

    pub fn graph_latency() -> Color {
        ACTIVE_THEME.read().unwrap().graph_latency
    }

    pub fn duration_ok() -> Color {
        ACTIVE_THEME.read().unwrap().duration_ok
    }

    pub fn duration_warn() -> Color {
        ACTIVE_THEME.read().unwrap().duration_warn
    }

    pub fn duration_danger() -> Color {
        ACTIVE_THEME.read().unwrap().duration_danger
    }

    pub fn state_active() -> Color {
        ACTIVE_THEME.read().unwrap().state_active
    }

    pub fn state_idle_txn() -> Color {
        ACTIVE_THEME.read().unwrap().state_idle_txn
    }

    pub fn overlay_bg() -> Color {
        ACTIVE_THEME.read().unwrap().overlay_bg
    }

    pub fn highlight_bg() -> Color {
        ACTIVE_THEME.read().unwrap().highlight_bg
    }

    pub fn sql_keyword() -> Color {
        ACTIVE_THEME.read().unwrap().sql_keyword
    }

    pub fn sql_string() -> Color {
        ACTIVE_THEME.read().unwrap().sql_string
    }

    pub fn sql_number() -> Color {
        ACTIVE_THEME.read().unwrap().sql_number
    }

    pub fn sql_comment() -> Color {
        ACTIVE_THEME.read().unwrap().sql_comment
    }

    pub fn title_style() -> Style {
        Style::default()
            .fg(Self::fg())
            .add_modifier(Modifier::BOLD)
    }

    pub fn border_style(color: Color) -> Style {
        Style::default().fg(color)
    }

    pub fn duration_color(secs: f64) -> Color {
        let (warn, danger) = *DURATION_THRESHOLDS.read().unwrap();
        if secs < warn {
            Self::duration_ok()
        } else if secs < danger {
            Self::duration_warn()
        } else {
            Self::duration_danger()
        }
    }

    pub fn state_color(state: Option<&str>) -> Color {
        match state {
            Some("active") => Self::state_active(),
            Some("idle in transaction" | "idle in transaction (aborted)") => {
                Self::state_idle_txn()
            }
            _ => Self::fg(),
        }
    }

    /// Color for buffer cache hit ratio (0.0-1.0 scale)
    pub fn hit_ratio_color(ratio: f64) -> Color {
        if ratio >= 0.99 {
            Self::border_ok()
        } else if ratio >= 0.90 {
            Self::border_warn()
        } else {
            Self::border_danger()
        }
    }

    /// Color for dead tuple ratio percentage
    pub fn dead_ratio_color(ratio: f64) -> Color {
        if ratio > 20.0 {
            Self::border_danger()
        } else if ratio > 5.0 {
            Self::border_warn()
        } else {
            Self::border_ok()
        }
    }

    /// Color for bloat percentage
    pub fn bloat_color(pct: f64) -> Color {
        if pct > 50.0 {
            Self::border_danger()
        } else if pct > 20.0 {
            Self::border_warn()
        } else {
            Self::border_ok()
        }
    }

    /// Color for transaction ID wraparound percentage
    pub fn wraparound_color(pct: f64) -> Color {
        if pct > 75.0 {
            Self::border_danger()
        } else if pct > 50.0 {
            Self::border_warn()
        } else {
            Self::border_ok()
        }
    }

    /// Color for index usage (0 scans = unused/danger)
    pub fn index_usage_color(scan_count: i64) -> Color {
        if scan_count == 0 {
            Self::border_danger()
        } else {
            Self::border_ok()
        }
    }

    /// Color for replication lag in seconds
    pub fn lag_color(secs: Option<f64>) -> Color {
        match secs {
            Some(s) if s > 10.0 => Self::border_danger(),
            Some(s) if s > 1.0 => Self::border_warn(),
            _ => Self::fg(),
        }
    }

    pub fn wait_event_color(event_type: &str) -> Color {
        match event_type {
            "Lock" => Color::Red,
            "IO" => Color::Yellow,
            "IPC" => Color::Magenta,
            "LWLock" => Color::Cyan,
            "Client" => Color::White,
            "BufferPin" => Color::LightBlue,
            "CPU/Running" => Color::Green,
            "Activity" => Color::DarkGray,
            _ => Color::Gray,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // Reset theme to default before each test to avoid cross-test pollution
    fn setup() {
        set_theme(ThemeColors::TOKYO_NIGHT);
        set_duration_thresholds(1.0, 10.0);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // duration_color boundary tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn duration_color_below_warn() {
        setup();
        // Anything < 1.0 should be OK (green)
        let color = Theme::duration_color(0.0);
        assert_eq!(color, Theme::duration_ok());

        let color = Theme::duration_color(0.5);
        assert_eq!(color, Theme::duration_ok());

        let color = Theme::duration_color(0.999);
        assert_eq!(color, Theme::duration_ok());
    }

    #[test]
    #[serial]
    fn duration_color_at_warn_boundary() {
        setup();
        // Exactly at warn threshold (1.0) should be warn
        let color = Theme::duration_color(1.0);
        assert_eq!(color, Theme::duration_warn());
    }

    #[test]
    #[serial]
    fn duration_color_between_warn_and_danger() {
        setup();
        // Between 1.0 and 10.0 should be warn
        let color = Theme::duration_color(1.5);
        assert_eq!(color, Theme::duration_warn());

        let color = Theme::duration_color(5.0);
        assert_eq!(color, Theme::duration_warn());

        let color = Theme::duration_color(9.999);
        assert_eq!(color, Theme::duration_warn());
    }

    #[test]
    #[serial]
    fn duration_color_at_danger_boundary() {
        setup();
        // At danger threshold (10.0) should be danger
        let color = Theme::duration_color(10.0);
        assert_eq!(color, Theme::duration_danger());
    }

    #[test]
    #[serial]
    fn duration_color_above_danger() {
        setup();
        // Above 10.0 should be danger
        let color = Theme::duration_color(10.1);
        assert_eq!(color, Theme::duration_danger());

        let color = Theme::duration_color(100.0);
        assert_eq!(color, Theme::duration_danger());

        let color = Theme::duration_color(f64::MAX);
        assert_eq!(color, Theme::duration_danger());
    }

    #[test]
    #[serial]
    fn duration_color_with_custom_thresholds() {
        setup();
        set_duration_thresholds(5.0, 30.0);

        assert_eq!(Theme::duration_color(4.9), Theme::duration_ok());
        assert_eq!(Theme::duration_color(5.0), Theme::duration_warn());
        assert_eq!(Theme::duration_color(29.9), Theme::duration_warn());
        assert_eq!(Theme::duration_color(30.0), Theme::duration_danger());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // state_color tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn state_color_active() {
        setup();
        let color = Theme::state_color(Some("active"));
        assert_eq!(color, Theme::state_active());
    }

    #[test]
    #[serial]
    fn state_color_idle_in_transaction() {
        setup();
        let color = Theme::state_color(Some("idle in transaction"));
        assert_eq!(color, Theme::state_idle_txn());
    }

    #[test]
    #[serial]
    fn state_color_idle_in_transaction_aborted() {
        setup();
        let color = Theme::state_color(Some("idle in transaction (aborted)"));
        assert_eq!(color, Theme::state_idle_txn());
    }

    #[test]
    #[serial]
    fn state_color_idle() {
        setup();
        // Plain idle should get default fg color
        let color = Theme::state_color(Some("idle"));
        assert_eq!(color, Theme::fg());
    }

    #[test]
    #[serial]
    fn state_color_none() {
        setup();
        let color = Theme::state_color(None);
        assert_eq!(color, Theme::fg());
    }

    #[test]
    #[serial]
    fn state_color_unknown() {
        setup();
        let color = Theme::state_color(Some("something else"));
        assert_eq!(color, Theme::fg());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // hit_ratio_color boundary tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn hit_ratio_color_excellent() {
        setup();
        // >= 0.99 should be OK (green)
        assert_eq!(Theme::hit_ratio_color(1.0), Theme::border_ok());
        assert_eq!(Theme::hit_ratio_color(0.999), Theme::border_ok());
        assert_eq!(Theme::hit_ratio_color(0.99), Theme::border_ok());
    }

    #[test]
    #[serial]
    fn hit_ratio_color_good() {
        setup();
        // >= 0.90 but < 0.99 should be warn
        assert_eq!(Theme::hit_ratio_color(0.989), Theme::border_warn());
        assert_eq!(Theme::hit_ratio_color(0.95), Theme::border_warn());
        assert_eq!(Theme::hit_ratio_color(0.90), Theme::border_warn());
    }

    #[test]
    #[serial]
    fn hit_ratio_color_bad() {
        setup();
        // < 0.90 should be danger
        assert_eq!(Theme::hit_ratio_color(0.899), Theme::border_danger());
        assert_eq!(Theme::hit_ratio_color(0.5), Theme::border_danger());
        assert_eq!(Theme::hit_ratio_color(0.0), Theme::border_danger());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // dead_ratio_color boundary tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn dead_ratio_color_ok() {
        setup();
        // <= 5% should be OK
        assert_eq!(Theme::dead_ratio_color(0.0), Theme::border_ok());
        assert_eq!(Theme::dead_ratio_color(2.5), Theme::border_ok());
        assert_eq!(Theme::dead_ratio_color(5.0), Theme::border_ok());
    }

    #[test]
    #[serial]
    fn dead_ratio_color_warn() {
        setup();
        // > 5% but <= 20% should be warn
        assert_eq!(Theme::dead_ratio_color(5.1), Theme::border_warn());
        assert_eq!(Theme::dead_ratio_color(10.0), Theme::border_warn());
        assert_eq!(Theme::dead_ratio_color(20.0), Theme::border_warn());
    }

    #[test]
    #[serial]
    fn dead_ratio_color_danger() {
        setup();
        // > 20% should be danger
        assert_eq!(Theme::dead_ratio_color(20.1), Theme::border_danger());
        assert_eq!(Theme::dead_ratio_color(50.0), Theme::border_danger());
        assert_eq!(Theme::dead_ratio_color(100.0), Theme::border_danger());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // bloat_color boundary tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn bloat_color_ok() {
        setup();
        // <= 20% should be OK
        assert_eq!(Theme::bloat_color(0.0), Theme::border_ok());
        assert_eq!(Theme::bloat_color(10.0), Theme::border_ok());
        assert_eq!(Theme::bloat_color(20.0), Theme::border_ok());
    }

    #[test]
    #[serial]
    fn bloat_color_warn() {
        setup();
        // > 20% but <= 50% should be warn
        assert_eq!(Theme::bloat_color(20.1), Theme::border_warn());
        assert_eq!(Theme::bloat_color(35.0), Theme::border_warn());
        assert_eq!(Theme::bloat_color(50.0), Theme::border_warn());
    }

    #[test]
    #[serial]
    fn bloat_color_danger() {
        setup();
        // > 50% should be danger
        assert_eq!(Theme::bloat_color(50.1), Theme::border_danger());
        assert_eq!(Theme::bloat_color(75.0), Theme::border_danger());
        assert_eq!(Theme::bloat_color(100.0), Theme::border_danger());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // wraparound_color boundary tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn wraparound_color_ok() {
        setup();
        // <= 50% should be OK
        assert_eq!(Theme::wraparound_color(0.0), Theme::border_ok());
        assert_eq!(Theme::wraparound_color(25.0), Theme::border_ok());
        assert_eq!(Theme::wraparound_color(50.0), Theme::border_ok());
    }

    #[test]
    #[serial]
    fn wraparound_color_warn() {
        setup();
        // > 50% but <= 75% should be warn
        assert_eq!(Theme::wraparound_color(50.1), Theme::border_warn());
        assert_eq!(Theme::wraparound_color(60.0), Theme::border_warn());
        assert_eq!(Theme::wraparound_color(75.0), Theme::border_warn());
    }

    #[test]
    #[serial]
    fn wraparound_color_danger() {
        setup();
        // > 75% should be danger
        assert_eq!(Theme::wraparound_color(75.1), Theme::border_danger());
        assert_eq!(Theme::wraparound_color(90.0), Theme::border_danger());
        assert_eq!(Theme::wraparound_color(100.0), Theme::border_danger());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // index_usage_color tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn index_usage_color_unused() {
        setup();
        // 0 scans = danger (unused index)
        assert_eq!(Theme::index_usage_color(0), Theme::border_danger());
    }

    #[test]
    #[serial]
    fn index_usage_color_used() {
        setup();
        // Any scans > 0 = OK
        assert_eq!(Theme::index_usage_color(1), Theme::border_ok());
        assert_eq!(Theme::index_usage_color(100), Theme::border_ok());
        assert_eq!(Theme::index_usage_color(1_000_000), Theme::border_ok());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // lag_color tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn lag_color_none() {
        setup();
        assert_eq!(Theme::lag_color(None), Theme::fg());
    }

    #[test]
    #[serial]
    fn lag_color_low() {
        setup();
        // <= 1.0 sec is fine
        assert_eq!(Theme::lag_color(Some(0.0)), Theme::fg());
        assert_eq!(Theme::lag_color(Some(0.5)), Theme::fg());
        assert_eq!(Theme::lag_color(Some(1.0)), Theme::fg());
    }

    #[test]
    #[serial]
    fn lag_color_medium() {
        setup();
        // > 1.0 but <= 10.0 is warn
        assert_eq!(Theme::lag_color(Some(1.1)), Theme::border_warn());
        assert_eq!(Theme::lag_color(Some(5.0)), Theme::border_warn());
        assert_eq!(Theme::lag_color(Some(10.0)), Theme::border_warn());
    }

    #[test]
    #[serial]
    fn lag_color_high() {
        setup();
        // > 10.0 is danger
        assert_eq!(Theme::lag_color(Some(10.1)), Theme::border_danger());
        assert_eq!(Theme::lag_color(Some(30.0)), Theme::border_danger());
        assert_eq!(Theme::lag_color(Some(100.0)), Theme::border_danger());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // wait_event_color tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn wait_event_color_known_types() {
        assert_eq!(Theme::wait_event_color("Lock"), Color::Red);
        assert_eq!(Theme::wait_event_color("IO"), Color::Yellow);
        assert_eq!(Theme::wait_event_color("IPC"), Color::Magenta);
        assert_eq!(Theme::wait_event_color("LWLock"), Color::Cyan);
        assert_eq!(Theme::wait_event_color("Client"), Color::White);
        assert_eq!(Theme::wait_event_color("BufferPin"), Color::LightBlue);
        assert_eq!(Theme::wait_event_color("CPU/Running"), Color::Green);
        assert_eq!(Theme::wait_event_color("Activity"), Color::DarkGray);
    }

    #[test]
    fn wait_event_color_unknown() {
        assert_eq!(Theme::wait_event_color("Unknown"), Color::Gray);
        assert_eq!(Theme::wait_event_color(""), Color::Gray);
        assert_eq!(Theme::wait_event_color("SomethingNew"), Color::Gray);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Theme accessors
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    #[serial]
    fn theme_accessors_return_colors() {
        setup();
        // Just verify accessors don't panic and return some color
        let _ = Theme::header_bg();
        let _ = Theme::fg();
        let _ = Theme::fg_dim();
        let _ = Theme::border_active();
        let _ = Theme::border_warn();
        let _ = Theme::border_danger();
        let _ = Theme::border_ok();
        let _ = Theme::border_dim();
        let _ = Theme::graph_connections();
        let _ = Theme::graph_cache();
        let _ = Theme::graph_latency();
        let _ = Theme::duration_ok();
        let _ = Theme::duration_warn();
        let _ = Theme::duration_danger();
        let _ = Theme::state_active();
        let _ = Theme::state_idle_txn();
        let _ = Theme::overlay_bg();
        let _ = Theme::highlight_bg();
        let _ = Theme::sql_keyword();
        let _ = Theme::sql_string();
        let _ = Theme::sql_number();
        let _ = Theme::sql_comment();
    }

    #[test]
    #[serial]
    fn title_style_is_bold() {
        setup();
        let style = Theme::title_style();
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    #[serial]
    fn border_style_applies_color() {
        setup();
        let style = Theme::border_style(Color::Red);
        assert_eq!(style.fg, Some(Color::Red));
    }
}
