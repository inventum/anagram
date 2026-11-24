//! Engine (read-only data) and Session (mutable game state). See SPEC-ui §4.3.

use std::sync::{Arc, Mutex, MutexGuard};

use anagram::affix_hints::{affix_hints, AffixHints};
use anagram::app::{self, Action, GameState, Phase};
use anagram::config::AppConfig;
use anagram::dictionary::Dictionary;
use anagram::signature_index::SignatureIndex;
use anyhow::bail;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::dto::{self, ActionDto, DtoError, ThemeDto, UiState};
use crate::theme;

/// Core size bounds (`app.rs` SIZE_MIN / SIZE_MAX). Effective bounds are config ∩ these (D-4).
const SIZE_MIN: u8 = 4;
const SIZE_MAX: u8 = 10;

/// Read-only game data. Shared via `Arc`, no lock.
pub struct Engine {
    pub dict: Dictionary,
    pub index: SignatureIndex,
    pub theme: ThemeDto,
}

impl Engine {
    pub fn load(config: &AppConfig) -> anyhow::Result<Self> {
        Ok(Self {
            dict: Dictionary::from_embedded()?,
            index: SignatureIndex::from_embedded()?,
            theme: theme::theme_dto(config),
        })
    }
}

/// Mutable session. Guarded by `AppState.session`.
pub struct Session {
    pub state: GameState,
    pub rng: StdRng,
    /// Keyed by the internal solution string (never serialized).
    pub affix_cache: Option<(String, AffixHints)>,
    /// Number of `affix_hints` computations (test builds only).
    #[cfg(test)]
    pub computations: usize,
}

impl Session {
    pub fn new(engine: &Engine, config: &AppConfig) -> anyhow::Result<Self> {
        Self::build(engine, config, StdRng::from_entropy())
    }

    pub fn new_seeded(engine: &Engine, config: &AppConfig, seed: u64) -> anyhow::Result<Self> {
        Self::build(engine, config, StdRng::seed_from_u64(seed))
    }

    fn build(engine: &Engine, config: &AppConfig, rng: StdRng) -> anyhow::Result<Self> {
        let lo = config.min_size.clamp(SIZE_MIN, SIZE_MAX);
        let hi = config.max_size.clamp(SIZE_MIN, SIZE_MAX);
        if lo > hi {
            bail!("effective puzzle size range {lo}..={hi} is empty (check min_size/max_size)");
        }
        ensure_words_for_range(&engine.dict, lo, hi)?;

        let mut state = GameState::with_bounds(5u8.clamp(lo, hi), lo, hi);
        state.workbench_settings = config.workbench_settings();
        let mut session = Self {
            state,
            rng,
            affix_cache: None,
            #[cfg(test)]
            computations: 0,
        };
        session.apply_actions(engine, vec![Action::NewPuzzle]);
        if session.state.phase != Phase::Playing {
            bail!("could not start initial puzzle: {:?}", session.state.status);
        }
        Ok(session)
    }

    /// Sole mutation entry point. Applies the action, then returns the snapshot.
    pub fn dispatch(&mut self, engine: &Engine, action: ActionDto) -> Result<UiState, String> {
        self.apply(engine, action)?;
        Ok(self.snapshot(engine))
    }

    pub fn snapshot(&mut self, engine: &Engine) -> UiState {
        self.ensure_affix_cache();
        let affix = if affix_visible(&self.state) {
            self.affix_cache.as_ref().map(|(_, hints)| hints)
        } else {
            None
        };
        dto::ui_state(&self.state, affix, &engine.theme)
    }

    /// Convert and apply one UI action under the caller's lock. Builds no snapshot.
    fn apply(&mut self, engine: &Engine, action: ActionDto) -> Result<(), String> {
        let action = self.clamp_action(action);
        let actions = match action {
            // D-9: cancel is a no-op outside the custom phases.
            ActionDto::CancelCustomWord if !in_custom_phase(self.state.phase) => Vec::new(),
            other => other.into_actions().map_err(|e| match e {
                DtoError::InvalidLetter(ch) => format!("invalid letter: {ch}"),
            })?,
        };
        self.apply_actions(engine, actions);
        match self.state.phase {
            Phase::Menu | Phase::SizeSelect => Err("internal: unexpected phase".into()),
            _ => Ok(()),
        }
    }

    /// Apply core actions in order, then refresh the affix cache.
    fn apply_actions(&mut self, engine: &Engine, actions: Vec<Action>) {
        for action in actions {
            let state = std::mem::replace(&mut self.state, GameState::new(SIZE_MIN));
            self.state = app::reduce(state, action, &engine.dict, &engine.index, &mut self.rng);
        }
        self.ensure_affix_cache();
    }

    /// Clamp `selectSize` to the effective bounds (D-4).
    fn clamp_action(&self, action: ActionDto) -> ActionDto {
        match action {
            ActionDto::SelectSize { n } => ActionDto::SelectSize {
                n: n.clamp(self.state.min_word_len, self.state.max_word_len),
            },
            other => other,
        }
    }

    /// Recompute affixes only when the visible round's solution changed (§4.3).
    fn ensure_affix_cache(&mut self) {
        if !affix_visible(&self.state) {
            return;
        }
        let Some(round) = self.state.round.as_ref() else {
            return;
        };
        let fresh = matches!(&self.affix_cache, Some((key, _)) if key == &round.solution);
        if fresh {
            return;
        }
        let hints = affix_hints(round);
        #[cfg(test)]
        {
            self.computations += 1;
        }
        self.affix_cache = Some((round.solution.clone(), hints));
    }
}

/// Affix hints are visible only while a round is shown with the workbench on (§5.5).
fn affix_visible(state: &GameState) -> bool {
    state.workbench_settings.enabled
        && state.round.is_some()
        && matches!(state.phase, Phase::Playing | Phase::Help | Phase::Won)
}

fn in_custom_phase(phase: Phase) -> bool {
    matches!(phase, Phase::CustomWordInput | Phase::CustomWordPick)
}

/// Every length in `lo..=hi` must have at least one dictionary word (D-5).
pub fn ensure_words_for_range(dict: &Dictionary, lo: u8, hi: u8) -> anyhow::Result<()> {
    for n in lo..=hi {
        if dict.words_with_length(n as usize).is_empty() {
            bail!("no dictionary words of length {n}");
        }
    }
    Ok(())
}

/// Tauri managed state.
pub struct AppState {
    pub engine: Arc<Engine>,
    pub session: Mutex<Session>,
}

impl AppState {
    /// Lock the session for one command. Poisoned lock → `Err("internal error")`.
    pub fn lock_session(&self) -> Result<MutexGuard<'_, Session>, String> {
        lock_session(&self.session)
    }
}

fn lock_session(session: &Mutex<Session>) -> Result<MutexGuard<'_, Session>, String> {
    session.lock().map_err(|_| "internal error".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::PhaseDto;
    use std::sync::OnceLock;

    fn test_engine() -> &'static Engine {
        static ENGINE: OnceLock<Engine> = OnceLock::new();
        ENGINE.get_or_init(|| Engine {
            dict: Dictionary::from_embedded().expect("embedded dictionary"),
            index: SignatureIndex::from_embedded().expect("embedded index"),
            theme: ThemeDto {
                empty_slot: "#ff00ff".into(),
                filled_slot: "#ffff00".into(),
                pool: "#00ffff".into(),
                correct_position: "#00ff00".into(),
            },
        })
    }

    fn session_with(config: &AppConfig, seed: u64) -> Session {
        Session::new_seeded(test_engine(), config, seed).expect("session")
    }

    fn workbench_off() -> AppConfig {
        AppConfig {
            workbench: false,
            ..AppConfig::default()
        }
    }

    #[test]
    fn reducer_equivalence_fixed_seed() {
        let config = AppConfig::default();
        let engine = test_engine();
        let mut session = session_with(&config, 4242);

        let mut rng = StdRng::seed_from_u64(4242);
        let mut direct = GameState::with_bounds(5, SIZE_MIN, SIZE_MAX);
        direct.workbench_settings = config.workbench_settings();
        direct = app::reduce(direct, Action::NewPuzzle, &engine.dict, &engine.index, &mut rng);

        let actions = vec![
            Action::TypeLetter('a'),
            Action::CursorRight,
            Action::Backspace,
            Action::RequestClue,
            Action::ToggleHelp,
            Action::ToggleHelp,
            Action::ResetRound,
            Action::OpenCustomWord,
            Action::CustomWordType('p'),
            Action::CustomWordType('l'),
            Action::CancelCustomWord,
            Action::NewPuzzle,
            Action::RequestClue,
            Action::TypeLetter('z'),
        ];
        for action in actions.clone() {
            direct = app::reduce(direct, action, &engine.dict, &engine.index, &mut rng);
        }
        session.apply_actions(engine, actions);

        assert_eq!(session.state.phase, direct.phase);
        assert_eq!(session.state.round, direct.round);
        assert!(session.state.round.is_some());
    }

    #[test]
    fn cancel_custom_runs_pair() {
        let engine = test_engine();
        let mut session = session_with(&AppConfig::default(), 7);
        let size_before = session.state.size;
        session.apply_actions(
            engine,
            vec![Action::OpenCustomWord, Action::CustomWordType('p')],
        );
        assert_eq!(session.state.phase, Phase::CustomWordInput);

        // The pair `into_actions` emits for `cancelCustomWord` (D-9).
        session.apply_actions(engine, vec![Action::CancelCustomWord, Action::NewPuzzle]);
        assert_eq!(session.state.phase, Phase::Playing);
        assert!(session.state.round.is_some());
        assert_eq!(session.state.size, size_before);
    }

    #[test]
    fn cancel_outside_custom_is_noop() {
        let engine = test_engine();
        let mut session = session_with(&AppConfig::default(), 8);
        let round_before = session.state.round.clone();
        assert_eq!(session.state.phase, Phase::Playing);

        session
            .apply(engine, ActionDto::CancelCustomWord)
            .expect("no-op cancel");
        assert_eq!(session.state.phase, Phase::Playing);
        assert_eq!(session.state.round, round_before);
    }

    #[test]
    fn select_size_clamps_to_config_bounds() {
        let config = AppConfig {
            min_size: 5,
            max_size: 8,
            ..AppConfig::default()
        };
        let session = session_with(&config, 9);
        assert_eq!(session.state.min_word_len, 5);
        assert_eq!(session.state.max_word_len, 8);

        match session.clamp_action(ActionDto::SelectSize { n: 2 }) {
            ActionDto::SelectSize { n } => assert_eq!(n, 5),
            other => panic!("unexpected action {other:?}"),
        }
        match session.clamp_action(ActionDto::SelectSize { n: 20 }) {
            ActionDto::SelectSize { n } => assert_eq!(n, 8),
            other => panic!("unexpected action {other:?}"),
        }
    }

    #[test]
    fn affix_cache_reuse() {
        let engine = test_engine();
        let mut session = session_with(&AppConfig::default(), 11);
        assert_eq!(session.computations, 1, "first round warms the cache");

        session.apply_actions(engine, vec![Action::RequestClue]);
        assert_eq!(session.computations, 1, "same round reuses the cache");

        let solution_before = session.state.round.as_ref().unwrap().solution.clone();
        session.apply_actions(engine, vec![Action::NewPuzzle]);
        let solution_after = session.state.round.as_ref().unwrap().solution.clone();
        assert_ne!(solution_before, solution_after);
        assert_eq!(session.computations, 2, "new round recomputes");
        assert_eq!(
            session.affix_cache.as_ref().map(|(k, _)| k.as_str()),
            Some(solution_after.as_str())
        );
    }

    #[test]
    fn affix_hidden_in_custom_input() {
        let engine = test_engine();
        let mut session = session_with(&AppConfig::default(), 12);
        assert!(affix_visible(&session.state));
        session.apply_actions(engine, vec![Action::OpenCustomWord]);
        assert_eq!(session.state.phase, Phase::CustomWordInput);
        assert!(!affix_visible(&session.state));
    }

    #[test]
    fn workbench_disabled_no_affixes() {
        let engine = test_engine();
        let mut session = session_with(&workbench_off(), 13);
        assert!(session.state.workbench.is_none());
        assert_eq!(session.computations, 0);
        assert!(session.affix_cache.is_none());

        session.apply_actions(engine, vec![Action::NewPuzzle, Action::RequestClue]);
        assert_eq!(session.computations, 0);
        assert!(session.affix_cache.is_none());
    }

    #[test]
    fn startup_rejects_length_without_words() {
        let dict = &test_engine().dict;
        assert!(ensure_words_for_range(dict, 40, 40).is_err());
        assert!(ensure_words_for_range(dict, SIZE_MIN, SIZE_MAX).is_ok());
    }

    #[test]
    fn startup_rejects_empty_size_range() {
        let config = AppConfig {
            min_size: 8,
            max_size: 6,
            ..AppConfig::default()
        };
        assert!(Session::new_seeded(test_engine(), &config, 14).is_err());
    }

    #[test]
    fn startup_length_clamped_to_effective_range() {
        let config = AppConfig {
            min_size: 6,
            max_size: 8,
            ..AppConfig::default()
        };
        let session = session_with(&config, 15);
        assert_eq!(session.state.size, 6);
        assert_eq!(session.state.round.as_ref().unwrap().solution.len(), 6);

        let wide = AppConfig {
            min_size: 2,
            max_size: 12,
            ..AppConfig::default()
        };
        let session = session_with(&wide, 16);
        assert_eq!(session.state.min_word_len, SIZE_MIN);
        assert_eq!(session.state.max_word_len, SIZE_MAX);
    }

    #[test]
    fn poisoned_lock_maps_to_internal_error() {
        let session = Mutex::new(session_with(&AppConfig::default(), 17));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = session.lock().unwrap();
            panic!("poison the session lock for this test");
        }));
        assert_eq!(
            lock_session(&session).err().as_deref(),
            Some("internal error")
        );
    }

    /// Needs WP-2 (`dto::ui_state`, `ActionDto::into_actions`); currently `todo!()`.
    #[test]
    #[ignore = "WP-2 dependency: dto::ui_state and into_actions are todo!()"]
    fn wp2_dependent_dispatch_returns_snapshot() {
        let engine = test_engine();
        let mut session = session_with(&AppConfig::default(), 18);
        let snap = session
            .dispatch(engine, ActionDto::RequestClue)
            .expect("dispatch");
        assert_eq!(snap.phase, PhaseDto::Playing);
        assert!(snap.round.is_some());
    }

    /// Needs WP-2 (`into_actions` pair mapping) and the snapshot; currently `todo!()`.
    #[test]
    #[ignore = "WP-2 dependency: dto::ui_state and into_actions are todo!()"]
    fn wp2_dependent_dispatch_cancel_custom_pair() {
        let engine = test_engine();
        let mut session = session_with(&AppConfig::default(), 19);
        session.apply_actions(engine, vec![Action::OpenCustomWord]);
        let snap = session
            .dispatch(engine, ActionDto::CancelCustomWord)
            .expect("dispatch");
        assert_eq!(snap.phase, PhaseDto::Playing);
        assert!(snap.round.is_some());
    }

    /// Needs WP-5 (`theme::theme_dto`); currently `todo!()`.
    #[test]
    #[ignore = "WP-5 dependency: theme::theme_dto is todo!()"]
    fn wp5_dependent_engine_load_builds_theme() {
        let engine = Engine::load(&AppConfig::default()).expect("engine");
        assert_eq!(engine.theme.pool, "#00ffff");
    }
}
