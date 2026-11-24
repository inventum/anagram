//! Config color names → CSS colors (SPEC-ui §6). One mapping table.

use anagram::config::AppConfig;
use ratatui::style::Color;

use crate::dto::ThemeDto;

pub fn theme_dto(config: &AppConfig) -> ThemeDto {
    todo!()
}

pub fn color_to_css(c: Color) -> &'static str {
    todo!()
}
