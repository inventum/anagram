//! Config color names → CSS colors (SPEC-ui §6). One mapping table.

use anagram::config::AppConfig;
use anagram::ui::parse_color_name;
use ratatui::style::Color;

use crate::dto::ThemeDto;

pub fn theme_dto(config: &AppConfig) -> ThemeDto {
    let css = |name: &str| color_to_css(parse_color_name(name)).to_string();
    ThemeDto {
        empty_slot: css(&config.empty_slot_color),
        filled_slot: css(&config.filled_slot_color),
        pool: css(&config.pool_color),
        correct_position: css(&config.correct_position_color),
    }
}

/// xterm palette. `Reset`, `Rgb`, and `Indexed` cannot come from a config name
/// (`parse_color_name` yields only the 16 named variants), so they fall back to
/// the unknown-name color (magenta), matching the TUI.
pub fn color_to_css(c: Color) -> &'static str {
    match c {
        Color::Black => "#000000",
        Color::Red => "#cd0000",
        Color::Green => "#00cd00",
        Color::Yellow => "#cdcd00",
        Color::Blue => "#0000ee",
        Color::Magenta => "#cd00cd",
        Color::Cyan => "#00cdcd",
        Color::Gray => "#e5e5e5",
        Color::DarkGray => "#7f7f7f",
        Color::LightRed => "#ff0000",
        Color::LightGreen => "#00ff00",
        Color::LightYellow => "#ffff00",
        Color::LightBlue => "#5c5cff",
        Color::LightMagenta => "#ff00ff",
        Color::LightCyan => "#00ffff",
        Color::White => "#ffffff",
        Color::Reset | Color::Rgb(..) | Color::Indexed(_) => "#cd00cd",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_16_named_variants_map_to_xterm_table() {
        let table: [(Color, &str); 16] = [
            (Color::Black, "#000000"),
            (Color::Red, "#cd0000"),
            (Color::Green, "#00cd00"),
            (Color::Yellow, "#cdcd00"),
            (Color::Blue, "#0000ee"),
            (Color::Magenta, "#cd00cd"),
            (Color::Cyan, "#00cdcd"),
            (Color::Gray, "#e5e5e5"),
            (Color::DarkGray, "#7f7f7f"),
            (Color::LightRed, "#ff0000"),
            (Color::LightGreen, "#00ff00"),
            (Color::LightYellow, "#ffff00"),
            (Color::LightBlue, "#5c5cff"),
            (Color::LightMagenta, "#ff00ff"),
            (Color::LightCyan, "#00ffff"),
            (Color::White, "#ffffff"),
        ];
        for (color, css) in table {
            assert_eq!(color_to_css(color), css, "{color:?}");
        }
    }

    #[test]
    fn non_named_variants_fall_back_to_magenta() {
        assert_eq!(color_to_css(Color::Reset), "#cd00cd");
        assert_eq!(color_to_css(Color::Rgb(1, 2, 3)), "#cd00cd");
        assert_eq!(color_to_css(Color::Indexed(42)), "#cd00cd");
    }

    #[test]
    fn default_config_mapping() {
        let theme = theme_dto(&AppConfig::default());
        assert_eq!(theme.empty_slot, "#ff00ff");
        assert_eq!(theme.filled_slot, "#ffff00");
        assert_eq!(theme.pool, "#00ffff");
        assert_eq!(theme.correct_position, "#00ff00");
    }

    #[test]
    fn unknown_name_falls_back_to_magenta() {
        let config = AppConfig {
            pool_color: "not_a_color".into(),
            ..AppConfig::default()
        };
        assert_eq!(theme_dto(&config).pool, "#cd00cd");
    }

    #[test]
    fn color_names_are_case_insensitive() {
        let lower = AppConfig {
            correct_position_color: "light_green".into(),
            ..AppConfig::default()
        };
        let mixed = AppConfig {
            correct_position_color: "Light_Green".into(),
            ..AppConfig::default()
        };
        let upper = AppConfig {
            correct_position_color: "LIGHTGREEN".into(),
            ..AppConfig::default()
        };
        assert_eq!(theme_dto(&lower).correct_position, "#00ff00");
        assert_eq!(theme_dto(&mixed).correct_position, "#00ff00");
        assert_eq!(theme_dto(&upper).correct_position, "#00ff00");
    }
}
