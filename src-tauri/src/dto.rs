//! IPC data transfer objects. Only these types cross the Tauri boundary (SPEC-ui §5.1–§5.5).
//! Core types (`GameState`, `Round`, `WordEntry`, `LetterWorkbench`, `Action`) derive no serde traits.

use anagram::affix_hints::AffixHints;
use anagram::app::{Action, GameState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    pub phase: PhaseDto,
    pub help_open: bool,
    pub length: u8,
    pub min_size: u8,
    pub max_size: u8,
    pub clues: ClueDto,
    pub active_clue: Option<String>,
    pub status: StatusDto,
    pub round: Option<RoundDto>,
    pub workbench: Option<WorkbenchDto>,
    pub custom: CustomDto,
    pub theme: ThemeDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PhaseDto {
    CustomWordInput,
    CustomWordPick,
    Playing,
    Help,
    Won,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClueDto {
    pub used: usize,
    pub budget: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub kind: StatusKind,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusKind {
    None,
    Error,
    Info,
    WrongAnswer,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundDto {
    pub length: u8,
    pub pool: Vec<char>,
    pub answer: Vec<Option<char>>,
    pub cursor: usize,
    /// `Some` only while `highlight_correct_positions` is on.
    pub correct: Option<Vec<bool>>,
    /// `Some` only when phase is `won`.
    pub solution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchDto {
    pub layout: LayoutDto,
    pub rows: u8,
    pub cols: u8,
    pub cells: Vec<Option<char>>,
    pub placed: Vec<PlacedDto>,
    pub pool: Vec<char>,
    pub cursor: usize,
    pub held: Option<char>,
    pub focus: FocusDto,
    pub affixes: AffixDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutDto {
    Grid,
    Free,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedDto {
    pub ch: char,
    pub x: u8,
    pub y: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FocusDto {
    Answer,
    Grid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffixDto {
    pub starts: Vec<String>,
    pub ends: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomDto {
    pub input: String,
    pub choices: Vec<ChoiceDto>,
    pub pick: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceDto {
    pub word: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeDto {
    pub empty_slot: String,
    pub filled_slot: String,
    pub pool: String,
    pub correct_position: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ActionDto {
    ToggleHelp,
    NewPuzzle,
    ResetRound,
    RequestClue,
    Backspace,
    CursorLeft,
    CursorRight,
    TypeLetter { ch: char },
    SelectSize { n: u8 },
    OpenCustomWord,
    CustomWordType { ch: char },
    CustomWordBackspace,
    SubmitCustomWord,
    CancelCustomWord,
    CustomPickDelta { delta: i32 },
    CustomPickSelect { idx: usize },
    ConfirmCustomPick,
    WorkbenchClickCell { idx: usize },
    WorkbenchClickPool { idx: usize },
    WorkbenchSetFocus { focus: FocusDto },
    WorkbenchClear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtoError {
    InvalidLetter(char),
}

/// Build the frontend snapshot. Answer data is included only per SPEC-ui §5.5.
pub fn ui_state(state: &GameState, affix: Option<&AffixHints>, theme: &ThemeDto) -> UiState {
    todo!()
}

impl ActionDto {
    /// Convert to core actions (1:1, except `cancelCustomWord` → two actions, D-9).
    pub fn into_actions(self) -> Result<Vec<Action>, DtoError> {
        todo!()
    }
}
