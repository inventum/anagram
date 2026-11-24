//! Config loading through the desktop session (SPEC-ui §6, §10.1 "Config").
//!
//! Files live under `std::env::temp_dir()` with unique names. `~/.config` is never read.

use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use anagram::config::{AppConfig, WorkbenchSettings};
use anagram_desktop::session::{Engine, Session};
use anagram_desktop::theme;

/// Unique scratch directory, removed on drop (also on panic).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "anagram-desktop-wp11-{tag}-{}-{nanos}",
            process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_file_yields_defaults() {
    let dir = TempDir::new("missing");
    let path = dir.file("config.toml");
    assert!(!path.exists());

    let cfg = AppConfig::load(Some(&path)).expect("missing file is not an error");
    assert_eq!(cfg, AppConfig::default());
    assert_eq!(
        cfg.workbench_settings(),
        WorkbenchSettings {
            enabled: true,
            rows: 5,
            cols: 5,
        }
    );
}

#[test]
fn malformed_file_is_load_error() {
    let dir = TempDir::new("malformed");
    let path = dir.file("config.toml");
    std::fs::write(&path, "min_size = = 5\n[unterminated\n").expect("write fixture");

    assert!(AppConfig::load(Some(&path)).is_err());
}

#[test]
fn invalid_workbench_dims_become_5x5() {
    let dir = TempDir::new("dims");

    let bad = dir.file("bad.toml");
    std::fs::write(&bad, "workbench_rows = 3\nworkbench_cols = 5\n").expect("write fixture");
    let cfg = AppConfig::load(Some(&bad)).expect("load");
    assert_eq!(cfg.workbench_rows, 3);
    assert_eq!(cfg.workbench_cols, 5);
    assert_eq!(
        cfg.workbench_settings(),
        WorkbenchSettings {
            enabled: true,
            rows: 5,
            cols: 5,
        }
    );

    // The session must carry the same resolved dimensions into its game state.
    let engine = Engine::load(&cfg).expect("engine");
    let session = Session::new_seeded(&engine, &cfg, 1).expect("session");
    assert_eq!(session.state.workbench_settings, cfg.workbench_settings());
    assert_eq!(session.state.workbench_settings.rows, 5);
    assert_eq!(session.state.workbench_settings.cols, 5);

    // Valid 4×4 is kept, so the fallback is not unconditional.
    let ok = dir.file("ok.toml");
    std::fs::write(&ok, "workbench_rows = 4\nworkbench_cols = 4\n").expect("write fixture");
    let cfg = AppConfig::load(Some(&ok)).expect("load");
    assert_eq!(
        cfg.workbench_settings(),
        WorkbenchSettings {
            enabled: true,
            rows: 4,
            cols: 4,
        }
    );
}

#[test]
fn configured_colors_reach_snapshot_via_theme() {
    let cfg = AppConfig {
        pool_color: "Light_Red".into(),
        ..AppConfig::default()
    };
    let engine = Engine::load(&cfg).expect("engine");
    let mut session = Session::new_seeded(&engine, &cfg, 2).expect("session");
    let snap = serde_json::to_value(session.snapshot(&engine)).expect("serialize");
    assert_eq!(snap["theme"]["pool"], "#ff0000");
    assert_eq!(
        snap["theme"],
        serde_json::to_value(theme::theme_dto(&cfg)).expect("serialize")
    );
}
