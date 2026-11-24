//! Map crossterm events to `Action`.

use crate::app::{Action, InputFocus, Phase};
use crate::GameState;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventContext {
    pub phase: Phase,
    pub input_focus: InputFocus,
    pub has_workbench: bool,
}

impl EventContext {
    pub fn from_state(state: &GameState) -> Self {
        Self {
            phase: state.phase,
            input_focus: state.input_focus,
            has_workbench: state.workbench.is_some(),
        }
    }
}

pub fn map_event(event: &Event, ctx: &EventContext) -> Option<Action> {
    let KeyEvent {
        code,
        modifiers,
        kind,
        ..
    } = match event {
        Event::Key(key) => key,
        _ => return None,
    };

    if *kind == KeyEventKind::Release {
        return None;
    }

    if quit_chord(*code, *modifiers) {
        return Some(Action::Quit);
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        match (*code, ctx.phase) {
            (KeyCode::Char('c') | KeyCode::Char('C'), _) => return Some(Action::Quit),
            (KeyCode::Char('r') | KeyCode::Char('R'), Phase::Playing | Phase::Help) => {
                return Some(Action::ResetRound);
            }
            _ => {}
        }
    }

    if ctx.phase == Phase::CustomWordInput {
        return map_custom_word_input(*code);
    }

    if ctx.phase == Phase::CustomWordPick {
        return map_custom_word_pick(*code);
    }

    if matches!(ctx.phase, Phase::Playing | Phase::Help) {
        return map_playing_bucket(*code, *modifiers, ctx);
    }

    if ctx.phase == Phase::Won {
        if ctx.has_workbench {
            if workbench_ctrl_g_toggles(*modifiers, *code) {
                return Some(Action::ToggleWorkbenchFocus);
            }
            if ctx.input_focus == InputFocus::Grid {
                if let Some(action) = grid_map(*code, *modifiers) {
                    return Some(action);
                }
                return None;
            }
        }
        return map_navigation(*code, *modifiers, ctx.phase);
    }

    map_navigation(*code, *modifiers, ctx.phase)
}

fn workbench_ctrl_g_toggles(modifiers: KeyModifiers, code: KeyCode) -> bool {
    modifiers.contains(KeyModifiers::CONTROL)
        && !modifiers.contains(KeyModifiers::SHIFT)
        && matches!(code, KeyCode::Char('g') | KeyCode::Char('G'))
}

fn map_playing_bucket(code: KeyCode, modifiers: KeyModifiers, ctx: &EventContext) -> Option<Action> {
    if !ctx.has_workbench {
        return map_playing(code);
    }
    if workbench_ctrl_g_toggles(modifiers, code) {
        return Some(Action::ToggleWorkbenchFocus);
    }
    if ctx.input_focus == InputFocus::Grid {
        if let Some(action) = grid_map(code, modifiers) {
            return Some(action);
        }
        if matches!(code, KeyCode::Char('?') | KeyCode::Esc) {
            return map_playing(code);
        }
        return None;
    }
    map_playing(code)
}

fn grid_map(code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
    if modifiers.contains(KeyModifiers::CONTROL)
        && modifiers.contains(KeyModifiers::SHIFT)
        && matches!(code, KeyCode::Char('g') | KeyCode::Char('G'))
    {
        return Some(Action::WorkbenchResetGrid);
    }
    match code {
        KeyCode::Left => Some(Action::WorkbenchMove(-1, 0)),
        KeyCode::Right => Some(Action::WorkbenchMove(1, 0)),
        KeyCode::Up => Some(Action::WorkbenchMove(0, -1)),
        KeyCode::Down => Some(Action::WorkbenchMove(0, 1)),
        KeyCode::Char(' ') => Some(Action::WorkbenchPickPlace),
        KeyCode::Backspace => Some(Action::WorkbenchClearCell),
        KeyCode::Char(ch) if ch.is_ascii_alphabetic() => Some(Action::WorkbenchTypeLetter(ch)),
        _ => None,
    }
}

fn map_custom_word_pick(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Esc => Some(Action::Quit),
        KeyCode::Enter => Some(Action::ConfirmCustomPick),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(Action::CustomPickDelta(-1)),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(Action::CustomPickDelta(1)),
        KeyCode::Char('u') | KeyCode::Char('U') => Some(Action::CancelCustomWord),
        _ => None,
    }
}

fn map_custom_word_input(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Esc => Some(Action::CancelCustomWord),
        KeyCode::Enter | KeyCode::Char('\r') | KeyCode::Char('\n') => {
            Some(Action::SubmitCustomWord)
        }
        KeyCode::Backspace => Some(Action::CustomWordBackspace),
        KeyCode::Char(ch) if ch.is_ascii_alphabetic() => Some(Action::CustomWordType(ch)),
        _ => None,
    }
}

fn quit_chord(code: KeyCode, modifiers: KeyModifiers) -> bool {
    let command = modifiers.contains(KeyModifiers::SUPER) || modifiers.contains(KeyModifiers::META);
    command && matches!(code, KeyCode::Char('q') | KeyCode::Char('Q'))
}

/// During an active puzzle, letters go to the answer — not menu commands.
fn map_playing(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Esc => Some(Action::Quit),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Left => Some(Action::CursorLeft),
        KeyCode::Right => Some(Action::CursorRight),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Tab => Some(Action::RequestClue),
        KeyCode::Char(ch) if ch.is_ascii_alphabetic() => Some(Action::TypeLetter(ch)),
        _ => None,
    }
}

fn map_navigation(code: KeyCode, _modifiers: KeyModifiers, phase: Phase) -> Option<Action> {
    match code {
        KeyCode::Char('n') | KeyCode::Char('N') => Some(Action::NewPuzzle),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(Action::OpenSizeSelect),
        KeyCode::Char('i') | KeyCode::Char('I')
            if matches!(phase, Phase::Menu | Phase::Won) =>
        {
            Some(Action::OpenCustomWord)
        }
        KeyCode::Char('c') | KeyCode::Char('C') => Some(Action::RequestClue),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::ResetRound),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Esc => Some(Action::Quit),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Char('4') => Some(Action::SelectSize(4)),
        KeyCode::Char('5') => Some(Action::SelectSize(5)),
        KeyCode::Char('6') => Some(Action::SelectSize(6)),
        KeyCode::Char('7') => Some(Action::SelectSize(7)),
        KeyCode::Char('8') => Some(Action::SelectSize(8)),
        KeyCode::Char('9') => Some(Action::SelectSize(9)),
        KeyCode::Char('0') => Some(Action::SelectSize(10)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventState};
    use std::mem::discriminant;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn ctx_for_phase(phase: Phase) -> EventContext {
        ctx(phase, InputFocus::Answer, false)
    }

    fn ctx(phase: Phase, input_focus: InputFocus, has_workbench: bool) -> EventContext {
        EventContext {
            phase,
            input_focus,
            has_workbench,
        }
    }

    fn map_nav(event: &Event) -> Option<Action> {
        map_event(event, &ctx_for_phase(Phase::Menu))
    }

    fn map_play(event: &Event) -> Option<Action> {
        map_event(event, &ctx_for_phase(Phase::Playing))
    }

    #[test]
    fn quit_chord_and_ctrl_c() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Char('q'), KeyModifiers::SUPER),
                &ctx_for_phase(Phase::Menu)
            ),
            Some(Action::Quit)
        ));
        assert!(matches!(
            map_nav(&press(KeyCode::Char('q'), KeyModifiers::empty())),
            None
        ));
        assert!(matches!(
            map_event(
                &press(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &ctx_for_phase(Phase::Menu)
            ),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn command_keys_in_menu() {
        let cases: &[(KeyCode, fn(&Action) -> bool)] = &[
            (KeyCode::Char('n'), |a| matches!(a, Action::NewPuzzle)),
            (KeyCode::Char('i'), |a| matches!(a, Action::OpenCustomWord)),
            (KeyCode::Char('s'), |a| matches!(a, Action::OpenSizeSelect)),
            (KeyCode::Char('c'), |a| matches!(a, Action::RequestClue)),
            (KeyCode::Char('r'), |a| matches!(a, Action::ResetRound)),
        ];
        for (code, pred) in cases {
            let action = map_nav(&press(*code, KeyModifiers::empty()));
            assert!(action.as_ref().is_some_and(pred), "key {:?}", code);
        }
    }

    #[test]
    fn playing_treats_nscr_as_letters() {
        for ch in ['n', 's', 'c', 'r', 'q', 'N', 'S', 'Q'] {
            assert!(
                matches!(map_play(&press(KeyCode::Char(ch), KeyModifiers::empty())), Some(Action::TypeLetter(c)) if c == ch),
                "key {}",
                ch
            );
        }
    }

    #[test]
    fn playing_clue_on_tab() {
        assert!(matches!(
            map_play(&press(KeyCode::Tab, KeyModifiers::empty())),
            Some(Action::RequestClue)
        ));
    }

    #[test]
    fn playing_q_is_letter_not_quit() {
        assert!(matches!(
            map_play(&press(KeyCode::Char('q'), KeyModifiers::empty())),
            Some(Action::TypeLetter('q'))
        ));
        assert!(matches!(
            map_play(&press(KeyCode::Char('Q'), KeyModifiers::empty())),
            Some(Action::TypeLetter('Q'))
        ));
    }

    #[test]
    fn type_letters_case_insensitive() {
        assert!(matches!(
            map_play(&press(KeyCode::Char('a'), KeyModifiers::empty())),
            Some(Action::TypeLetter('a'))
        ));
    }

    #[test]
    fn navigation_command_letters_not_type_letter() {
        for ch in ['n', 's', 'c', 'r'] {
            let action = map_nav(&press(KeyCode::Char(ch), KeyModifiers::empty())).unwrap();
            assert_ne!(discriminant(&action), discriminant(&Action::TypeLetter(ch)));
        }
    }

    #[test]
    fn escape_quits() {
        assert!(matches!(
            map_play(&press(KeyCode::Esc, KeyModifiers::empty())),
            Some(Action::Quit)
        ));
        assert!(matches!(
            map_nav(&press(KeyCode::Esc, KeyModifiers::empty())),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn open_custom_word_from_won() {
        let i = press(KeyCode::Char('i'), KeyModifiers::empty());
        assert!(matches!(
            map_event(&i, &ctx_for_phase(Phase::Won)),
            Some(Action::OpenCustomWord)
        ));
    }

    #[test]
    fn custom_word_input_u_is_letter_not_cancel() {
        let u = press(KeyCode::Char('u'), KeyModifiers::empty());
        assert!(matches!(
            map_event(&u, &ctx_for_phase(Phase::CustomWordInput)),
            Some(Action::CustomWordType('u'))
        ));
        let enter = press(KeyCode::Char('\r'), KeyModifiers::empty());
        assert!(matches!(
            map_event(&enter, &ctx_for_phase(Phase::CustomWordInput)),
            Some(Action::SubmitCustomWord)
        ));
        let esc = press(KeyCode::Esc, KeyModifiers::empty());
        assert!(matches!(
            map_event(&esc, &ctx_for_phase(Phase::CustomWordInput)),
            Some(Action::CancelCustomWord)
        ));
    }

    #[test]
    fn without_workbench_behaves_like_ctx_for_phase() {
        let cases: &[(Phase, KeyCode)] = &[
            (Phase::Playing, KeyCode::Tab),
            (Phase::Playing, KeyCode::Char('g')),
            (Phase::Won, KeyCode::Char('n')),
            (Phase::Menu, KeyCode::Char('s')),
        ];
        for (phase, code) in cases {
            let event = press(*code, KeyModifiers::empty());
            let legacy = map_event(&event, &ctx_for_phase(*phase));
            let explicit = map_event(&event, &ctx(*phase, InputFocus::Answer, false));
            match (&legacy, &explicit) {
                (None, None) => {}
                (Some(a), Some(b)) => {
                    assert_eq!(
                        discriminant(a),
                        discriminant(b),
                        "phase {:?} key {:?}",
                        phase,
                        code
                    );
                }
                _ => panic!("mismatch phase {:?} key {:?}", phase, code),
            }
        }
    }

    #[test]
    fn workbench_playing_answer_g_types_letter() {
        for ch in ['g', 'G'] {
            assert!(matches!(
                map_event(
                    &press(KeyCode::Char(ch), KeyModifiers::empty()),
                    &ctx(Phase::Playing, InputFocus::Answer, true)
                ),
                Some(Action::TypeLetter(c)) if c == ch
            ));
        }
    }

    #[test]
    fn workbench_playing_answer_ctrl_g_toggles_to_grid() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Char('g'), KeyModifiers::CONTROL),
                &ctx(Phase::Playing, InputFocus::Answer, true)
            ),
            Some(Action::ToggleWorkbenchFocus)
        ));
    }

    #[test]
    fn workbench_playing_ctrl_g_toggles_from_grid() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Char('g'), KeyModifiers::CONTROL),
                &ctx(Phase::Playing, InputFocus::Grid, true)
            ),
            Some(Action::ToggleWorkbenchFocus)
        ));
    }

    #[test]
    fn workbench_grid_types_letter() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Char('a'), KeyModifiers::empty()),
                &ctx(Phase::Playing, InputFocus::Grid, true)
            ),
            Some(Action::WorkbenchTypeLetter('a'))
        ));
    }

    #[test]
    fn workbench_playing_answer_tab_still_clue() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Tab, KeyModifiers::empty()),
                &ctx(Phase::Playing, InputFocus::Answer, true)
            ),
            Some(Action::RequestClue)
        ));
    }

    #[test]
    fn workbench_playing_answer_space_noop() {
        assert!(map_event(
            &press(KeyCode::Char(' '), KeyModifiers::empty()),
            &ctx(Phase::Playing, InputFocus::Answer, true)
        )
        .is_none());
    }

    #[test]
    fn workbench_grid_space_pick_place() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Char(' '), KeyModifiers::empty()),
                &ctx(Phase::Playing, InputFocus::Grid, true)
            ),
            Some(Action::WorkbenchPickPlace)
        ));
    }

    #[test]
    fn workbench_grid_arrows_move() {
        let cases: &[(KeyCode, i32, i32)] = &[
            (KeyCode::Left, -1, 0),
            (KeyCode::Right, 1, 0),
            (KeyCode::Up, 0, -1),
            (KeyCode::Down, 0, 1),
        ];
        for (code, dx, dy) in cases {
            assert!(matches!(
                map_event(
                    &press(*code, KeyModifiers::empty()),
                    &ctx(Phase::Playing, InputFocus::Grid, true)
                ),
                Some(Action::WorkbenchMove(x, y)) if x == *dx && y == *dy
            ));
        }
    }

    #[test]
    fn workbench_grid_ctrl_shift_g_reset() {
        let mods = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
        for ch in ['g', 'G'] {
            assert!(matches!(
                map_event(
                    &press(KeyCode::Char(ch), mods),
                    &ctx(Phase::Playing, InputFocus::Grid, true)
                ),
                Some(Action::WorkbenchResetGrid)
            ));
        }
    }

    #[test]
    fn workbench_won_ctrl_g_toggles_from_grid() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Char('g'), KeyModifiers::CONTROL),
                &ctx(Phase::Won, InputFocus::Grid, true)
            ),
            Some(Action::ToggleWorkbenchFocus)
        ));
    }

    #[test]
    fn workbench_won_answer_n_new_puzzle() {
        assert!(matches!(
            map_event(
                &press(KeyCode::Char('n'), KeyModifiers::empty()),
                &ctx(Phase::Won, InputFocus::Answer, true)
            ),
            Some(Action::NewPuzzle)
        ));
    }

    #[test]
    fn ignores_release_and_non_key_events() {
        let release = Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        });
        assert!(map_nav(&release).is_none());
        assert!(map_event(&Event::Resize(80, 24), &ctx_for_phase(Phase::Menu)).is_none());
    }
}