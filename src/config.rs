use ratatui::style::Color;
use ratatui::symbols::Marker;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum GraphMarkerStyle {
    #[default]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum ColorTheme {
    #[default]
    TokyoNight,
    Dracula,
    Nord,
    SolarizedDark,
    SolarizedLight,
    CatppuccinLatte,
}

impl ColorTheme {
    pub fn next(self) -> Self {
        match self {
            Self::TokyoNight => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::SolarizedDark,
            Self::SolarizedDark => Self::SolarizedLight,
            Self::SolarizedLight => Self::CatppuccinLatte,
            Self::CatppuccinLatte => Self::TokyoNight,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::TokyoNight => Self::CatppuccinLatte,
            Self::Dracula => Self::TokyoNight,
            Self::Nord => Self::Dracula,
            Self::SolarizedDark => Self::Nord,
            Self::SolarizedLight => Self::SolarizedDark,
            Self::CatppuccinLatte => Self::SolarizedLight,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TokyoNight => "Tokyo Night",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::SolarizedDark => "Solarized Dark",
            Self::SolarizedLight => "Solarized Light",
            Self::CatppuccinLatte => "Catppuccin Latte",
        }
    }

    pub fn colors(self) -> ThemeColors {
        match self {
            Self::TokyoNight => ThemeColors::TOKYO_NIGHT,
            Self::Dracula => ThemeColors::dracula(),
            Self::Nord => ThemeColors::nord(),
            Self::SolarizedDark => ThemeColors::solarized_dark(),
            Self::SolarizedLight => ThemeColors::solarized_light(),
            Self::CatppuccinLatte => ThemeColors::catppuccin_latte(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub header_bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
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
    // SQL syntax highlighting
    pub sql_keyword: Color,
    pub sql_string: Color,
    pub sql_number: Color,
    pub sql_comment: Color,
}

impl ThemeColors {
    pub const TOKYO_NIGHT: Self = Self {
        header_bg: Color::Rgb(36, 40, 59),
        fg: Color::Rgb(192, 202, 245),
        fg_dim: Color::Rgb(115, 121, 148),
        border_active: Color::Rgb(125, 207, 255),   // soft cyan
        border_warn: Color::Rgb(224, 175, 104),     // soft amber
        border_danger: Color::Rgb(247, 118, 142),   // soft red
        border_ok: Color::Rgb(158, 206, 106),       // soft green
        border_dim: Color::Rgb(59, 66, 97),         // muted blue-gray
        graph_connections: Color::Rgb(97, 175, 239),
        graph_cache: Color::Rgb(86, 182, 194),
        graph_latency: Color::Rgb(152, 195, 121),
        duration_ok: Color::Rgb(158, 206, 106),     // soft green
        duration_warn: Color::Rgb(224, 175, 104),   // soft amber
        duration_danger: Color::Rgb(247, 118, 142), // soft red
        state_active: Color::Rgb(158, 206, 106),    // soft green
        state_idle_txn: Color::Rgb(224, 175, 104),  // soft amber
        overlay_bg: Color::Rgb(26, 27, 38),
        highlight_bg: Color::Rgb(40, 42, 64),
        sql_keyword: Color::Rgb(198, 120, 221),     // purple
        sql_string: Color::Rgb(152, 195, 121),      // green
        sql_number: Color::Rgb(209, 154, 102),      // orange
        sql_comment: Color::Rgb(92, 99, 112),       // gray
    };

    pub fn dracula() -> Self {
        Self {
            header_bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            fg_dim: Color::Rgb(98, 114, 164),
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
            sql_keyword: Color::Rgb(255, 121, 198),  // pink
            sql_string: Color::Rgb(241, 250, 140),   // yellow
            sql_number: Color::Rgb(189, 147, 249),   // purple
            sql_comment: Color::Rgb(98, 114, 164),   // comment gray
        }
    }

    pub fn nord() -> Self {
        Self {
            header_bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(216, 222, 233),
            fg_dim: Color::Rgb(107, 121, 142),
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
            sql_keyword: Color::Rgb(180, 142, 173),  // purple (nord15)
            sql_string: Color::Rgb(163, 190, 140),   // green (nord14)
            sql_number: Color::Rgb(208, 135, 112),   // orange (nord12)
            sql_comment: Color::Rgb(76, 86, 106),    // gray (nord3)
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            header_bg: Color::Rgb(0, 43, 54),
            fg: Color::Rgb(131, 148, 150),
            fg_dim: Color::Rgb(88, 110, 117),
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
            sql_keyword: Color::Rgb(108, 113, 196),  // violet
            sql_string: Color::Rgb(42, 161, 152),    // cyan
            sql_number: Color::Rgb(203, 75, 22),     // orange
            sql_comment: Color::Rgb(88, 110, 117),   // base01
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            header_bg: Color::Rgb(238, 232, 213),    // base2
            fg: Color::Rgb(101, 123, 131),           // base00
            fg_dim: Color::Rgb(147, 161, 161),       // base1
            border_active: Color::Rgb(38, 139, 210), // blue
            border_warn: Color::Rgb(181, 137, 0),    // yellow
            border_danger: Color::Rgb(220, 50, 47),  // red
            border_ok: Color::Rgb(133, 153, 0),      // green
            border_dim: Color::Rgb(147, 161, 161),   // base1
            graph_connections: Color::Rgb(38, 139, 210),
            graph_cache: Color::Rgb(42, 161, 152),   // cyan
            graph_latency: Color::Rgb(133, 153, 0),
            duration_ok: Color::Rgb(133, 153, 0),
            duration_warn: Color::Rgb(181, 137, 0),
            duration_danger: Color::Rgb(220, 50, 47),
            state_active: Color::Rgb(133, 153, 0),
            state_idle_txn: Color::Rgb(181, 137, 0),
            overlay_bg: Color::Rgb(253, 246, 227),   // base3
            highlight_bg: Color::Rgb(238, 232, 213), // base2
            sql_keyword: Color::Rgb(108, 113, 196),  // violet
            sql_string: Color::Rgb(42, 161, 152),    // cyan
            sql_number: Color::Rgb(203, 75, 22),     // orange
            sql_comment: Color::Rgb(147, 161, 161),  // base1
        }
    }

    pub fn catppuccin_latte() -> Self {
        Self {
            header_bg: Color::Rgb(230, 233, 239),    // mantle
            fg: Color::Rgb(76, 79, 105),             // text
            fg_dim: Color::Rgb(140, 143, 161),       // overlay0
            border_active: Color::Rgb(30, 102, 245), // blue
            border_warn: Color::Rgb(223, 142, 29),   // yellow
            border_danger: Color::Rgb(210, 15, 57),  // red
            border_ok: Color::Rgb(64, 160, 43),      // green
            border_dim: Color::Rgb(140, 143, 161),   // overlay0
            graph_connections: Color::Rgb(30, 102, 245),
            graph_cache: Color::Rgb(23, 146, 153),   // teal
            graph_latency: Color::Rgb(64, 160, 43),
            duration_ok: Color::Rgb(64, 160, 43),
            duration_warn: Color::Rgb(223, 142, 29),
            duration_danger: Color::Rgb(210, 15, 57),
            state_active: Color::Rgb(64, 160, 43),
            state_idle_txn: Color::Rgb(223, 142, 29),
            overlay_bg: Color::Rgb(239, 241, 245),   // base
            highlight_bg: Color::Rgb(220, 224, 232), // surface0
            sql_keyword: Color::Rgb(136, 57, 239),   // mauve
            sql_string: Color::Rgb(64, 160, 43),     // green
            sql_number: Color::Rgb(254, 100, 11),    // peach
            sql_comment: Color::Rgb(140, 143, 161),  // overlay0
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────────
    // GraphMarkerStyle tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn graph_marker_next_cycles() {
        assert_eq!(GraphMarkerStyle::Braille.next(), GraphMarkerStyle::HalfBlock);
        assert_eq!(GraphMarkerStyle::HalfBlock.next(), GraphMarkerStyle::Block);
        assert_eq!(GraphMarkerStyle::Block.next(), GraphMarkerStyle::Braille);
    }

    #[test]
    fn graph_marker_prev_cycles() {
        assert_eq!(GraphMarkerStyle::Braille.prev(), GraphMarkerStyle::Block);
        assert_eq!(GraphMarkerStyle::HalfBlock.prev(), GraphMarkerStyle::Braille);
        assert_eq!(GraphMarkerStyle::Block.prev(), GraphMarkerStyle::HalfBlock);
    }

    #[test]
    fn graph_marker_next_prev_inverse() {
        for style in [
            GraphMarkerStyle::Braille,
            GraphMarkerStyle::HalfBlock,
            GraphMarkerStyle::Block,
        ] {
            assert_eq!(style.next().prev(), style);
            assert_eq!(style.prev().next(), style);
        }
    }

    #[test]
    fn graph_marker_labels_not_empty() {
        assert!(!GraphMarkerStyle::Braille.label().is_empty());
        assert!(!GraphMarkerStyle::HalfBlock.label().is_empty());
        assert!(!GraphMarkerStyle::Block.label().is_empty());
    }

    #[test]
    fn graph_marker_to_marker() {
        assert!(matches!(
            GraphMarkerStyle::Braille.to_marker(),
            Marker::Braille
        ));
        assert!(matches!(
            GraphMarkerStyle::HalfBlock.to_marker(),
            Marker::HalfBlock
        ));
        assert!(matches!(GraphMarkerStyle::Block.to_marker(), Marker::Block));
    }

    #[test]
    fn graph_marker_default() {
        assert_eq!(GraphMarkerStyle::default(), GraphMarkerStyle::Braille);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // ColorTheme tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn color_theme_next_cycles() {
        assert_eq!(ColorTheme::TokyoNight.next(), ColorTheme::Dracula);
        assert_eq!(ColorTheme::Dracula.next(), ColorTheme::Nord);
        assert_eq!(ColorTheme::Nord.next(), ColorTheme::SolarizedDark);
        assert_eq!(ColorTheme::SolarizedDark.next(), ColorTheme::SolarizedLight);
        assert_eq!(ColorTheme::SolarizedLight.next(), ColorTheme::CatppuccinLatte);
        assert_eq!(ColorTheme::CatppuccinLatte.next(), ColorTheme::TokyoNight);
    }

    #[test]
    fn color_theme_prev_cycles() {
        assert_eq!(ColorTheme::TokyoNight.prev(), ColorTheme::CatppuccinLatte);
        assert_eq!(ColorTheme::CatppuccinLatte.prev(), ColorTheme::SolarizedLight);
        assert_eq!(ColorTheme::SolarizedLight.prev(), ColorTheme::SolarizedDark);
        assert_eq!(ColorTheme::SolarizedDark.prev(), ColorTheme::Nord);
        assert_eq!(ColorTheme::Nord.prev(), ColorTheme::Dracula);
        assert_eq!(ColorTheme::Dracula.prev(), ColorTheme::TokyoNight);
    }

    #[test]
    fn color_theme_next_prev_inverse() {
        for theme in [
            ColorTheme::TokyoNight,
            ColorTheme::Dracula,
            ColorTheme::Nord,
            ColorTheme::SolarizedDark,
            ColorTheme::SolarizedLight,
            ColorTheme::CatppuccinLatte,
        ] {
            assert_eq!(theme.next().prev(), theme);
            assert_eq!(theme.prev().next(), theme);
        }
    }

    #[test]
    fn color_theme_labels_not_empty() {
        for theme in [
            ColorTheme::TokyoNight,
            ColorTheme::Dracula,
            ColorTheme::Nord,
            ColorTheme::SolarizedDark,
            ColorTheme::SolarizedLight,
            ColorTheme::CatppuccinLatte,
        ] {
            assert!(!theme.label().is_empty(), "{:?} has empty label", theme);
        }
    }

    #[test]
    fn color_theme_colors_returns_valid_theme() {
        // Just verify colors() doesn't panic and returns something
        for theme in [
            ColorTheme::TokyoNight,
            ColorTheme::Dracula,
            ColorTheme::Nord,
            ColorTheme::SolarizedDark,
            ColorTheme::SolarizedLight,
            ColorTheme::CatppuccinLatte,
        ] {
            let colors = theme.colors();
            // Verify some colors are set (not default/black)
            assert!(!matches!(colors.fg, Color::Black));
        }
    }

    #[test]
    fn color_theme_default() {
        assert_eq!(ColorTheme::default(), ColorTheme::TokyoNight);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // AppConfig tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn app_config_default_values() {
        let config = AppConfig::default();
        assert_eq!(config.graph_marker, GraphMarkerStyle::Braille);
        assert_eq!(config.color_theme, ColorTheme::TokyoNight);
        assert_eq!(config.refresh_interval_secs, 2);
        assert_eq!(config.warn_duration_secs, 1.0);
        assert_eq!(config.danger_duration_secs, 10.0);
        assert_eq!(config.recording_retention_secs, 3600);
    }

    #[test]
    fn app_config_serialization_roundtrip() {
        let config = AppConfig {
            graph_marker: GraphMarkerStyle::Block,
            color_theme: ColorTheme::Nord,
            refresh_interval_secs: 5,
            warn_duration_secs: 2.5,
            danger_duration_secs: 15.0,
            recording_retention_secs: 7200,
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.graph_marker, config.graph_marker);
        assert_eq!(parsed.color_theme, config.color_theme);
        assert_eq!(parsed.refresh_interval_secs, config.refresh_interval_secs);
        assert_eq!(parsed.warn_duration_secs, config.warn_duration_secs);
        assert_eq!(parsed.danger_duration_secs, config.danger_duration_secs);
        assert_eq!(
            parsed.recording_retention_secs,
            config.recording_retention_secs
        );
    }

    #[test]
    fn app_config_deserialize_with_missing_fields() {
        // Test that serde(default) works - missing fields get defaults
        let toml_str = r#"
            refresh_interval_secs = 10
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.refresh_interval_secs, 10);
        // Other fields should have defaults
        assert_eq!(config.graph_marker, GraphMarkerStyle::Braille);
        assert_eq!(config.color_theme, ColorTheme::TokyoNight);
    }

    #[test]
    fn app_config_deserialize_empty_string() {
        let config: AppConfig = toml::from_str("").unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn app_config_json_roundtrip() {
        let config = AppConfig {
            graph_marker: GraphMarkerStyle::HalfBlock,
            color_theme: ColorTheme::Dracula,
            refresh_interval_secs: 3,
            warn_duration_secs: 0.5,
            danger_duration_secs: 5.0,
            recording_retention_secs: 1800,
        };

        let json_str = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed.graph_marker, config.graph_marker);
        assert_eq!(parsed.color_theme, config.color_theme);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // ConfigItem tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn config_item_all_contains_all_variants() {
        // Ensure ALL array has correct count
        assert_eq!(ConfigItem::ALL.len(), 6);

        // Ensure all variants are present
        assert!(ConfigItem::ALL.contains(&ConfigItem::GraphMarker));
        assert!(ConfigItem::ALL.contains(&ConfigItem::ColorTheme));
        assert!(ConfigItem::ALL.contains(&ConfigItem::RefreshInterval));
        assert!(ConfigItem::ALL.contains(&ConfigItem::WarnDuration));
        assert!(ConfigItem::ALL.contains(&ConfigItem::DangerDuration));
        assert!(ConfigItem::ALL.contains(&ConfigItem::RecordingRetention));
    }

    #[test]
    fn config_item_labels_not_empty() {
        for item in ConfigItem::ALL {
            assert!(!item.label().is_empty(), "{:?} has empty label", item);
        }
    }

    #[test]
    fn config_item_labels_unique() {
        let labels: Vec<_> = ConfigItem::ALL.iter().map(|i| i.label()).collect();
        let mut unique = labels.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(labels.len(), unique.len(), "ConfigItem labels should be unique");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // ThemeColors tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn theme_colors_tokyo_night_is_const() {
        // Verify the const is accessible and has expected structure
        let colors = ThemeColors::TOKYO_NIGHT;
        assert!(matches!(colors.header_bg, Color::Rgb(36, 40, 59)));
    }

    #[test]
    fn theme_colors_all_themes_have_distinct_header_bg() {
        let themes = [
            ColorTheme::TokyoNight.colors().header_bg,
            ColorTheme::Dracula.colors().header_bg,
            ColorTheme::Nord.colors().header_bg,
            ColorTheme::SolarizedDark.colors().header_bg,
            ColorTheme::SolarizedLight.colors().header_bg,
            ColorTheme::CatppuccinLatte.colors().header_bg,
        ];

        // Check all are distinct (simple pairwise comparison)
        for i in 0..themes.len() {
            for j in (i + 1)..themes.len() {
                assert_ne!(
                    format!("{:?}", themes[i]),
                    format!("{:?}", themes[j]),
                    "Themes {} and {} have same header_bg",
                    i,
                    j
                );
            }
        }
    }
}
