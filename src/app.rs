//! Application state machine: `GameState`, `Action`, `reduce`.

use crate::config::WorkbenchSettings;
use crate::dictionary::{ClueKind, Dictionary, WordEntry};
use crate::game::{Round, SolveResult};
use crate::signature_index::{LookupResult, SignatureIndex};
use crate::workbench::LetterWorkbench;
use rand::rngs::StdRng;

const SIZE_MIN: u8 = 4;
const SIZE_MAX: u8 = 10;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Menu,
    SizeSelect,
    CustomWordInput,
    CustomWordPick,
    Playing,
    Help,
    Won,
}

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    ToggleHelp,
    NewPuzzle,
    OpenSizeSelect,
    SelectSize(u8),
    CancelSizeSelect,
    RequestClue,
    ResetRound,
    TypeLetter(char),
    Backspace,
    CursorLeft,
    CursorRight,
    CursorSet(usize),
    OpenCustomWord,
    CustomWordType(char),
    CustomWordBackspace,
    SubmitCustomWord,
    CancelCustomWord,
    CustomPickDelta(i32),
    ConfirmCustomPick,
    ToggleWorkbenchFocus,
    WorkbenchMove(i32, i32),
    WorkbenchPickPlace,
    WorkbenchClearCell,
    WorkbenchTypeLetter(char),
    WorkbenchResetGrid,
    CustomPickSelect(usize),
    WorkbenchClickCell(usize),
    WorkbenchClickPool(usize),
    SetInputFocus(InputFocus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputFocus {
    #[default]
    Answer,
    Grid,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    None,
    Error(String),
    Info(String),
    WrongAnswer,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub phase: Phase,
    pub size: u8,
    pub round: Option<Round>,
    pub current_word: Option<WordEntry>,
    /// Same-length dictionary words that share this round's letter multiset.
    pub anagram_alternates: Vec<WordEntry>,
    pub clues_used: usize,
    /// Advances on each Tab; `clue_index % budget` selects the active clue (cycles forever).
    pub clue_index: usize,
    pub active_clue_text: Option<String>,
    /// Set when the "correct positions" clue is used; matching slots show in reverse video.
    pub highlight_correct_positions: bool,
    pub status: Status,
    pub help_open: bool,
    pub min_word_len: u8,
    pub max_word_len: u8,
    /// Letters typed for “your anagram” before the round starts.
    pub custom_word_input: String,
    /// Dictionary hits when letters match more than one word (scramble disambiguation).
    pub custom_word_choices: Vec<WordEntry>,
    pub custom_word_pick: usize,
    pub workbench: Option<LetterWorkbench>,
    pub input_focus: InputFocus,
    pub workbench_settings: WorkbenchSettings,
}

impl GameState {
    pub fn new(default_size: u8) -> Self {
        Self::with_bounds(default_size, SIZE_MIN, SIZE_MAX)
    }

    pub fn with_bounds(default_size: u8, min_word_len: u8, max_word_len: u8) -> Self {
        Self {
            phase: Phase::Menu,
            size: default_size.clamp(min_word_len, max_word_len),
            round: None,
            current_word: None,
            anagram_alternates: Vec::new(),
            clues_used: 0,
            clue_index: 0,
            active_clue_text: None,
            highlight_correct_positions: false,
            status: Status::None,
            help_open: false,
            min_word_len,
            max_word_len,
            custom_word_input: String::new(),
            custom_word_choices: Vec::new(),
            custom_word_pick: 0,
            workbench: None,
            input_focus: InputFocus::Answer,
            workbench_settings: WorkbenchSettings {
                enabled: false,
                rows: 5,
                cols: 5,
            },
        }
    }
}

pub fn validate_custom_word(word: &str, min_len: u8, max_len: u8) -> Result<(), String> {
    let w = word.trim();
    if w.is_empty() {
        return Err("Enter a word.".into());
    }
    if !w.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("Use letters A–Z only.".into());
    }
    let len = w.len();
    let min = min_len as usize;
    let max = max_len as usize;
    if len < min || len > max {
        return Err(format!("Word length must be {min}–{max} letters."));
    }
    Ok(())
}

pub fn reduce(
    mut state: GameState,
    action: Action,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) -> GameState {
    match action {
        Action::Quit => {}
        Action::ToggleHelp => reduce_toggle_help(&mut state),
        Action::OpenSizeSelect => reduce_open_size_select(&mut state),
        Action::CancelSizeSelect => reduce_cancel_size_select(&mut state),
        Action::SelectSize(n) => reduce_select_size(&mut state, n, dict, index, rng),
        Action::NewPuzzle => reduce_new_puzzle(&mut state, dict, index, rng),
        Action::ResetRound => reduce_reset_round(&mut state, rng),
        Action::RequestClue => reduce_request_clue(&mut state),
        Action::TypeLetter(ch) => reduce_type_letter(&mut state, ch),
        Action::Backspace => reduce_backspace(&mut state),
        Action::CursorLeft => reduce_cursor(&mut state, -1),
        Action::CursorRight => reduce_cursor(&mut state, 1),
        Action::CursorSet(idx) => reduce_cursor_set(&mut state, idx),
        Action::OpenCustomWord => reduce_open_custom_word(&mut state),
        Action::CustomWordType(ch) => reduce_custom_word_type(&mut state, ch),
        Action::CustomWordBackspace => reduce_custom_word_backspace(&mut state),
        Action::SubmitCustomWord => reduce_submit_custom_word(&mut state, dict, index, rng),
        Action::CancelCustomWord => reduce_cancel_custom_word(&mut state),
        Action::CustomPickDelta(d) => reduce_custom_pick_delta(&mut state, d),
        Action::ConfirmCustomPick => reduce_confirm_custom_pick(&mut state, dict, index, rng),
        Action::ToggleWorkbenchFocus => reduce_toggle_workbench_focus(&mut state),
        Action::WorkbenchMove(dx, dy) => reduce_workbench_move(&mut state, dx, dy),
        Action::WorkbenchPickPlace => reduce_workbench_pick_place(&mut state),
        Action::WorkbenchClearCell => reduce_workbench_clear_cell(&mut state),
        Action::WorkbenchTypeLetter(ch) => reduce_workbench_type_letter(&mut state, ch),
        Action::WorkbenchResetGrid => reduce_workbench_reset_grid(&mut state, rng),
        Action::CustomPickSelect(i) => reduce_custom_pick_select(&mut state, i),
        Action::WorkbenchClickCell(i) => reduce_workbench_click_cell(&mut state, i),
        Action::WorkbenchClickPool(i) => reduce_workbench_click_pool(&mut state, i),
        Action::SetInputFocus(f) => reduce_set_input_focus(&mut state, f),
    }
    state
}

fn reduce_custom_pick_select(state: &mut GameState, idx: usize) {
    if state.phase != Phase::CustomWordPick || idx >= state.custom_word_choices.len() {
        return;
    }
    state.custom_word_pick = idx;
}

fn reduce_workbench_click_cell(state: &mut GameState, idx: usize) {
    if let Some(wb) = state.workbench.as_mut() {
        let _ = wb.click_cell(idx);
    }
}

fn reduce_workbench_click_pool(state: &mut GameState, idx: usize) {
    if let Some(wb) = state.workbench.as_mut() {
        let _ = wb.click_pool(idx);
    }
}

fn reduce_set_input_focus(state: &mut GameState, focus: InputFocus) {
    if focus == InputFocus::Grid && state.workbench.is_none() {
        return;
    }
    state.input_focus = focus;
}

fn reduce_toggle_workbench_focus(state: &mut GameState) {
    if state.workbench.is_none() {
        return;
    }
    state.input_focus = match state.input_focus {
        InputFocus::Answer => InputFocus::Grid,
        InputFocus::Grid => InputFocus::Answer,
    };
}

fn reduce_workbench_move(state: &mut GameState, dx: i32, dy: i32) {
    if let Some(wb) = state.workbench.as_mut() {
        let _ = wb.move_cursor(dx, dy);
    }
}

fn reduce_workbench_pick_place(state: &mut GameState) {
    if let Some(wb) = state.workbench.as_mut() {
        let _ = wb.pick_place_swap();
    }
}

fn reduce_workbench_clear_cell(state: &mut GameState) {
    if let Some(wb) = state.workbench.as_mut() {
        let _ = wb.clear_cell();
    }
}

fn reduce_workbench_type_letter(state: &mut GameState, ch: char) {
    if state.input_focus != InputFocus::Grid {
        return;
    }
    if let Some(wb) = state.workbench.as_mut() {
        let _ = wb.try_type_letter(ch);
    }
}

fn reduce_workbench_reset_grid(state: &mut GameState, rng: &mut StdRng) {
    if let Some(wb) = state.workbench.as_mut() {
        wb.reset_grid(rng);
    }
}

fn init_workbench(state: &GameState, rng: &mut StdRng) -> Option<LetterWorkbench> {
    if !state.workbench_settings.enabled {
        return None;
    }
    let word = state
        .round
        .as_ref()
        .map(|r| r.solution.as_str())
        .or_else(|| state.current_word.as_ref().map(|e| e.word.as_str()))?;
    Some(LetterWorkbench::from_solution(
        word,
        state.workbench_settings.rows,
        state.workbench_settings.cols,
        rng,
    ))
}

fn refresh_workbench(state: &mut GameState, rng: &mut StdRng) {
    state.workbench = init_workbench(state, rng);
    state.input_focus = InputFocus::Answer;
}

/// Start a round for a user-chosen solution word (dictionary entry used when present).
pub fn start_custom_word(
    mut state: GameState,
    word: &str,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) -> GameState {
    match validate_custom_word(word, state.min_word_len, state.max_word_len) {
        Ok(()) => match resolve_scramble_or_word(word, index) {
            ScrambleResolve::Solution(solution) => {
                start_round_solution(&mut state, dict, index, rng, &solution);
            }
            ScrambleResolve::Pick(choices) => {
                state.custom_word_input = word.to_ascii_lowercase();
                state.custom_word_choices = choices;
                state.custom_word_pick = 0;
                state.phase = Phase::CustomWordPick;
                state.status = Status::Info(
                    "Multiple words match; pick one in the TUI.".into(),
                );
            }
            ScrambleResolve::AsTypedFallback => {
                start_round_solution(&mut state, dict, index, rng, word);
            }
        },
        Err(msg) => {
            state.status = Status::Error(msg);
            state.phase = Phase::CustomWordInput;
            state.custom_word_input = word.to_ascii_lowercase();
        }
    }
    state
}

pub fn solve_after_fill(round: &Round) -> SolveResult {
    round.check_solution()
}

fn clamp_puzzle_size(size: u8) -> u8 {
    size.clamp(SIZE_MIN, SIZE_MAX)
}

fn reduce_open_custom_word(state: &mut GameState) {
    state.phase = Phase::CustomWordInput;
    state.custom_word_input.clear();
    state.custom_word_choices.clear();
    state.custom_word_pick = 0;
    state.status = Status::None;
    state.help_open = false;
}

fn reduce_custom_word_type(state: &mut GameState, ch: char) {
    if !matches!(state.phase, Phase::CustomWordInput) {
        return;
    }
    if state.custom_word_input.len() >= state.max_word_len as usize {
        state.status = Status::Error(format!(
            "Max length is {} letters.",
            state.max_word_len
        ));
        return;
    }
    if ch.is_ascii_alphabetic() {
        state.custom_word_input.push(ch.to_ascii_lowercase());
        state.status = Status::None;
    }
}

fn reduce_custom_word_backspace(state: &mut GameState) {
    if state.phase != Phase::CustomWordInput {
        return;
    }
    state.custom_word_input.pop();
    state.status = Status::None;
}

fn reduce_cancel_custom_word(state: &mut GameState) {
    state.custom_word_input.clear();
    state.custom_word_choices.clear();
    state.custom_word_pick = 0;
    state.status = Status::None;
    state.phase = Phase::Menu;
}

fn reduce_submit_custom_word(
    state: &mut GameState,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) {
    if state.phase != Phase::CustomWordInput {
        return;
    }
    let typed = state.custom_word_input.clone();
    match validate_custom_word(&typed, state.min_word_len, state.max_word_len) {
        Ok(()) => match resolve_scramble_or_word(&typed, index) {
            ScrambleResolve::Solution(word) => start_round_solution(state, dict, index, rng, &word),
            ScrambleResolve::Pick(choices) => {
                state.custom_word_choices = choices;
                state.custom_word_pick = 0;
                state.phase = Phase::CustomWordPick;
                state.status = Status::Info(format!(
                    "{} possible words — pick one and press Enter.",
                    state.custom_word_choices.len()
                ));
            }
            ScrambleResolve::AsTypedFallback => start_round_solution(state, dict, index, rng, &typed),
        },
        Err(msg) => state.status = Status::Error(msg),
    }
}

enum ScrambleResolve {
    Solution(String),
    Pick(Vec<WordEntry>),
    AsTypedFallback,
}

fn resolve_scramble_or_word(typed: &str, index: &SignatureIndex) -> ScrambleResolve {
    match index.lookup(typed) {
        LookupResult::NoMatch => ScrambleResolve::AsTypedFallback,
        LookupResult::Matches(entries) => {
            if let Some(exact) = entries
                .iter()
                .find(|e| e.word.eq_ignore_ascii_case(typed))
            {
                return ScrambleResolve::Solution(exact.word.clone());
            }
            if entries.len() == 1 {
                ScrambleResolve::Solution(entries[0].word.clone())
            } else {
                ScrambleResolve::Pick(entries)
            }
        }
    }
}

fn start_round_solution(
    state: &mut GameState,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
    solution: &str,
) {
    if begin_round_with_word(state, solution, dict, index, rng) {
        state.size = solution.len() as u8;
        state.phase = Phase::Playing;
        state.help_open = false;
        state.custom_word_input.clear();
        state.custom_word_choices.clear();
        state.custom_word_pick = 0;
        state.status = Status::None;
    } else {
        state.status = Status::Error("Could not start puzzle.".into());
    }
}

fn reduce_custom_pick_delta(state: &mut GameState, delta: i32) {
    if state.phase != Phase::CustomWordPick || state.custom_word_choices.is_empty() {
        return;
    }
    let n = state.custom_word_choices.len();
    let pick = state.custom_word_pick as i32 + delta;
    state.custom_word_pick = pick.rem_euclid(n as i32) as usize;
}

fn reduce_confirm_custom_pick(
    state: &mut GameState,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) {
    if state.phase != Phase::CustomWordPick {
        return;
    }
    let Some(entry) = state.custom_word_choices.get(state.custom_word_pick) else {
        return;
    };
    let word = entry.word.clone();
    start_round_solution(state, dict, index, rng, &word);
}

fn reduce_toggle_help(state: &mut GameState) {
    state.help_open = !state.help_open;
    match state.phase {
        Phase::Playing if state.help_open => state.phase = Phase::Help,
        Phase::Help if !state.help_open => state.phase = Phase::Playing,
        Phase::Won if state.help_open => {}
        Phase::Won if !state.help_open => {}
        _ => {}
    }
}

fn reduce_open_size_select(state: &mut GameState) {
    state.status = Status::None;
    state.phase = Phase::SizeSelect;
}

fn reduce_cancel_size_select(state: &mut GameState) {
    state.phase = if state.round.is_some() {
        Phase::Playing
    } else {
        Phase::Menu
    };
}

fn reduce_select_size(
    state: &mut GameState,
    n: u8,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) {
    state.size = clamp_puzzle_size(n);
    if begin_round(state, dict, index, rng) {
        state.phase = Phase::Playing;
        state.help_open = false;
    } else if state.round.is_none() {
        state.phase = Phase::Menu;
    }
}

fn reduce_new_puzzle(
    state: &mut GameState,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) {
    state.status = Status::None;
    if begin_round(state, dict, index, rng) {
        state.phase = Phase::Playing;
        state.help_open = false;
    }
}

fn reduce_reset_round(state: &mut GameState, rng: &mut StdRng) {
    if !matches!(state.phase, Phase::Playing | Phase::Help | Phase::Won) {
        return;
    }
    let Some(round) = state.round.as_mut() else {
        return;
    };
    round.reset_same_word(rng);
    state.clues_used = 0;
    state.clue_index = 0;
    state.active_clue_text = None;
    state.highlight_correct_positions = false;
    state.status = Status::None;
    refresh_workbench(state, rng);
    if state.phase == Phase::Won {
        state.phase = Phase::Playing;
    }
}

fn reduce_request_clue(state: &mut GameState) {
    if !matches!(state.phase, Phase::Playing | Phase::Help) {
        return;
    }
    let Some(entry) = state.current_word.as_ref() else {
        return;
    };
    let clues = Dictionary::clues_for_round(entry, &state.anagram_alternates);
    let clue_budget = clues.len();
    if clue_budget == 0 {
        state.status = Status::Info("No clues for this word.".into());
        return;
    }
    let clue_idx = state.clue_index % clue_budget;
    apply_clue(state, &clues[clue_idx]);
    state.clue_index += 1;
    state.clues_used = state.clue_index.min(clue_budget);
    state.status = Status::None;
    auto_check_solution(state);
}

fn reduce_type_letter(state: &mut GameState, ch: char) {
    if !matches!(state.phase, Phase::Playing | Phase::Help) {
        return;
    }
    let Some(round) = state.round.as_mut() else {
        return;
    };
    if round.try_type_letter(ch) {
        state.status = Status::None;
        refresh_correct_position_clue(state);
        auto_check_solution(state);
    } else {
        state.status = Status::Error("Letter not in pool.".into());
    }
}

fn reduce_cursor(state: &mut GameState, delta: i32) {
    if !matches!(state.phase, Phase::Playing | Phase::Help) {
        return;
    }
    let Some(round) = state.round.as_mut() else {
        return;
    };
    if round.move_cursor(delta) {
        state.status = Status::None;
    }
}

fn reduce_cursor_set(state: &mut GameState, idx: usize) {
    if !matches!(state.phase, Phase::Playing | Phase::Help) {
        return;
    }
    let Some(round) = state.round.as_mut() else {
        return;
    };
    if idx < round.answer.len() {
        round.cursor = idx;
        state.status = Status::None;
    }
}

fn reduce_backspace(state: &mut GameState) {
    if !matches!(state.phase, Phase::Playing | Phase::Help) {
        return;
    }
    let Some(round) = state.round.as_mut() else {
        return;
    };
    if round.backspace() {
        if matches!(state.status, Status::WrongAnswer) {
            state.status = Status::None;
        }
        refresh_correct_position_clue(state);
    }
}

fn same_length_anagram_alternates(index: &SignatureIndex, solution: &str) -> Vec<WordEntry> {
    let target_len = solution.len();
    let LookupResult::Matches(entries) = index.lookup(solution) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter(|e| e.word.len() == target_len && !e.word.eq_ignore_ascii_case(solution))
        .collect()
}

fn begin_round_with_word(
    state: &mut GameState,
    word: &str,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) -> bool {
    if validate_custom_word(word, state.min_word_len, state.max_word_len).is_err() {
        return false;
    }
    let entry = dict.resolve_entry(word);
    state.current_word = Some(entry);
    state.anagram_alternates = same_length_anagram_alternates(index, word);
    state.round = Some(Round::new(word, rng));
    state.clues_used = 0;
    state.clue_index = 0;
    state.active_clue_text = None;
    state.highlight_correct_positions = false;
    state.status = Status::None;
    refresh_workbench(state, rng);
    true
}

fn begin_round(
    state: &mut GameState,
    dict: &Dictionary,
    index: &SignatureIndex,
    rng: &mut StdRng,
) -> bool {
    let exclude = state
        .current_word
        .as_ref()
        .map(|e| e.word.as_str());
    let Some(entry) = dict.random_word_excluding(state.size as usize, exclude, rng) else {
        state.status = Status::Error(format!(
            "No words of length {} in dictionary.",
            state.size
        ));
        return false;
    };
    let entry = entry.clone();
    state.current_word = Some(entry.clone());
    state.anagram_alternates = same_length_anagram_alternates(index, &entry.word);
    state.round = Some(Round::new(&entry.word, rng));
    state.clues_used = 0;
    state.clue_index = 0;
    state.active_clue_text = None;
    state.highlight_correct_positions = false;
    state.status = Status::None;
    refresh_workbench(state, rng);
    true
}

fn apply_clue(state: &mut GameState, clue: &ClueKind) {
    match clue {
        ClueKind::Definition { text } => {
            state.active_clue_text = Some(text.clone());
        }
        ClueKind::CorrectPositions => {
            state.highlight_correct_positions = true;
            refresh_correct_position_clue(state);
        }
        ClueKind::FirstLetter => {
            state.active_clue_text = Some("First letter revealed.".into());
            if let Some(round) = state.round.as_mut() {
                reveal_first_letter(round);
            }
        }
        ClueKind::VowelCount { text } => {
            state.active_clue_text = Some(text.clone());
        }
        ClueKind::Category { text } => {
            state.active_clue_text = Some(text.clone());
        }
    }
}

pub fn format_correct_positions_clue(round: &Round) -> String {
    let mut parts = Vec::new();
    for (i, slot) in round.answer.iter().enumerate() {
        if round.slot_is_correct(i) {
            if let Some(ch) = slot {
                parts.push(format!("{}={}", i + 1, ch));
            }
        }
    }
    if parts.is_empty() {
        "Correct positions: none yet (matching slots use reverse video).".into()
    } else {
        format!(
            "Correct positions: {} (reverse video in answer).",
            parts.join(", ")
        )
    }
}

fn refresh_correct_position_clue(state: &mut GameState) {
    if !state.highlight_correct_positions {
        return;
    }
    if let Some(round) = state.round.as_ref() {
        state.active_clue_text = Some(format_correct_positions_clue(round));
    }
}

fn reveal_first_letter(round: &mut Round) {
    let Some(first) = round.solution.chars().next() else {
        return;
    };
    if round.answer.first().and_then(|s| *s).is_some() {
        return;
    }
    let upper = first.to_ascii_uppercase();
    if let Some(idx) = round
        .pool
        .iter()
        .position(|c| c.eq_ignore_ascii_case(&upper))
    {
        round.pool.remove(idx);
    }
    round.answer[0] = Some(upper);
    round.snap_cursor_to_next_empty();
    round.assert_invariant();
}

fn state_guess_matches_alternates(guess: &str, alternates: &[WordEntry]) -> bool {
    alternates
        .iter()
        .any(|e| e.word.eq_ignore_ascii_case(guess))
}

fn auto_check_solution(state: &mut GameState) {
    let Some(round) = state.round.as_ref() else {
        return;
    };
    if !round.is_answer_full() {
        return;
    }
    let guess: String = round
        .answer
        .iter()
        .map(|s| s.expect("full answer"))
        .collect();
    let correct = guess.eq_ignore_ascii_case(&round.solution)
        || state_guess_matches_alternates(&guess, &state.anagram_alternates);
    if correct {
        state.phase = Phase::Won;
        state.status = Status::Info("Correct!".into());
        state.help_open = false;
    } else {
        state.status = Status::WrongAnswer;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::multiset_matches;
    use crate::signature_index::SignatureIndex;
    use rand::SeedableRng;

    fn embedded_index() -> SignatureIndex {
        SignatureIndex::from_embedded().expect("embedded index")
    }

    fn seeded_dict_playing(seed: u64, size: u8) -> (GameState, Dictionary, SignatureIndex, StdRng) {
        let dict = Dictionary::from_embedded().expect("embedded dictionary");
        let mut rng = StdRng::seed_from_u64(seed);
        let mut state = GameState::new(size);
        let index = embedded_index();
        state = reduce(state, Action::NewPuzzle, &dict, &index, &mut rng);
        assert_eq!(state.phase, Phase::Playing);
        assert!(state.round.is_some());
        (state, dict, index, rng)
    }

    fn playing_with_word(word: &str, size: u8, seed: u64) -> (GameState, Dictionary, SignatureIndex, StdRng) {
        let dict = Dictionary::from_embedded().expect("embedded dictionary");
        let index = embedded_index();
        let mut rng = StdRng::seed_from_u64(seed);
        let mut state = GameState::new(size);
        begin_round_with_word(&mut state, word, &dict, &index, &mut rng)
            .then_some(())
            .expect("begin round");
        state.phase = Phase::Playing;
        (state, dict, index, rng)
    }

    #[test]
    fn alternate_anagram_clue_and_win() {
        let (mut state, dict, index, mut rng) = playing_with_word("aliment", 7, 1);
        assert!(
            state
                .anagram_alternates
                .iter()
                .any(|e| e.word.eq_ignore_ascii_case("ailment")),
            "expected ailment alternate {:?}",
            state.anagram_alternates
        );
        let entry = state.current_word.as_ref().expect("word").clone();
        let clues = Dictionary::clues_for_round(&entry, &state.anagram_alternates);
        let defs: Vec<&str> = clues
            .iter()
            .filter_map(|c| match c {
                ClueKind::Definition { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            defs.len() >= 2,
            "expected definitions for primary + alternate: {:?}",
            clues
        );
        for text in &defs {
            assert!(
                !text.to_ascii_lowercase().starts_with("aliment")
                    && !text.to_ascii_lowercase().starts_with("ailment"),
                "clue must not reveal answer spelling: {text:?}"
            );
        }
        assert!(
            defs.iter().any(|t| t.contains("bodily disorder")),
            "expected alternate sense gloss: {:?}",
            defs
        );
        state = reduce(state, Action::RequestClue, &dict, &index, &mut rng);
        if let Some(round) = state.round.as_mut() {
            for ch in "ailment".chars() {
                let _ = round.try_type_letter(ch);
            }
        }
        auto_check_solution(&mut state);
        assert_eq!(state.phase, Phase::Won);
    }

    fn solve_round_via_typing(
        mut state: GameState,
        dict: &Dictionary,
        index: &SignatureIndex,
        rng: &mut StdRng,
    ) -> GameState {
        let solution = state.round.as_ref().expect("round").solution.clone();
        for (i, ch) in solution.chars().enumerate() {
            if let Some(round) = state.round.as_mut() {
                round.cursor = i;
            }
            state = reduce(state, Action::TypeLetter(ch), dict, index, rng);
        }
        state
    }

    #[test]
    fn menu_new_puzzle_starts_playing() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let state = GameState::new(5);
        assert_eq!(state.phase, Phase::Menu);
        let state = reduce(state, Action::NewPuzzle, &dict, &embedded_index(), &mut rng);
        assert_eq!(state.phase, Phase::Playing);
        assert!(state.current_word.is_some());
        assert!(state.round.is_some());
        assert_eq!(state.clues_used, 0);
    }

    #[test]
    fn size_select_flow_from_menu() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(2);
        let state = GameState::new(5);
        let state = reduce(state, Action::OpenSizeSelect, &dict, &embedded_index(), &mut rng);
        assert_eq!(state.phase, Phase::SizeSelect);
        let state = reduce(state, Action::SelectSize(6), &dict, &embedded_index(), &mut rng);
        assert_eq!(state.phase, Phase::Playing);
        assert_eq!(state.size, 6);
        let round = state.round.as_ref().unwrap();
        assert_eq!(round.solution.len(), 6);
    }

    #[test]
    fn cancel_size_select_without_round_returns_menu() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(3);
        let state = reduce(GameState::new(5), Action::OpenSizeSelect, &dict, &embedded_index(), &mut rng);
        let state = reduce(state, Action::CancelSizeSelect, &dict, &embedded_index(), &mut rng);
        assert_eq!(state.phase, Phase::Menu);
    }

    #[test]
    fn clue_tab_cycles_after_budget() {
        let (mut state, dict, index, mut rng) = playing_with_word("heart", 5, 10);
        let entry = state.current_word.as_ref().expect("word").clone();
        let clues = Dictionary::clues_for_round(&entry, &state.anagram_alternates);
        let budget = clues.len();
        let first_definition = match &clues[0] {
            ClueKind::Definition { text } => text.clone(),
            _ => panic!("first clue is definition"),
        };
        for _ in 0..budget {
            state = reduce(state, Action::RequestClue, &dict, &embedded_index(), &mut rng);
            assert!(state.clues_used <= budget);
        }
        assert_eq!(state.clues_used, budget);
        assert_eq!(state.clue_index, budget);
        state = reduce(state, Action::RequestClue, &dict, &embedded_index(), &mut rng);
        assert_eq!(state.clues_used, budget);
        assert_eq!(state.clue_index, budget + 1);
        assert_eq!(state.active_clue_text.as_deref(), Some(first_definition.as_str()));
        assert!(matches!(state.status, Status::None));
    }

    #[test]
    fn first_letter_clue_preserves_multiset() {
        let (mut state, dict, index, mut rng) = playing_with_word("heart", 5, 99);
        let solution = state.round.as_ref().unwrap().solution.clone();
        let entry = state.current_word.as_ref().expect("word entry");
        let clues = Dictionary::clues_for_round(entry, &state.anagram_alternates);
        let first_letter_idx = clues
            .iter()
            .position(|c| matches!(c, ClueKind::FirstLetter));
        if let Some(idx) = first_letter_idx {
            for _ in 0..=idx {
                state = reduce(state, Action::RequestClue, &dict, &embedded_index(), &mut rng);
            }
        }
        let round = state.round.as_ref().unwrap();
        assert!(multiset_matches(
            &solution,
            &round.pool,
            &round.answer
        ));
        if first_letter_idx.is_some() {
            assert_eq!(
                round.answer[0],
                Some(solution.chars().next().unwrap().to_ascii_uppercase())
            );
        }
    }

    #[test]
    fn correct_positions_clue_highlights_matching_slots() {
        let (mut state, dict, index, mut rng) = playing_with_word("heart", 5, 5);
        let solution: Vec<char> = state.round.as_ref().unwrap().solution.chars().collect();
        if let Some(round) = state.round.as_mut() {
            round.cursor = 0;
            round.try_type_letter(solution[0]);
        }
        let entry = state.current_word.as_ref().expect("word entry");
        let pos_idx = Dictionary::clues_for_round(entry, &state.anagram_alternates)
            .iter()
            .position(|c| matches!(c, ClueKind::CorrectPositions))
            .expect("correct positions clue");
        for _ in 0..=pos_idx {
            state = reduce(state, Action::RequestClue, &dict, &embedded_index(), &mut rng);
        }
        assert!(state.highlight_correct_positions);
        assert!(state
            .active_clue_text
            .as_ref()
            .unwrap()
            .contains("1=H"));
        assert!(state.round.as_ref().unwrap().slot_is_correct(0));
    }

    #[test]
    fn auto_win_on_correct_full_answer() {
        let (mut state, dict, index, mut rng) = seeded_dict_playing(42, 4);
        state = solve_round_via_typing(state, &dict, &index, &mut rng);
        assert_eq!(state.phase, Phase::Won);
        assert!(matches!(state.status, Status::Info(_)));
    }

    #[test]
    fn wrong_full_word_sets_wrong_answer_status() {
        let (mut state, dict, index, mut rng) = seeded_dict_playing(7, 4);
        let round = state.round.as_ref().unwrap();
        let mut wrong: Vec<char> = round.solution.chars().collect();
        wrong.reverse();
        for (i, ch) in wrong.into_iter().enumerate() {
            if let Some(round) = state.round.as_mut() {
                round.cursor = i;
            }
            state = reduce(state, Action::TypeLetter(ch), &dict, &embedded_index(), &mut rng);
        }
        assert_eq!(state.phase, Phase::Playing);
        assert_eq!(state.status, Status::WrongAnswer);
    }

    #[test]
    fn reduce_sequence_type_backspace_and_reset() {
        let (mut state, dict, index, mut rng) = seeded_dict_playing(55, 5);
        let first = state.round.as_ref().unwrap().pool[0];
        state = reduce(state, Action::TypeLetter(first), &dict, &embedded_index(), &mut rng);
        assert_eq!(state.round.as_ref().unwrap().cursor, 1);
        state = reduce(state, Action::Backspace, &dict, &embedded_index(), &mut rng);
        assert_eq!(state.round.as_ref().unwrap().cursor, 0);
        state = reduce(state, Action::ResetRound, &dict, &embedded_index(), &mut rng);
        assert_eq!(state.clues_used, 0);
        assert!(state.active_clue_text.is_none());
        assert_eq!(state.round.as_ref().unwrap().cursor, 0);
    }

    #[test]
    fn cursor_set_moves_cursor_in_range() {
        let (mut state, dict, index, mut rng) = seeded_dict_playing(55, 5);
        state = reduce(state, Action::CursorSet(3), &dict, &index, &mut rng);
        assert_eq!(state.round.as_ref().unwrap().cursor, 3);
        state = reduce(state, Action::CursorSet(0), &dict, &index, &mut rng);
        assert_eq!(state.round.as_ref().unwrap().cursor, 0);
    }

    #[test]
    fn cursor_set_out_of_range_is_noop() {
        let (mut state, dict, index, mut rng) = seeded_dict_playing(55, 5);
        state = reduce(state, Action::CursorSet(2), &dict, &index, &mut rng);
        state = reduce(state, Action::CursorSet(5), &dict, &index, &mut rng);
        state = reduce(state, Action::CursorSet(usize::MAX), &dict, &index, &mut rng);
        assert_eq!(state.round.as_ref().unwrap().cursor, 2);
    }

    #[test]
    fn cursor_set_ignored_outside_play_phases() {
        let (mut state, dict, index, mut rng) = seeded_dict_playing(55, 5);
        for phase in [
            Phase::Won,
            Phase::CustomWordInput,
            Phase::CustomWordPick,
            Phase::Menu,
        ] {
            state.phase = phase;
            state = reduce(state, Action::CursorSet(4), &dict, &index, &mut rng);
            assert_eq!(state.round.as_ref().unwrap().cursor, 0, "{phase:?}");
        }
        state.phase = Phase::Help;
        state = reduce(state, Action::CursorSet(4), &dict, &index, &mut rng);
        assert_eq!(state.round.as_ref().unwrap().cursor, 4);
    }

    #[test]
    fn cursor_set_then_type_fills_clicked_slot() {
        let (mut state, dict, index, mut rng) = seeded_dict_playing(55, 5);
        let letter = state.round.as_ref().unwrap().pool[0];
        state = reduce(state, Action::CursorSet(2), &dict, &index, &mut rng);
        state = reduce(state, Action::TypeLetter(letter), &dict, &index, &mut rng);
        let round = state.round.as_ref().unwrap();
        assert_eq!(round.answer[2], Some(letter));
        assert!(round.answer[0].is_none());
    }

    #[test]
    fn new_puzzle_prefers_different_word_at_same_size() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(77);
        let mut state = GameState::new(5);
        state = reduce(state, Action::NewPuzzle, &dict, &embedded_index(), &mut rng);
        let first = state
            .current_word
            .as_ref()
            .map(|w| w.word.to_ascii_lowercase())
            .expect("word");
        state.phase = Phase::Won;
        state = reduce(state, Action::NewPuzzle, &dict, &embedded_index(), &mut rng);
        let second = state
            .current_word
            .as_ref()
            .map(|w| w.word.to_ascii_lowercase())
            .expect("word");
        let choices: Vec<_> = dict
            .words_with_length(5)
            .into_iter()
            .map(|e| e.word.to_ascii_lowercase())
            .collect();
        if choices.len() > 1 {
            assert_ne!(first, second);
        }
    }

    #[test]
    fn validate_custom_word_rules() {
        assert!(validate_custom_word("", 4, 10).is_err());
        assert!(validate_custom_word("ab1", 4, 10).is_err());
        assert!(validate_custom_word("abc", 4, 10).is_err());
        assert!(validate_custom_word("abcdefghijk", 4, 10).is_err());
        assert!(validate_custom_word("custom", 4, 10).is_ok());
    }

    #[test]
    fn custom_scramble_resolves_to_puzzle() {
        let dict = Dictionary::from_embedded().unwrap();
        let index = embedded_index();
        let mut rng = StdRng::seed_from_u64(905);
        let mut state = GameState::new(5);
        state = reduce(state, Action::OpenCustomWord, &dict, &index, &mut rng);
        for ch in "tinles".chars() {
            state = reduce(state, Action::CustomWordType(ch), &dict, &index, &mut rng);
        }
        state = reduce(state, Action::SubmitCustomWord, &dict, &index, &mut rng);
        assert!(
            state.phase == Phase::Playing || state.phase == Phase::CustomWordPick,
            "expected puzzle or pick list"
        );
        if state.phase == Phase::CustomWordPick {
            state = reduce(state, Action::ConfirmCustomPick, &dict, &index, &mut rng);
        }
        assert_eq!(state.phase, Phase::Playing);
        assert!(state.round.is_some());
        assert_eq!(state.round.as_ref().unwrap().solution.len(), 6);
    }

    #[test]
    fn custom_scramble_with_u_letter_starts_playing() {
        let dict = Dictionary::from_embedded().unwrap();
        let index = embedded_index();
        let mut rng = StdRng::seed_from_u64(906);
        let mut state = reduce(
            GameState::new(5),
            Action::OpenCustomWord,
            &dict,
            &index,
            &mut rng,
        );
        for ch in "baldiesu".chars() {
            state = reduce(state, Action::CustomWordType(ch), &dict, &index, &mut rng);
        }
        assert_eq!(state.custom_word_input, "baldiesu");
        assert_eq!(state.phase, Phase::CustomWordInput);
        state = reduce(state, Action::SubmitCustomWord, &dict, &index, &mut rng);
        assert_eq!(state.phase, Phase::Playing);
        assert_eq!(
            state.round.as_ref().unwrap().solution.to_lowercase(),
            "audibles"
        );
    }

    #[test]
    fn custom_word_after_win_opens_input() {
        let dict = Dictionary::from_embedded().unwrap();
        let index = embedded_index();
        let mut rng = StdRng::seed_from_u64(907);
        let mut state = start_custom_word(
            GameState::new(5),
            "plum",
            &dict,
            &index,
            &mut rng,
        );
        assert_eq!(state.phase, Phase::Playing);
        state.phase = Phase::Won;
        state = reduce(state, Action::OpenCustomWord, &dict, &index, &mut rng);
        assert_eq!(state.phase, Phase::CustomWordInput);
        for ch in "hart".chars() {
            state = reduce(state, Action::CustomWordType(ch), &dict, &index, &mut rng);
        }
        state = reduce(state, Action::SubmitCustomWord, &dict, &index, &mut rng);
        assert_eq!(state.phase, Phase::Playing);
    }

    #[test]
    fn custom_word_submit_starts_playing() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(901);
        let index = embedded_index();
        let state = reduce(
            GameState::new(5),
            Action::OpenCustomWord,
            &dict,
            &index,
            &mut rng,
        );
        assert_eq!(state.phase, Phase::CustomWordInput);
        let mut state = state;
        for ch in "plum".chars() {
            state = reduce(state, Action::CustomWordType(ch), &dict, &index, &mut rng);
        }
        state = reduce(state, Action::SubmitCustomWord, &dict, &index, &mut rng);
        assert_eq!(state.phase, Phase::Playing);
        assert_eq!(state.size, 4);
        assert_eq!(state.round.as_ref().unwrap().solution.to_lowercase(), "plum");
    }

    #[test]
    fn custom_unknown_word_uses_synthetic_definition() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(902);
        let state = start_custom_word(GameState::new(5), "zzyx", &dict, &embedded_index(), &mut rng);
        assert_eq!(state.phase, Phase::Playing);
        let def = state
            .current_word
            .as_ref()
            .and_then(|e| e.definition.as_deref())
            .unwrap_or("");
        assert!(def.contains("Custom puzzle"));
    }

    #[test]
    fn select_size_clamps_to_bounds() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(8);
        let index = embedded_index();
        let state = reduce(
            GameState::new(5),
            Action::SelectSize(2),
            &dict,
            &index,
            &mut rng,
        );
        assert_eq!(state.size, SIZE_MIN);
        let state = reduce(state, Action::SelectSize(20), &dict, &index, &mut rng);
        assert_eq!(state.size, SIZE_MAX);
    }

    fn state_with_workbench(size: u8) -> GameState {
        let mut state = GameState::new(size);
        state.workbench_settings.enabled = true;
        state.workbench_settings.rows = 5;
        state.workbench_settings.cols = 5;
        state
    }

    #[test]
    fn workbench_disabled_stays_none_on_new_puzzle() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(501);
        let state = reduce(
            GameState::new(5),
            Action::NewPuzzle,
            &dict,
            &embedded_index(),
            &mut rng,
        );
        assert_eq!(state.phase, Phase::Playing);
        assert!(state.workbench.is_none());
    }

    #[test]
    fn workbench_pick_place_does_not_change_round_pool() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(502);
        let state = reduce(
            state_with_workbench(5),
            Action::NewPuzzle,
            &dict,
            &embedded_index(),
            &mut rng,
        );
        let pool_before: Vec<char> = state.round.as_ref().unwrap().pool.clone();
        let state = reduce(
            state,
            Action::WorkbenchPickPlace,
            &dict,
            &embedded_index(),
            &mut rng,
        );
        assert_eq!(state.round.as_ref().unwrap().pool, pool_before);
        assert!(state.workbench.as_ref().unwrap().cells.iter().any(|c| c.is_some()));
    }

    #[test]
    fn new_puzzle_reinits_workbench() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(503);
        let mut state = reduce(
            state_with_workbench(5),
            Action::NewPuzzle,
            &dict,
            &embedded_index(),
            &mut rng,
        );
        assert!(state.workbench.is_some());
        state = reduce(
            state,
            Action::WorkbenchPickPlace,
            &dict,
            &embedded_index(),
            &mut rng,
        );
        assert!(state
            .workbench
            .as_ref()
            .unwrap()
            .cells
            .iter()
            .any(|c| c.is_some()));
        state = reduce(state, Action::NewPuzzle, &dict, &embedded_index(), &mut rng);
        assert!(state.workbench.is_some());
        assert!(state
            .workbench
            .as_ref()
            .unwrap()
            .cells
            .iter()
            .all(|c| c.is_none()));
    }

    #[test]
    fn reset_round_reinits_workbench_and_answer_focus() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng = StdRng::seed_from_u64(504);
        let mut state = reduce(
            state_with_workbench(5),
            Action::NewPuzzle,
            &dict,
            &embedded_index(),
            &mut rng,
        );
        state.input_focus = InputFocus::Grid;
        state = reduce(
            state,
            Action::WorkbenchPickPlace,
            &dict,
            &embedded_index(),
            &mut rng,
        );
        state = reduce(state, Action::ResetRound, &dict, &embedded_index(), &mut rng);
        assert_eq!(state.input_focus, InputFocus::Answer);
        assert!(state
            .workbench
            .as_ref()
            .unwrap()
            .cells
            .iter()
            .all(|c| c.is_none()));
    }

    #[test]
    fn custom_pick_select_only_in_pick_phase_and_bounded() {
        let dict = Dictionary::from_embedded().unwrap();
        let index = embedded_index();
        let mut rng = StdRng::seed_from_u64(610);
        let choices: Vec<WordEntry> = dict
            .words_with_length(5)
            .into_iter()
            .take(3)
            .cloned()
            .collect();
        assert_eq!(choices.len(), 3);
        let mut state = GameState::new(5);
        state.custom_word_choices = choices;
        state.phase = Phase::CustomWordPick;
        state = reduce(state, Action::CustomPickSelect(2), &dict, &index, &mut rng);
        assert_eq!(state.custom_word_pick, 2);
        assert_eq!(state.phase, Phase::CustomWordPick);
        state = reduce(state, Action::CustomPickSelect(3), &dict, &index, &mut rng);
        assert_eq!(state.custom_word_pick, 2);
        state.phase = Phase::Playing;
        state = reduce(state, Action::CustomPickSelect(0), &dict, &index, &mut rng);
        assert_eq!(state.custom_word_pick, 2);
        assert_eq!(state.phase, Phase::Playing);
    }

    #[test]
    fn set_input_focus_grid_ignored_without_workbench() {
        let dict = Dictionary::from_embedded().unwrap();
        let index = embedded_index();
        let mut rng = StdRng::seed_from_u64(611);
        let state = reduce(GameState::new(5), Action::NewPuzzle, &dict, &index, &mut rng);
        assert!(state.workbench.is_none());
        let state = reduce(state, Action::SetInputFocus(InputFocus::Grid), &dict, &index, &mut rng);
        assert_eq!(state.input_focus, InputFocus::Answer);

        let mut state = reduce(state_with_workbench(5), Action::NewPuzzle, &dict, &index, &mut rng);
        state = reduce(state, Action::SetInputFocus(InputFocus::Grid), &dict, &index, &mut rng);
        assert_eq!(state.input_focus, InputFocus::Grid);
        state = reduce(state, Action::SetInputFocus(InputFocus::Answer), &dict, &index, &mut rng);
        assert_eq!(state.input_focus, InputFocus::Answer);
    }

    #[test]
    fn workbench_click_actions_reduce_and_keep_round_pool() {
        let dict = Dictionary::from_embedded().unwrap();
        let index = embedded_index();
        let mut rng = StdRng::seed_from_u64(612);
        let state = reduce(GameState::new(5), Action::NewPuzzle, &dict, &index, &mut rng);
        let state = reduce(state, Action::WorkbenchClickCell(0), &dict, &index, &mut rng);
        assert!(state.workbench.is_none());

        let mut state = reduce(state_with_workbench(5), Action::NewPuzzle, &dict, &index, &mut rng);
        let round_pool = state.round.as_ref().unwrap().pool.clone();
        state = reduce(state, Action::WorkbenchClickPool(0), &dict, &index, &mut rng);
        assert!(state.workbench.as_ref().unwrap().held.is_some());
        assert_eq!(state.workbench.as_ref().unwrap().pool.len(), 4);
        state = reduce(state, Action::WorkbenchClickCell(0), &dict, &index, &mut rng);
        let wb = state.workbench.as_ref().unwrap();
        assert!(wb.cells[0].is_some());
        assert_eq!(wb.held, None);
        assert_eq!(state.round.as_ref().unwrap().pool, round_pool);
    }
}
