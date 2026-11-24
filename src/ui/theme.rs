//! Terminal colors for UI elements (from config).

use crate::config::AppConfig;
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiTheme {
    /// Empty answer slots (dash glyphs).
    pub empty_slot: Color,
    /// Filled answer letters.
    pub filled_slot: Color,
    /// Scrambled pool letters.
    pub pool: Color,
    /// Letter in the right slot (correct-positions clue).
    pub correct_position: Color,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            empty_slot: Color::LightMagenta,
            filled_slot: Color::LightYellow,
            pool: Color::LightCyan,
            correct_position: Color::LightGreen,
        }
    }
}

impl UiTheme {
    pub fn from_config(cfg: &AppConfig) -> Self {
        Self {
            empty_slot: parse_color_name(&cfg.empty_slot_color),
            filled_slot: parse_color_name(&cfg.filled_slot_color),
            pool: parse_color_name(&cfg.pool_color),
            correct_position: parse_color_name(&cfg.correct_position_color),
        }
    }
}

/// Parse a color name from `config.toml` (case-insensitive).
pub fn parse_color_name(name: &str) -> Color {
    match name.trim().to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark_gray" | "darkgrey" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        _ => Color::Magenta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_colors() {
        assert_eq!(parse_color_name("magenta"), Color::Magenta);
        assert_eq!(parse_color_name("BLUE"), Color::Blue);
        assert_eq!(parse_color_name("bogus"), Color::Magenta);
    }
}
