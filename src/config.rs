use ratatui::style::Color;
use ratatui::symbols::Marker;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphMarkerStyle {
    #[default]
    Braille,
    HalfBlock,
    Block,
}

impl GraphMarkerStyle {
    pub const fn next(self) -> Self {
        match self {
            Self::Braille => Self::HalfBlock,
            Self::HalfBlock => Self::Block,
            Self::Block => Self::Braille,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Braille => Self::Block,
            Self::HalfBlock => Self::Braille,
            Self::Block => Self::HalfBlock,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Braille => "Braille",
            Self::HalfBlock => "Half Block",
            Self::Block => "Block",
        }
    }

    pub const fn to_marker(self) -> Marker {
        match self {
            Self::Braille => Marker::Braille,
            Self::HalfBlock => Marker::HalfBlock,
            Self::Block => Marker::Block,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    pub const fn next(self) -> Self {
        match self {
            Self::TokyoNight => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::SolarizedDark,
            Self::SolarizedDark => Self::SolarizedLight,
            Self::SolarizedLight => Self::CatppuccinLatte,
            Self::CatppuccinLatte => Self::TokyoNight,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::TokyoNight => Self::CatppuccinLatte,
            Self::Dracula => Self::TokyoNight,
            Self::Nord => Self::Dracula,
            Self::SolarizedDark => Self::Nord,
            Self::SolarizedLight => Self::SolarizedDark,
            Self::CatppuccinLatte => Self::SolarizedLight,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::TokyoNight => "Tokyo Night",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::SolarizedDark => "Solarized Dark",
            Self::SolarizedLight => "Solarized Light",
            Self::CatppuccinLatte => "Catppuccin Latte",
        }
    }

    pub const fn colors(self) -> ThemeColors {
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

    pub const fn dracula() -> Self {
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

    pub const fn nord() -> Self {
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

    pub const fn solarized_dark() -> Self {
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

    pub const fn solarized_light() -> Self {
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

    pub const fn catppuccin_latte() -> Self {
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
        fs::read_to_string(&path)
            .map_or_else(|_| Self::default(), |contents| toml::from_str(&contents).unwrap_or_default())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigItem {
    GraphMarker,
    ColorTheme,
    RefreshInterval,
    WarnDuration,
    DangerDuration,
    RecordingRetention,
}

impl ConfigItem {
    pub const ALL: [Self; 6] = [
        Self::GraphMarker,
        Self::ColorTheme,
        Self::RefreshInterval,
        Self::WarnDuration,
        Self::DangerDuration,
        Self::RecordingRetention,
    ];

    pub const fn label(self) -> &'static str {
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
            assert!(!theme.label().is_empty(), "{theme:?} has empty label");
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
        let toml_str = r"
            refresh_interval_secs = 10
        ";
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
            assert!(!item.label().is_empty(), "{item:?} has empty label");
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

    // ─────────────────────────────────────────────────────────────────────────────
    // Config deserialization error handling
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn deserialize_invalid_toml_returns_default() {
        let invalid_toml = "this is not { valid toml at all [[";
        let result: Result<AppConfig, _> = toml::from_str(invalid_toml);
        // When parsing fails, we should use default
        assert!(result.is_err());

        // The load() function handles this gracefully
        let config: AppConfig = toml::from_str(invalid_toml).unwrap_or_default();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn deserialize_wrong_types_returns_default() {
        // String where number expected
        let wrong_type = r#"
            refresh_interval_secs = "not a number"
        "#;
        let result: Result<AppConfig, _> = toml::from_str(wrong_type);
        assert!(result.is_err());

        let config: AppConfig = toml::from_str(wrong_type).unwrap_or_default();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn deserialize_invalid_enum_variant() {
        let invalid_enum = r#"
            graph_marker = "InvalidMarker"
        "#;
        let result: Result<AppConfig, _> = toml::from_str(invalid_enum);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_negative_numbers() {
        // Negative where unsigned expected - TOML will fail
        let negative = r"
            refresh_interval_secs = -5
        ";
        let result: Result<AppConfig, _> = toml::from_str(negative);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_extra_unknown_fields() {
        // Extra fields should be ignored
        let extra_fields = r#"
            refresh_interval_secs = 5
            unknown_field = "should be ignored"
            another_unknown = 123
        "#;
        let config: AppConfig = toml::from_str(extra_fields).unwrap();
        assert_eq!(config.refresh_interval_secs, 5);
        // Other fields should have defaults
        assert_eq!(config.graph_marker, GraphMarkerStyle::Braille);
    }

    #[test]
    fn deserialize_partial_config() {
        // Only some fields specified
        let partial = r#"
            color_theme = "Nord"
            warn_duration_secs = 2.5
        "#;
        let config: AppConfig = toml::from_str(partial).unwrap();
        assert_eq!(config.color_theme, ColorTheme::Nord);
        assert_eq!(config.warn_duration_secs, 2.5);
        // Unspecified fields should have defaults
        assert_eq!(config.refresh_interval_secs, 2);
        assert_eq!(config.graph_marker, GraphMarkerStyle::Braille);
    }

    #[test]
    fn deserialize_large_values() {
        // Large but valid numbers (within i64 range for TOML compatibility)
        let large = r"
            refresh_interval_secs = 9223372036854775807
            warn_duration_secs = 999999999.99
            danger_duration_secs = 999999999.99
            recording_retention_secs = 9223372036854775807
        ";
        let config: AppConfig = toml::from_str(large).unwrap();
        assert_eq!(config.refresh_interval_secs, i64::MAX as u64);
        assert!(config.warn_duration_secs > 999_999_999.0);
    }

    #[test]
    fn deserialize_extreme_values_fails() {
        // u64::MAX is larger than i64::MAX, which TOML can't handle
        let extreme = r"
            refresh_interval_secs = 18446744073709551615
        ";
        let result: Result<AppConfig, _> = toml::from_str(extreme);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_zero_values() {
        let zeros = r"
            refresh_interval_secs = 0
            warn_duration_secs = 0.0
            danger_duration_secs = 0.0
            recording_retention_secs = 0
        ";
        let config: AppConfig = toml::from_str(zeros).unwrap();
        assert_eq!(config.refresh_interval_secs, 0);
        assert_eq!(config.warn_duration_secs, 0.0);
    }

    #[test]
    fn deserialize_float_precision() {
        let floats = r"
            warn_duration_secs = 0.123456789
            danger_duration_secs = 1.987654321
        ";
        let config: AppConfig = toml::from_str(floats).unwrap();
        assert!((config.warn_duration_secs - 0.123_456_789).abs() < 1e-9);
        assert!((config.danger_duration_secs - 1.987_654_321).abs() < 1e-9);
    }

    #[test]
    fn serialize_produces_valid_toml() {
        let config = AppConfig {
            graph_marker: GraphMarkerStyle::HalfBlock,
            color_theme: ColorTheme::Dracula,
            refresh_interval_secs: 5,
            warn_duration_secs: 2.5,
            danger_duration_secs: 15.0,
            recording_retention_secs: 7200,
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Verify it's valid TOML that can be parsed back
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed, config);

        // Verify string contains expected content
        assert!(toml_str.contains("graph_marker"));
        assert!(toml_str.contains("HalfBlock"));
        assert!(toml_str.contains("Dracula"));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // GraphMarkerStyle serialization
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn graph_marker_serialization() {
        for marker in [
            GraphMarkerStyle::Braille,
            GraphMarkerStyle::HalfBlock,
            GraphMarkerStyle::Block,
        ] {
            let json = serde_json::to_string(&marker).unwrap();
            let parsed: GraphMarkerStyle = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, marker);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // ColorTheme serialization
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn color_theme_serialization() {
        for theme in [
            ColorTheme::TokyoNight,
            ColorTheme::Dracula,
            ColorTheme::Nord,
            ColorTheme::SolarizedDark,
            ColorTheme::SolarizedLight,
            ColorTheme::CatppuccinLatte,
        ] {
            let json = serde_json::to_string(&theme).unwrap();
            let parsed: ColorTheme = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, theme);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // AppConfig::load behavior (without filesystem mocking)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn config_path_returns_option() {
        // This just verifies the function doesn't panic
        let path = AppConfig::config_path();
        // On most systems, config_dir should exist
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("pg_glimpse"));
            assert!(p.to_string_lossy().contains("config.toml"));
        }
    }

    #[test]
    fn load_returns_default_when_config_missing() {
        // Since we can't guarantee the config file exists, just verify
        // that load() returns a valid config (either loaded or default)
        let config = AppConfig::load();

        // Should be a valid config with reasonable values
        assert!(config.refresh_interval_secs >= 1);
        assert!(config.danger_duration_secs >= config.warn_duration_secs);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Fuzz tests for TOML parsing robustness
    // ─────────────────────────────────────────────────────────────────────────────

    mod fuzz_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Parsing arbitrary strings should never panic
            #[test]
            fn toml_parse_never_panics(input in ".*") {
                // Should return Ok or Err, never panic
                let _ = toml::from_str::<AppConfig>(&input);
            }

            /// Parsing arbitrary bytes as UTF-8 then TOML should never panic
            #[test]
            fn toml_parse_arbitrary_bytes_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..1000)) {
                if let Ok(input) = String::from_utf8(bytes) {
                    let _ = toml::from_str::<AppConfig>(&input);
                }
            }

            /// unwrap_or_default pattern should always succeed
            #[test]
            fn toml_unwrap_or_default_always_works(input in ".*") {
                let config: AppConfig = toml::from_str(&input).unwrap_or_default();
                // Should always get a valid config
                prop_assert!(config.refresh_interval_secs <= u64::MAX);
            }

            /// Valid TOML with random field values should parse or fail gracefully
            #[test]
            fn toml_with_random_values(
                refresh in 0u64..1_000_000,
                warn in 0.0f64..10000.0,
                danger in 0.0f64..10000.0,
                retention in 0u64..1_000_000
            ) {
                let toml_str = format!(
                    r"
                    refresh_interval_secs = {refresh}
                    warn_duration_secs = {warn}
                    danger_duration_secs = {danger}
                    recording_retention_secs = {retention}
                    "
                );
                let result = toml::from_str::<AppConfig>(&toml_str);
                prop_assert!(result.is_ok());
                let config = result.unwrap();
                prop_assert_eq!(config.refresh_interval_secs, refresh);
            }

            /// Random strings as enum values should fail gracefully
            #[test]
            fn toml_random_enum_values(marker in "[a-zA-Z0-9_]{1,20}", theme in "[a-zA-Z0-9_]{1,20}") {
                let toml_str = format!(
                    r#"
                    graph_marker = "{marker}"
                    color_theme = "{theme}"
                    "#
                );
                // Should either parse (if valid enum) or return error
                let _ = toml::from_str::<AppConfig>(&toml_str);
            }

            /// Deeply nested/malformed TOML should not cause stack overflow
            #[test]
            fn toml_nested_structures(depth in 1usize..50) {
                let open_brackets: String = "[".repeat(depth);
                let close_brackets: String = "]".repeat(depth);
                let input = format!("{open_brackets}value{close_brackets}");
                let _ = toml::from_str::<AppConfig>(&input);
            }

            /// Very long string values should be handled
            #[test]
            fn toml_long_string_values(len in 100usize..10000) {
                let long_value = "x".repeat(len);
                let toml_str = format!(r#"graph_marker = "{long_value}""#);
                let _ = toml::from_str::<AppConfig>(&toml_str);
            }

            /// Unicode in TOML should be handled gracefully
            #[test]
            fn toml_unicode_values(s in "\\PC*") {
                let toml_str = format!(r#"graph_marker = "{}""#, s.replace('\\', "\\\\").replace('"', "\\\""));
                let _ = toml::from_str::<AppConfig>(&toml_str);
            }

            /// Negative numbers where unsigned expected should fail gracefully
            #[test]
            fn toml_negative_unsigned(n in -1_000_000i64..-1) {
                let toml_str = format!("refresh_interval_secs = {n}");
                let result = toml::from_str::<AppConfig>(&toml_str);
                prop_assert!(result.is_err());
            }

            /// NaN/Infinity in floats should be handled
            #[test]
            fn toml_special_floats(special in prop_oneof![
                Just("nan"),
                Just("inf"),
                Just("-inf"),
                Just("NaN"),
                Just("Infinity")
            ]) {
                let toml_str = format!("warn_duration_secs = {special}");
                // TOML doesn't support these, should fail
                let _ = toml::from_str::<AppConfig>(&toml_str);
            }

            /// Roundtrip: valid config -> TOML -> parse should preserve values
            #[test]
            fn toml_roundtrip_preserves_values(
                refresh in 1u64..10000,
                warn in 0.1f64..100.0,
                danger in 0.1f64..100.0,
                retention in 60u64..100_000
            ) {
                let config = AppConfig {
                    graph_marker: GraphMarkerStyle::Braille,
                    color_theme: ColorTheme::TokyoNight,
                    refresh_interval_secs: refresh,
                    warn_duration_secs: warn,
                    danger_duration_secs: danger,
                    recording_retention_secs: retention,
                };

                let toml_str = toml::to_string_pretty(&config).unwrap();
                let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

                prop_assert_eq!(parsed.refresh_interval_secs, refresh);
                prop_assert!((parsed.warn_duration_secs - warn).abs() < 1e-10);
                prop_assert!((parsed.danger_duration_secs - danger).abs() < 1e-10);
                prop_assert_eq!(parsed.recording_retention_secs, retention);
            }
        }
    }
}
