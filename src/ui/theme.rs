use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub const HEADER_BG: Color = Color::Rgb(36, 40, 59);
    pub const FG: Color = Color::Rgb(192, 202, 245);
    pub const BORDER_ACTIVE: Color = Color::Cyan;
    pub const BORDER_WARN: Color = Color::Yellow;
    pub const BORDER_DANGER: Color = Color::Red;
    pub const BORDER_OK: Color = Color::Green;
    pub const BORDER_DIM: Color = Color::Rgb(68, 71, 90);

    pub const GRAPH_CONNECTIONS: Color = Color::Rgb(97, 175, 239);
    pub const GRAPH_QUERIES: Color = Color::Rgb(152, 195, 121);
    pub const GRAPH_CACHE: Color = Color::Rgb(86, 182, 194);
    pub const GRAPH_LOCKS: Color = Color::Rgb(224, 108, 117);

    pub const DURATION_OK: Color = Color::Green;
    pub const DURATION_WARN: Color = Color::Yellow;
    pub const DURATION_DANGER: Color = Color::Red;

    pub const STATE_ACTIVE: Color = Color::Green;
    pub const STATE_IDLE_TXN: Color = Color::Yellow;

    pub fn title_style() -> Style {
        Style::default()
            .fg(Self::FG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border_style(color: Color) -> Style {
        Style::default().fg(color)
    }

    pub fn duration_color(secs: f64) -> Color {
        if secs < 1.0 {
            Self::DURATION_OK
        } else if secs < 10.0 {
            Self::DURATION_WARN
        } else {
            Self::DURATION_DANGER
        }
    }

    pub fn state_color(state: Option<&str>) -> Color {
        match state {
            Some("active") => Self::STATE_ACTIVE,
            Some("idle in transaction") | Some("idle in transaction (aborted)") => {
                Self::STATE_IDLE_TXN
            }
            _ => Self::FG,
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
