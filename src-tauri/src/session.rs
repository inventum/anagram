//! Engine (read-only data) and Session (mutable game state). See SPEC-ui §4.3.

use std::sync::{Arc, Mutex};

use anagram::affix_hints::AffixHints;
use anagram::app::GameState;
use anagram::config::AppConfig;
use anagram::dictionary::Dictionary;
use anagram::signature_index::SignatureIndex;
use rand::rngs::StdRng;

use crate::dto::{ActionDto, ThemeDto, UiState};

/// Read-only game data. Shared via `Arc`, no lock.
pub struct Engine {
    pub dict: Dictionary,
    pub index: SignatureIndex,
    pub theme: ThemeDto,
}

impl Engine {
    pub fn load(config: &AppConfig) -> anyhow::Result<Self> {
        todo!()
    }
}

/// Mutable session. Guarded by `AppState.session`.
pub struct Session {
    pub state: GameState,
    pub rng: StdRng,
    /// Keyed by the internal solution string (never serialized).
    pub affix_cache: Option<(String, AffixHints)>,
}

impl Session {
    pub fn new(engine: &Engine, config: &AppConfig) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn new_seeded(engine: &Engine, config: &AppConfig, seed: u64) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn dispatch(&mut self, engine: &Engine, action: ActionDto) -> Result<UiState, String> {
        todo!()
    }

    pub fn snapshot(&mut self, engine: &Engine) -> UiState {
        todo!()
    }
}

/// Tauri managed state.
pub struct AppState {
    pub engine: Arc<Engine>,
    pub session: Mutex<Session>,
}
