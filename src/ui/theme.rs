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
            Some("idle in transaction") | Some("idle in transaction (aborted)") => {
                Self::state_idle_txn()
            }
            _ => Self::fg(),
        }
    }

    pub fn hit_ratio_color(ratio_pct: f64) -> Color {
        if ratio_pct >= 99.0 {
            Color::Green
        } else if ratio_pct >= 95.0 {
            Color::Yellow
        } else {
            Color::Red
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
