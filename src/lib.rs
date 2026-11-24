//! Anagram TUI library.
//!
//! `run` — application entry (CLI config, event loop).
//! Core modules: `game`, `dictionary`, `app`, `ui`.

pub mod affix_hints;
pub mod app;
pub mod cli;
pub mod config;
pub mod dictionary;
pub mod game;
pub mod input;
pub mod solve;
pub mod terminal;
pub mod signature_index;
pub mod ui;
pub mod workbench;

pub use signature_index::{LookupResult, Signature, SignatureError, SignatureIndex};

use app::{validate_custom_word, Action, GameState};
use cli::Cli;
use config::AppConfig;
use crossterm::event::Event;
use dictionary::Dictionary;
use ui::UiTheme;
use rand::rngs::StdRng;
use rand::SeedableRng;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Write};

/// Application entry from the binary. Returns process exit code (`0` on success).
pub fn run(cli: Cli) -> anyhow::Result<i32> {
    if let Some(scramble) = cli.solve_scramble() {
        return solve::run_solve(scramble);
    }
    run_play(cli)?;
    Ok(0)
}

fn run_play(cli: Cli) -> anyhow::Result<()> {
    let config = AppConfig::load(cli.config.as_deref())?;
    if cli.plain {
        return Ok(());
    }

    let dict = Dictionary::from_embedded()?;
    let index = SignatureIndex::from_embedded()?;
    let _guard = terminal::TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut rng = StdRng::from_entropy();
    let default_size = config.clamp_size(5);
    let mut state = GameState::with_bounds(default_size, config.min_size, config.max_size);
    state.workbench_settings = config.workbench_settings();
    let theme = UiTheme::from_config(&config);

    if let Some(word) = cli.play_word() {
        validate_custom_word(word, state.min_word_len, state.max_word_len)
            .map_err(anyhow::Error::msg)?;
        state = app::start_custom_word(state, word, &dict, &index, &mut rng);
        if state.phase != app::Phase::Playing {
            anyhow::bail!("could not start puzzle for that word");
        }
    }

    loop {
        terminal.draw(|frame| ui::draw(frame, &state, &theme))?;

        let Some(event) = terminal::poll_event()? else {
            continue;
        };

        if matches!(event, Event::Resize(_, _)) {
            continue;
        }

        let ctx = input::EventContext::from_state(&state);
        let Some(action) = input::map_event(&event, &ctx) else {
            continue;
        };

        if matches!(action, Action::Quit) {
            break;
        }

        state = app::reduce(state, action, &dict, &index, &mut rng);
    }

    stdout().flush()?;
    Ok(())
}