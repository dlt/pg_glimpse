use ratatui::style::Color;
use ratatui::symbols::Marker;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GraphMarkerStyle {
    Braille,
    HalfBlock,
    Block,
}

impl GraphMarkerStyle {
    pub fn next(self) -> Self {
        match self {
            Self::Braille => Self::HalfBlock,
            Self::HalfBlock => Self::Block,
            Self::Block => Self::Braille,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Braille => Self::Block,
            Self::HalfBlock => Self::Braille,
            Self::Block => Self::HalfBlock,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Braille => "Braille",
            Self::HalfBlock => "Half Block",
            Self::Block => "Block",
        }
    }

    pub fn to_marker(self) -> Marker {
        match self {
            Self::Braille => Marker::Braille,
            Self::HalfBlock => Marker::HalfBlock,
            Self::Block => Marker::Block,
        }
    }
}

impl Default for GraphMarkerStyle {
    fn default() -> Self {
        Self::Braille
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorTheme {
    TokyoNight,
    Dracula,
    Nord,
    SolarizedDark,
}

impl ColorTheme {
    pub fn next(self) -> Self {
        match self {
            Self::TokyoNight => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::SolarizedDark,
            Self::SolarizedDark => Self::TokyoNight,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::TokyoNight => Self::SolarizedDark,
            Self::Dracula => Self::TokyoNight,
            Self::Nord => Self::Dracula,
            Self::SolarizedDark => Self::Nord,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TokyoNight => "Tokyo Night",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::SolarizedDark => "Solarized Dark",
        }
    }

    pub fn colors(self) -> ThemeColors {
        match self {
            Self::TokyoNight => ThemeColors::TOKYO_NIGHT,
            Self::Dracula => ThemeColors::dracula(),
            Self::Nord => ThemeColors::nord(),
            Self::SolarizedDark => ThemeColors::solarized_dark(),
        }
    }
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self::TokyoNight
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub header_bg: Color,
    pub fg: Color,
    pub border_active: Color,
    pub border_warn: Color,
    pub border_danger: Color,
    pub border_ok: Color,
    pub border_dim: Color,
    pub graph_connections: Color,
    pub graph_cache: Color,
    pub graph_latency: Color,
    pub duration_ok: Color,
    pub duration_warn: Color,
    pub duration_danger: Color,
    pub state_active: Color,
    pub state_idle_txn: Color,
    pub overlay_bg: Color,
    pub highlight_bg: Color,
}

impl ThemeColors {
    pub const TOKYO_NIGHT: Self = Self {
        header_bg: Color::Rgb(36, 40, 59),
        fg: Color::Rgb(192, 202, 245),
        border_active: Color::Cyan,
        border_warn: Color::Yellow,
        border_danger: Color::Red,
        border_ok: Color::Green,
        border_dim: Color::Rgb(68, 71, 90),
        graph_connections: Color::Rgb(97, 175, 239),
        graph_cache: Color::Rgb(86, 182, 194),
        graph_latency: Color::Rgb(152, 195, 121),
        duration_ok: Color::Green,
        duration_warn: Color::Yellow,
        duration_danger: Color::Red,
        state_active: Color::Green,
        state_idle_txn: Color::Yellow,
        overlay_bg: Color::Rgb(26, 27, 38),
        highlight_bg: Color::Rgb(40, 42, 64),
    };

    pub fn dracula() -> Self {
        Self {
            header_bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            border_active: Color::Rgb(139, 233, 253),
            border_warn: Color::Rgb(241, 250, 140),
            border_danger: Color::Rgb(255, 85, 85),
            border_ok: Color::Rgb(80, 250, 123),
            border_dim: Color::Rgb(68, 71, 90),
            graph_connections: Color::Rgb(139, 233, 253),
            graph_cache: Color::Rgb(189, 147, 249),
            graph_latency: Color::Rgb(80, 250, 123),
            duration_ok: Color::Rgb(80, 250, 123),
            duration_warn: Color::Rgb(241, 250, 140),
            duration_danger: Color::Rgb(255, 85, 85),
            state_active: Color::Rgb(80, 250, 123),
            state_idle_txn: Color::Rgb(241, 250, 140),
            overlay_bg: Color::Rgb(33, 34, 44),
            highlight_bg: Color::Rgb(55, 57, 74),
        }
    }

    pub fn nord() -> Self {
        Self {
            header_bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(216, 222, 233),
            border_active: Color::Rgb(136, 192, 208),
            border_warn: Color::Rgb(235, 203, 139),
            border_danger: Color::Rgb(191, 97, 106),
            border_ok: Color::Rgb(163, 190, 140),
            border_dim: Color::Rgb(76, 86, 106),
            graph_connections: Color::Rgb(136, 192, 208),
            graph_cache: Color::Rgb(143, 188, 187),
            graph_latency: Color::Rgb(163, 190, 140),
            duration_ok: Color::Rgb(163, 190, 140),
            duration_warn: Color::Rgb(235, 203, 139),
            duration_danger: Color::Rgb(191, 97, 106),
            state_active: Color::Rgb(163, 190, 140),
            state_idle_txn: Color::Rgb(235, 203, 139),
            overlay_bg: Color::Rgb(38, 44, 57),
            highlight_bg: Color::Rgb(59, 66, 82),
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            header_bg: Color::Rgb(0, 43, 54),
            fg: Color::Rgb(131, 148, 150),
            border_active: Color::Rgb(38, 139, 210),
            border_warn: Color::Rgb(181, 137, 0),
            border_danger: Color::Rgb(220, 50, 47),
            border_ok: Color::Rgb(133, 153, 0),
            border_dim: Color::Rgb(88, 110, 117),
            graph_connections: Color::Rgb(38, 139, 210),
            graph_cache: Color::Rgb(42, 161, 152),
            graph_latency: Color::Rgb(133, 153, 0),
            duration_ok: Color::Rgb(133, 153, 0),
            duration_warn: Color::Rgb(181, 137, 0),
            duration_danger: Color::Rgb(220, 50, 47),
            state_active: Color::Rgb(133, 153, 0),
            state_idle_txn: Color::Rgb(181, 137, 0),
            overlay_bg: Color::Rgb(0, 36, 46),
            highlight_bg: Color::Rgb(7, 54, 66),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub graph_marker: GraphMarkerStyle,
    pub color_theme: ColorTheme,
    pub refresh_interval_secs: u64,
    pub warn_duration_secs: f64,
    pub danger_duration_secs: f64,
    pub recording_retention_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            graph_marker: GraphMarkerStyle::Braille,
            color_theme: ColorTheme::TokyoNight,
            refresh_interval_secs: 2,
            warn_duration_secs: 1.0,
            danger_duration_secs: 10.0,
            recording_retention_secs: 3600,
        }
    }
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("pg_glimpse").join("config.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(contents) = toml::to_string_pretty(self) {
            let _ = fs::write(&path, contents);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigItem {
    GraphMarker,
    ColorTheme,
    RefreshInterval,
    WarnDuration,
    DangerDuration,
    RecordingRetention,
}

impl ConfigItem {
    pub const ALL: [ConfigItem; 6] = [
        ConfigItem::GraphMarker,
        ConfigItem::ColorTheme,
        ConfigItem::RefreshInterval,
        ConfigItem::WarnDuration,
        ConfigItem::DangerDuration,
        ConfigItem::RecordingRetention,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::GraphMarker => "Graph Marker",
            Self::ColorTheme => "Color Theme",
            Self::RefreshInterval => "Refresh Interval",
            Self::WarnDuration => "Warn Duration",
            Self::DangerDuration => "Danger Duration",
            Self::RecordingRetention => "Recording Retention",
        }
    }
}
