//! IPC data transfer objects. Only these types cross the Tauri boundary (SPEC-ui §5.1–§5.5).
//! Core types (`GameState`, `Round`, `WordEntry`, `LetterWorkbench`, `Action`) derive no serde traits.

use anagram::affix_hints::AffixHints;
use anagram::app::{Action, GameState, InputFocus, Phase, Status};
use anagram::dictionary::Dictionary;
use anagram::game::Round;
use anagram::workbench::LetterWorkbench;
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
    /// Key absent (not `null`) outside play phases (§5.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<RoundDto>,
    /// Key absent (not `null`) outside play phases (§5.5).
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Key present only while `highlight_correct_positions` is on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correct: Option<Vec<bool>>,
    /// Key present only when phase is `won`.
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Status text for `Status::WrongAnswer` (same wording as `src/ui.rs`).
const WRONG_ANSWER_TEXT: &str = "Not quite — check your letters.";

/// Build the frontend snapshot. Answer data is included only per SPEC-ui §5.5.
pub fn ui_state(state: &GameState, affix: Option<&AffixHints>, theme: &ThemeDto) -> UiState {
    let phase = phase_dto(state.phase);
    // Round, workbench, and clue data are sent only in the play phases (§5.5, R-3).
    let revealed = matches!(state.phase, Phase::Playing | Phase::Help | Phase::Won);
    let custom_phase = matches!(state.phase, Phase::CustomWordInput | Phase::CustomWordPick);

    let round = if revealed {
        state
            .round
            .as_ref()
            .map(|r| round_dto(r, state.highlight_correct_positions, state.phase == Phase::Won))
    } else {
        None
    };

    let clues = if revealed {
        ClueDto {
            used: state.clues_used,
            budget: state.current_word.as_ref().map_or(0, |w| {
                Dictionary::clue_budget_round(w, &state.anagram_alternates)
            }),
        }
    } else {
        ClueDto { used: 0, budget: 0 }
    };

    let custom = CustomDto {
        input: if custom_phase {
            state.custom_word_input.clone()
        } else {
            String::new()
        },
        choices: if matches!(state.phase, Phase::CustomWordPick) {
            state
                .custom_word_choices
                .iter()
                .map(|e| ChoiceDto {
                    word: e.word.clone(),
                    definition: e.definition.clone().unwrap_or_default(),
                })
                .collect()
        } else {
            Vec::new()
        },
        pick: if matches!(state.phase, Phase::CustomWordPick) {
            state.custom_word_pick
        } else {
            0
        },
    };

    UiState {
        phase,
        help_open: state.help_open,
        length: state.size,
        min_size: state.min_word_len,
        max_size: state.max_word_len,
        clues,
        active_clue: if revealed {
            state.active_clue_text.clone()
        } else {
            None
        },
        status: status_dto(&state.status),
        round,
        workbench: if revealed {
            state
                .workbench
                .as_ref()
                .map(|wb| workbench_dto(wb, state.input_focus, affix))
        } else {
            None
        },
        custom,
        theme: theme.clone(),
    }
}

fn phase_dto(phase: Phase) -> PhaseDto {
    match phase {
        Phase::CustomWordInput => PhaseDto::CustomWordInput,
        Phase::CustomWordPick => PhaseDto::CustomWordPick,
        Phase::Playing => PhaseDto::Playing,
        Phase::Help => PhaseDto::Help,
        Phase::Won => PhaseDto::Won,
        // Never produced in the GUI; `Session` rejects these before snapshotting (WP-3).
        // Mapped to `playing` with all play-only fields hidden, so nothing leaks.
        Phase::Menu | Phase::SizeSelect => PhaseDto::Playing,
    }
}

fn round_dto(round: &Round, highlight: bool, won: bool) -> RoundDto {
    let n = round.answer.len();
    RoundDto {
        length: n as u8,
        pool: round.pool.clone(),
        answer: round.answer.clone(),
        cursor: round.cursor,
        correct: highlight.then(|| (0..n).map(|i| round.slot_is_correct(i)).collect()),
        solution: won.then(|| round.solution.clone()),
    }
}

fn status_dto(status: &Status) -> StatusDto {
    match status {
        Status::None => StatusDto {
            kind: StatusKind::None,
            message: None,
        },
        Status::Error(m) => StatusDto {
            kind: StatusKind::Error,
            message: Some(m.clone()),
        },
        Status::Info(m) => StatusDto {
            kind: StatusKind::Info,
            message: Some(m.clone()),
        },
        Status::WrongAnswer => StatusDto {
            kind: StatusKind::WrongAnswer,
            message: Some(WRONG_ANSWER_TEXT.into()),
        },
    }
}

fn workbench_dto(
    wb: &LetterWorkbench,
    focus: InputFocus,
    affix: Option<&AffixHints>,
) -> WorkbenchDto {
    let affixes = match affix {
        Some(a) => AffixDto {
            starts: a.starts.clone(),
            ends: a.ends.clone(),
        },
        None => AffixDto {
            starts: Vec::new(),
            ends: Vec::new(),
        },
    };
    WorkbenchDto {
        layout: LayoutDto::Grid,
        rows: wb.rows,
        cols: wb.cols,
        cells: wb.cells.clone(),
        placed: Vec::new(),
        pool: wb.pool.clone(),
        cursor: wb.cursor,
        held: wb.held,
        focus: match focus {
            InputFocus::Answer => FocusDto::Answer,
            InputFocus::Grid => FocusDto::Grid,
        },
        affixes,
    }
}

fn letter(ch: char) -> Result<char, DtoError> {
    if ch.is_ascii_alphabetic() {
        Ok(ch)
    } else {
        Err(DtoError::InvalidLetter(ch))
    }
}

impl ActionDto {
    /// Convert to core actions (1:1, except `cancelCustomWord` → two actions, D-9).
    pub fn into_actions(self) -> Result<Vec<Action>, DtoError> {
        Ok(match self {
            ActionDto::ToggleHelp => vec![Action::ToggleHelp],
            ActionDto::NewPuzzle => vec![Action::NewPuzzle],
            ActionDto::ResetRound => vec![Action::ResetRound],
            ActionDto::RequestClue => vec![Action::RequestClue],
            ActionDto::Backspace => vec![Action::Backspace],
            ActionDto::CursorLeft => vec![Action::CursorLeft],
            ActionDto::CursorRight => vec![Action::CursorRight],
            ActionDto::TypeLetter { ch } => vec![Action::TypeLetter(letter(ch)?)],
            ActionDto::SelectSize { n } => vec![Action::SelectSize(n)],
            ActionDto::OpenCustomWord => vec![Action::OpenCustomWord],
            ActionDto::CustomWordType { ch } => vec![Action::CustomWordType(letter(ch)?)],
            ActionDto::CustomWordBackspace => vec![Action::CustomWordBackspace],
            ActionDto::SubmitCustomWord => vec![Action::SubmitCustomWord],
            ActionDto::CancelCustomWord => vec![Action::CancelCustomWord, Action::NewPuzzle],
            ActionDto::CustomPickDelta { delta } => vec![Action::CustomPickDelta(delta)],
            ActionDto::CustomPickSelect { idx } => vec![Action::CustomPickSelect(idx)],
            ActionDto::ConfirmCustomPick => vec![Action::ConfirmCustomPick],
            ActionDto::WorkbenchClickCell { idx } => vec![Action::WorkbenchClickCell(idx)],
            ActionDto::WorkbenchClickPool { idx } => vec![Action::WorkbenchClickPool(idx)],
            ActionDto::WorkbenchSetFocus { focus } => vec![Action::SetInputFocus(match focus {
                FocusDto::Answer => InputFocus::Answer,
                FocusDto::Grid => InputFocus::Grid,
            })],
            ActionDto::WorkbenchClear => vec![Action::WorkbenchResetGrid],
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;
    use anagram::app::start_custom_word;
    use anagram::signature_index::SignatureIndex;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use serde_json::{json, Value};

    fn data() -> &'static (Dictionary, SignatureIndex) {
        static DATA: OnceLock<(Dictionary, SignatureIndex)> = OnceLock::new();
        DATA.get_or_init(|| {
            (
                Dictionary::from_embedded().expect("embedded dictionary"),
                SignatureIndex::from_embedded().expect("embedded index"),
            )
        })
    }

    fn theme() -> ThemeDto {
        ThemeDto {
            empty_slot: "#ff00ff".into(),
            filled_slot: "#ffff00".into(),
            pool: "#00ffff".into(),
            correct_position: "#00ff00".into(),
        }
    }

    fn hints() -> AffixHints {
        AffixHints {
            starts: vec!["cr".into()],
            ends: vec!["-ate".into()],
        }
    }

    fn playing(word: &str) -> GameState {
        let (dict, index) = data();
        let mut rng = StdRng::seed_from_u64(7);
        let state = start_custom_word(GameState::new(5), word, dict, index, &mut rng);
        assert_eq!(state.phase, Phase::Playing, "start {word}");
        state
    }

    fn snapshot(state: &GameState, affix: Option<&AffixHints>) -> Value {
        serde_json::to_value(ui_state(state, affix, &theme())).expect("serialize")
    }

    fn all_strings(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| all_strings(x, out)),
            Value::Object(m) => m.values().for_each(|x| all_strings(x, out)),
            _ => {}
        }
    }

    fn strings_of(v: &Value) -> Vec<String> {
        let mut out = Vec::new();
        all_strings(v, &mut out);
        out
    }

    fn acts(a: ActionDto) -> String {
        format!("{:?}", a.into_actions().expect("mapped"))
    }

    #[test]
    fn playing_hides_solution() {
        let state = playing("crate");
        let solution = state.round.as_ref().expect("round").solution.clone();
        let v = snapshot(&state, None);
        assert!(!v.to_string().contains("\"solution\""));
        assert!(strings_of(&v)
            .iter()
            .all(|s| !s.eq_ignore_ascii_case(&solution)));
        assert_eq!(v["phase"], "playing");
        assert!(v["activeClue"].is_null());
        assert_eq!(v["minSize"], 4);
        assert_eq!(v["maxSize"], 10);
        assert_eq!(v["helpOpen"], false);
    }

    #[test]
    fn won_exposes_solution() {
        let mut state = playing("crate");
        state.phase = Phase::Won;
        let solution = state.round.as_ref().expect("round").solution.clone();
        let v = snapshot(&state, None);
        assert_eq!(v["phase"], "won");
        assert_eq!(v["round"]["solution"], solution.as_str());
    }

    #[test]
    fn custom_phases_hide_round() {
        let mut state = playing("crate");
        state.workbench = Some(LetterWorkbench::from_solution(
            "CRATE",
            5,
            5,
            &mut StdRng::seed_from_u64(3),
        ));
        state.custom_word_input = "crat".into();
        let h = hints();
        for (phase, name) in [
            (Phase::CustomWordInput, "customWordInput"),
            (Phase::CustomWordPick, "customWordPick"),
        ] {
            state.phase = phase;
            let v = snapshot(&state, Some(&h));
            assert_eq!(v["phase"], name);
            assert!(v["round"].is_null());
            assert!(v["workbench"].is_null());
            assert!(v["activeClue"].is_null());
            assert_eq!(v["clues"], json!({"used": 0, "budget": 0}));
        }
    }

    #[test]
    fn workbench_carries_affixes_only_when_given() {
        let mut state = playing("crate");
        state.workbench = Some(LetterWorkbench::from_solution(
            "CRATE",
            5,
            5,
            &mut StdRng::seed_from_u64(3),
        ));
        let h = hints();
        let v = snapshot(&state, Some(&h));
        assert_eq!(v["workbench"]["layout"], "grid");
        assert_eq!(v["workbench"]["focus"], "answer");
        assert_eq!(v["workbench"]["affixes"]["starts"], json!(["cr"]));
        let v = snapshot(&state, None);
        assert_eq!(v["workbench"]["affixes"]["starts"], json!([]));
    }

    #[test]
    fn pick_exposes_choices_only() {
        let (dict, _) = data();
        let mut state = playing("crate");
        state.custom_word_choices = vec![dict.resolve_entry("crate"), dict.resolve_entry("trace")];
        state.custom_word_pick = 1;
        state.phase = Phase::CustomWordPick;
        let v = snapshot(&state, None);
        let choices = v["custom"]["choices"].as_array().expect("choices");
        assert_eq!(choices.len(), 2);
        assert_eq!(v["custom"]["pick"], 1);
        for c in choices {
            assert_eq!(c.as_object().expect("object").len(), 2);
            assert!(c.get("word").is_some() && c.get("definition").is_some());
        }
        assert!(v["round"].is_null());
        assert!(!v.to_string().contains("nounSenses"));
        assert!(!v.to_string().contains("noun_senses"));

        state.phase = Phase::Playing;
        let v = snapshot(&state, None);
        assert_eq!(v["custom"]["choices"].as_array().expect("choices").len(), 0);
    }

    #[test]
    fn correct_flags_follow_highlight() {
        let mut state = playing("crate");
        assert!(snapshot(&state, None)["round"]["correct"].is_null());

        state.highlight_correct_positions = true;
        let first = state
            .round
            .as_ref()
            .expect("round")
            .solution
            .chars()
            .next()
            .expect("letter");
        assert!(state.round.as_mut().expect("round").try_type_letter(first));

        let round = state.round.as_ref().expect("round");
        let expected: Vec<bool> = (0..round.answer.len())
            .map(|i| round.slot_is_correct(i))
            .collect();
        let v = snapshot(&state, None);
        let got: Vec<bool> = v["round"]["correct"]
            .as_array()
            .expect("correct flags")
            .iter()
            .map(|b| b.as_bool().expect("bool"))
            .collect();
        assert_eq!(got, expected);
        assert!(got[0]);
        assert!(!got[1]);
    }

    #[test]
    fn alternates_absent_outside_choices() {
        let state = playing("aliment");
        assert!(state
            .anagram_alternates
            .iter()
            .any(|e| e.word.eq_ignore_ascii_case("ailment")));
        let v = snapshot(&state, None);
        assert!(strings_of(&v)
            .iter()
            .all(|s| !s.eq_ignore_ascii_case("ailment")));
    }

    #[test]
    fn status_maps_wrong_answer() {
        let mut state = playing("crate");
        state.status = Status::WrongAnswer;
        let v = snapshot(&state, None);
        assert_eq!(v["status"]["kind"], "wrongAnswer");
        assert_eq!(v["status"]["message"], "Not quite — check your letters.");
    }

    #[test]
    fn actions_map_one_to_one() {
        assert_eq!(acts(ActionDto::ToggleHelp), "[ToggleHelp]");
        assert_eq!(acts(ActionDto::NewPuzzle), "[NewPuzzle]");
        assert_eq!(acts(ActionDto::ResetRound), "[ResetRound]");
        assert_eq!(acts(ActionDto::RequestClue), "[RequestClue]");
        assert_eq!(acts(ActionDto::Backspace), "[Backspace]");
        assert_eq!(acts(ActionDto::CursorLeft), "[CursorLeft]");
        assert_eq!(acts(ActionDto::CursorRight), "[CursorRight]");
        assert_eq!(acts(ActionDto::TypeLetter { ch: 'a' }), "[TypeLetter('a')]");
        assert_eq!(acts(ActionDto::SelectSize { n: 6 }), "[SelectSize(6)]");
        assert_eq!(acts(ActionDto::OpenCustomWord), "[OpenCustomWord]");
        assert_eq!(
            acts(ActionDto::CustomWordType { ch: 'b' }),
            "[CustomWordType('b')]"
        );
        assert_eq!(acts(ActionDto::CustomWordBackspace), "[CustomWordBackspace]");
        assert_eq!(acts(ActionDto::SubmitCustomWord), "[SubmitCustomWord]");
        assert_eq!(
            acts(ActionDto::CancelCustomWord),
            "[CancelCustomWord, NewPuzzle]"
        );
        assert_eq!(
            acts(ActionDto::CustomPickDelta { delta: -1 }),
            "[CustomPickDelta(-1)]"
        );
        assert_eq!(
            acts(ActionDto::CustomPickSelect { idx: 2 }),
            "[CustomPickSelect(2)]"
        );
        assert_eq!(acts(ActionDto::ConfirmCustomPick), "[ConfirmCustomPick]");
        assert_eq!(
            acts(ActionDto::WorkbenchClickCell { idx: 3 }),
            "[WorkbenchClickCell(3)]"
        );
        assert_eq!(
            acts(ActionDto::WorkbenchClickPool { idx: 4 }),
            "[WorkbenchClickPool(4)]"
        );
        assert_eq!(
            acts(ActionDto::WorkbenchSetFocus { focus: FocusDto::Grid }),
            "[SetInputFocus(Grid)]"
        );
        assert_eq!(acts(ActionDto::WorkbenchClear), "[WorkbenchResetGrid]");
    }

    #[test]
    fn non_letter_rejected() {
        assert!(matches!(
            ActionDto::TypeLetter { ch: '1' }.into_actions(),
            Err(DtoError::InvalidLetter('1'))
        ));
        assert!(matches!(
            ActionDto::TypeLetter { ch: 'é' }.into_actions(),
            Err(DtoError::InvalidLetter('é'))
        ));
        assert!(matches!(
            ActionDto::CustomWordType { ch: ' ' }.into_actions(),
            Err(DtoError::InvalidLetter(' '))
        ));
        assert!(ActionDto::TypeLetter { ch: 'A' }.into_actions().is_ok());
    }

    #[test]
    fn wire_format_deserializes() {
        let a: ActionDto = serde_json::from_str(r#"{"type":"typeLetter","ch":"a"}"#).unwrap();
        assert_eq!(a, ActionDto::TypeLetter { ch: 'a' });
        let f: ActionDto =
            serde_json::from_str(r#"{"type":"workbenchSetFocus","focus":"grid"}"#).unwrap();
        assert_eq!(
            f,
            ActionDto::WorkbenchSetFocus {
                focus: FocusDto::Grid
            }
        );
    }
}
