//! IPC leak sweep and custom-phase invariants (SPEC-ui §5.1, §5.5, §10.3).
//!
//! Every call goes through `AppState::lock_session` and `Session::dispatch`, the same
//! path as the Tauri commands, and each snapshot is inspected as serialized JSON.

use std::sync::{Arc, Mutex, OnceLock};

use anagram::config::AppConfig;
use anagram_desktop::dto::{ActionDto, PhaseDto};
use anagram_desktop::session::{AppState, Engine, Session};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};

const SEEDS: u64 = 20;
const STEPS_PER_SEED: usize = 30;
const CUSTOM_STEPS_PER_SEED: usize = 40;
const MIN_DISPATCHES: usize = 300;

/// Outcome of one dispatch, read under the same lock as the mutation.
struct Step {
    snapshot: Value,
    phase: PhaseDto,
    /// Internal solution of the current round (never serialized).
    solution: Option<String>,
}

fn engine() -> Arc<Engine> {
    static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();
    ENGINE
        .get_or_init(|| Arc::new(Engine::load(&AppConfig::default()).expect("engine")))
        .clone()
}

fn app_state(seed: u64) -> AppState {
    let engine = engine();
    let session = Session::new_seeded(&engine, &AppConfig::default(), seed).expect("session");
    AppState {
        engine,
        session: Mutex::new(session),
    }
}

/// Deserialize a wire action, dispatch it under the session lock, and capture the result.
fn dispatch(app: &AppState, action: &Value) -> Step {
    let action: ActionDto = serde_json::from_value(action.clone()).expect("wire action");
    let mut session = app.lock_session().expect("lock");
    let snap = session.dispatch(&app.engine, action).expect("dispatch");
    let solution = session.state.round.as_ref().map(|r| r.solution.clone());
    Step {
        phase: snap.phase,
        snapshot: serde_json::to_value(&snap).expect("serialize"),
        solution,
    }
}

/// Case-insensitive whole-word occurrence of `solution` in `text`.
fn whole_word_hit(text: &str, solution: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .any(|tok| tok.eq_ignore_ascii_case(solution))
}

/// Walk every key and string in the snapshot.
///
/// - Any `solution` key fails.
/// - When `check_strings` is set, no string value may equal the solution. `custom.input`
///   is skipped: it echoes the user's own keystrokes, not game data.
/// - When `check_strings` is set, whole-word solution hits in `activeClue` or
///   `definition` are recorded as findings (WordNet glosses can contain the answer).
fn leak_check(
    v: &Value,
    key: &str,
    solution: Option<&str>,
    check_strings: bool,
    ctx: &str,
    findings: &mut Vec<String>,
) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                assert_ne!(k, "solution", "{ctx}: `solution` key present");
                leak_check(child, k, solution, check_strings, ctx, findings);
            }
        }
        Value::Array(items) => {
            for child in items {
                leak_check(child, key, solution, check_strings, ctx, findings);
            }
        }
        Value::String(s) if check_strings => {
            let Some(sol) = solution else { return };
            if key != "input" {
                assert!(
                    !s.eq_ignore_ascii_case(sol),
                    "{ctx}: string under `{key}` equals the solution"
                );
            }
            if matches!(key, "activeClue" | "definition") && whole_word_hit(s, sol) {
                findings.push(format!("{ctx}: `{key}` = {s:?} contains {sol:?}"));
            }
        }
        _ => {}
    }
}

fn random_letter(rng: &mut StdRng, solution: Option<&str>) -> char {
    // Half the time type a letter from the answer so that `won` is reachable.
    if let Some(s) = solution.filter(|s| !s.is_empty()) {
        if rng.gen_bool(0.5) {
            return s.as_bytes()[rng.gen_range(0..s.len())] as char;
        }
    }
    (b'a' + rng.gen_range(0..26u8)) as char
}

/// Random legal play-phase action (spec §10.3 list: typeLetter, backspace, arrows, clue, help, reset).
fn play_action(rng: &mut StdRng, solution: Option<&str>) -> Value {
    match rng.gen_range(0..12) {
        0..=4 => json!({"type": "typeLetter", "ch": random_letter(rng, solution)}),
        5 => json!({"type": "backspace"}),
        6 => json!({"type": "cursorLeft"}),
        7 => json!({"type": "cursorRight"}),
        8 => json!({"type": "requestClue"}),
        9 => json!({"type": "toggleHelp"}),
        _ => json!({"type": "resetRound"}),
    }
}

/// Random custom-word and pick action. Typed letters are drawn from the unused letters of
/// "listen", so a full 6-letter entry is a permutation of an anagram family and submits
/// frequently hit the multi-match pick list. `input` is the current `custom.input`.
fn custom_action(rng: &mut StdRng, phase: PhaseDto, input: &str) -> Value {
    const FAMILY: &[u8] = b"listen";
    let unused: Vec<u8> = FAMILY
        .iter()
        .copied()
        .filter(|c| !input.as_bytes().contains(c))
        .collect();
    // Bias the walk so that it reliably moves through input → submit → pick.
    match phase {
        PhaseDto::CustomWordInput if input.len() == FAMILY.len() && rng.gen_bool(0.7) => {
            return json!({"type": "submitCustomWord"});
        }
        PhaseDto::CustomWordInput if !unused.is_empty() && rng.gen_bool(0.7) => {
            let ch = unused[rng.gen_range(0..unused.len())] as char;
            return json!({"type": "customWordType", "ch": ch});
        }
        PhaseDto::CustomWordPick if rng.gen_bool(0.6) => {
            return json!({"type": "customPickDelta", "delta": if rng.gen_bool(0.5) { 1 } else { -1 }});
        }
        PhaseDto::Playing | PhaseDto::Help | PhaseDto::Won if rng.gen_bool(0.6) => {
            return json!({"type": "openCustomWord"});
        }
        _ => {}
    }
    match rng.gen_range(0..14) {
        0..=1 => json!({"type": "openCustomWord"}),
        2..=5 => {
            let pool = if unused.is_empty() {
                FAMILY.to_vec()
            } else {
                unused
            };
            let ch = pool[rng.gen_range(0..pool.len())] as char;
            json!({"type": "customWordType", "ch": ch})
        }
        6 => json!({"type": "customWordBackspace"}),
        7 | 8 => json!({"type": "submitCustomWord"}),
        9 => json!({"type": "customPickDelta", "delta": if rng.gen_bool(0.5) { 1 } else { -1 }}),
        10 => json!({"type": "customPickSelect", "idx": rng.gen_range(0..3usize)}),
        11 => json!({"type": "confirmCustomPick"}),
        12 => json!({"type": "cancelCustomWord"}),
        _ => json!({"type": "newPuzzle"}),
    }
}

/// Tally for the sweep. `leak_check` runs only where the spec requires it.
#[derive(Default)]
struct Sweep {
    total: usize,
    checked: usize,
    skipped: usize,
    won_seen: usize,
    phases_seen: Vec<PhaseDto>,
    findings: Vec<String>,
}

impl Sweep {
    fn observe(&mut self, app: &AppState, action: &Value, ctx: &str) -> Step {
        let step = dispatch(app, action);
        self.total += 1;
        if !self.phases_seen.contains(&step.phase) {
            self.phases_seen.push(step.phase);
        }
        match (step.phase, step.solution.as_deref()) {
            (PhaseDto::Won, Some(sol)) => {
                // Positive control: the win phase must expose the solution (§10.1).
                self.won_seen += 1;
                assert_eq!(
                    step.snapshot["round"]["solution"], sol,
                    "{ctx}: won without solution"
                );
                self.skipped += 1;
            }
            (PhaseDto::CustomWordPick, _) => self.skipped += 1,
            (_, Some(sol)) => {
                self.checked += 1;
                leak_check(&step.snapshot, "", Some(sol), true, ctx, &mut self.findings);
            }
            (_, None) => {}
        }
        step
    }
}

#[test]
fn ipc_leak_sweep() {
    let mut sweep = Sweep::default();

    for seed in 0..SEEDS {
        let app = app_state(seed);
        let mut rng = StdRng::seed_from_u64(seed ^ 0x5EED_1EAF);

        let mut step = sweep.observe(
            &app,
            &json!({"type": "newPuzzle"}),
            &format!("seed {seed} new"),
        );
        for i in 0..STEPS_PER_SEED {
            let action = play_action(&mut rng, step.solution.as_deref());
            step = sweep.observe(&app, &action, &format!("seed {seed} step {i}"));
        }

        // Even seeds: reset the round, then type the solution in order to reach `won`.
        if seed % 2 == 0 {
            if step.phase == PhaseDto::Help {
                sweep.observe(
                    &app,
                    &json!({"type": "toggleHelp"}),
                    &format!("seed {seed} help-off"),
                );
            }
            let reset = sweep.observe(
                &app,
                &json!({"type": "resetRound"}),
                &format!("seed {seed} reset"),
            );
            let sol = reset.solution.clone().expect("round after reset");
            for (i, ch) in sol.to_ascii_lowercase().chars().enumerate() {
                let ctx = format!("seed {seed} solve {i}");
                sweep.observe(&app, &json!({"type": "typeLetter", "ch": ch}), &ctx);
            }
        }
    }

    for f in &sweep.findings {
        println!("⚠️ clue-hit finding (not a failure): {f}");
    }
    println!(
        "ipc_leak_sweep: {} dispatches over {SEEDS} seeds; {} snapshots leak-checked, {} skipped \
         (won/pick); won observed {}×; phases seen {:?}; ⚠️ {} clue-hit findings",
        sweep.total,
        sweep.checked,
        sweep.skipped,
        sweep.won_seen,
        sweep.phases_seen,
        sweep.findings.len()
    );
    assert!(
        sweep.total >= MIN_DISPATCHES,
        "only {} dispatches (need ≥ {MIN_DISPATCHES})",
        sweep.total
    );
    assert!(sweep.checked > 0, "no snapshots were leak-checked");
    assert!(
        sweep.won_seen > 0,
        "won phase never reached; exclusion untested"
    );
}

#[test]
fn custom_phase_invariants() {
    let mut total = 0usize;
    let mut saw_input = 0usize;
    let mut saw_pick = 0usize;
    let mut findings: Vec<String> = Vec::new();

    for seed in 0..SEEDS {
        let app = app_state(seed);
        let mut rng = StdRng::seed_from_u64(seed ^ 0xC0FF_EE00);
        let mut input = String::new();
        let mut phase = PhaseDto::Playing;

        for i in 0..CUSTOM_STEPS_PER_SEED {
            let ctx = format!("custom seed {seed} step {i}");
            let action = custom_action(&mut rng, phase, &input);
            let step = dispatch(&app, &action);
            total += 1;
            let snap = &step.snapshot;
            input = snap["custom"]["input"].as_str().unwrap_or("").to_string();
            phase = step.phase;
            let custom = &snap["custom"];
            let choices = custom["choices"].as_array().expect("choices array");

            match step.phase {
                PhaseDto::CustomWordInput | PhaseDto::CustomWordPick => {
                    assert!(
                        snap.get("round").is_none(),
                        "{ctx}: round present in custom phase"
                    );
                    assert!(
                        snap.get("workbench").is_none(),
                        "{ctx}: workbench present in custom phase"
                    );
                    assert!(
                        snap["activeClue"].is_null(),
                        "{ctx}: activeClue present in custom phase"
                    );
                }
                _ => {}
            }

            if step.phase == PhaseDto::CustomWordPick {
                saw_pick += 1;
                assert!(!choices.is_empty(), "{ctx}: pick phase with no choices");
            } else {
                assert!(
                    choices.is_empty(),
                    "{ctx}: choices non-empty in {:?}",
                    step.phase
                );
            }
            if step.phase == PhaseDto::CustomWordInput {
                saw_input += 1;
            }

            // `won` exposes the solution by design (§5.5). Pick candidates include it too,
            // so only the key check applies there.
            match step.phase {
                PhaseDto::Won => {}
                PhaseDto::CustomWordPick => {
                    leak_check(snap, "", None, false, &ctx, &mut findings);
                }
                _ => {
                    leak_check(
                        snap,
                        "",
                        step.solution.as_deref(),
                        true,
                        &ctx,
                        &mut findings,
                    );
                }
            }
        }
    }

    for f in &findings {
        println!("⚠️ clue-hit finding (not a failure): {f}");
    }
    println!(
        "custom_phase_invariants: {total} dispatches; customWordInput seen {saw_input}×, customWordPick seen {saw_pick}×; ⚠️ {} clue-hit findings",
        findings.len()
    );
    assert!(saw_input > 0, "custom input phase never reached");
    assert!(saw_pick > 0, "custom pick phase never reached");
}
