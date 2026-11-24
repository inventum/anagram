//! Tauri builder only: setup, managed state, commands. See SPEC-ui §4.3.

use std::sync::{Arc, Mutex};

use anagram::config::AppConfig;
use anagram_desktop::dto::{ActionDto, UiState};
use anagram_desktop::session::{AppState, Engine, Session};
use tauri::{App, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

/// Startup failure: native error dialog, then exit (SPEC-ui §4.3).
fn fatal(app: &App, err: anyhow::Error) -> ! {
    app.dialog()
        .message(format!("{err:#}"))
        .title("Anagram cannot start")
        .kind(MessageDialogKind::Error)
        .blocking_show();
    std::process::exit(1)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = AppConfig::load(None).unwrap_or_else(|e| fatal(app, e));
            let engine = Engine::load(&config).unwrap_or_else(|e| fatal(app, e));
            let session = Session::new(&engine, &config).unwrap_or_else(|e| fatal(app, e));
            app.manage(AppState {
                engine: Arc::new(engine),
                session: Mutex::new(session),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![dispatch, get_state])
        .run(tauri::generate_context!())
        .expect("tauri run");
}

#[tauri::command]
fn dispatch(state: tauri::State<'_, AppState>, action: ActionDto) -> Result<UiState, String> {
    let mut session = state.lock_session()?;
    session.dispatch(&state.engine, action)
}

#[tauri::command]
fn get_state(state: tauri::State<'_, AppState>) -> Result<UiState, String> {
    let mut session = state.lock_session()?;
    Ok(session.snapshot(&state.engine))
}
