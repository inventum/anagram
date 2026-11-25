# Architecture

Status: describes branch `tauri-ui` as of 2026-11-25.
Specs: [SPEC.md](SPEC.md), [SPEC-ui.md](SPEC-ui.md), [SPEC_SIGNATURE_INDEX.md](SPEC_SIGNATURE_INDEX.md), [SPEC_LETTER_WORKBENCH.md](SPEC_LETTER_WORKBENCH.md).
Build and run: [INSTALL.md](../INSTALL.md). Agent rules: [AGENTS.md](../AGENTS.md).

## 1. Overview

Two front ends share one Rust core:

| Front end | Location | Stack | Entry point |
|-----------|----------|-------|-------------|
| Terminal TUI | `src/` (crate `anagram`) | ratatui + crossterm | `src/main.rs` → `anagram::run` |
| Desktop app | `src-tauri/` (crate `anagram-desktop`) + `ui-web/` | Tauri 2, TypeScript, Vite | `src-tauri/src/main.rs` |

Both use the reducer `app::reduce`, the same embedded data, and the same config file. The desktop crate does not
re-implement rules: it converts IPC DTOs to `app::Action`, calls `reduce`, and serializes a view. The TypeScript
frontend renders that view and sends actions back.

## 2. Components and module map

### 2.1 Core crate (`src/`)

| Module | Responsibility | Key public items |
|--------|----------------|------------------|
| `lib.rs` | `run(cli)`: `solve` or the TUI loop | `run` |
| `app.rs` | Phases, actions, reducer, custom-word flow, clues | `GameState`, `Phase`, `Action`, `reduce`, `validate_custom_word` |
| `game.rs` | One round: pool, answer slots, cursor, win check | `Round`, `SolveResult`, `multiset_matches` |
| `dictionary.rs` | Embedded word list, length filter, random pick, clues | `Dictionary`, `WordEntry`, `ClueKind` |
| `signature_index.rs` | Sorted-letter buckets for anagram lookup | `SignatureIndex`, `LookupResult`, `signature_key` |
| `workbench.rs` | Letter grid (cue only; no effect on win) | `LetterWorkbench` |
| `affix_hints.rs` | Prefix and suffix hints | `AffixHints`, `affix_hints` |
| `solve.rs`, `cli.rs` | `solve` subcommand; clap arguments | `run_solve` (exit 0, 1, 2) |
| `config.rs` | TOML config, defaults, workbench dimension rule | `AppConfig`, `default_config_path` |
| `input.rs`, `ui.rs`, `ui/theme.rs`, `terminal.rs` | TUI only: keys, rendering, colors, raw mode | `map_event`, `draw`, `parse_color_name` |

Core types do not derive `Serialize` (`rg Serialize src/` is empty).

### 2.2 Desktop crate (`src-tauri/`)

| Module | Responsibility |
|--------|----------------|
| `dto.rs` | IPC boundary: `UiState` (output), `ActionDto` (input), visibility rules, `ui_state`, `into_actions` |
| `session.rs` | Read-only `Engine`; mutable `Session` behind one mutex; affix cache; `dispatch` |
| `theme.rs` | Config color names → CSS colors (`theme_dto`, `color_to_css`) |
| `main.rs` | Tauri builder, startup error dialog, commands `dispatch` and `get_state` |
| `lib.rs` | `pub mod dto; session; theme;` for `tests/` |

### 2.3 Frontend (`ui-web/src/`)

| Path | Responsibility |
|------|----------------|
| `main.ts` | Boot: `getState` → `applyTheme` → `renderApp` → `bindInput` → `applyA11y`; re-render after each result |
| `ipc.ts` | `invoke` wrappers, FIFO queue (D-15), dev-only `?fixture=` path |
| `types.ts`, `state.ts` | DTO mirror; `RenderCtx`, `LocalState` (`pending`, `sliderDraft`) |
| `render/`, `overlays/`, `workbench/` | Play window; confirm and custom dialogs and help (`HELP_ROWS`); pointer-only workbench |
| `input/keymap.ts`, `input/index.ts` | Pure `keyToAction`; one `window` keydown listener |
| `theme-apply.ts`, `a11y.ts`, `fixtures.ts` | CSS variables; ARIA and focus trap; dev snapshots |

### 2.4 Dependency diagram

```
 TUI binary                                  Desktop (Tauri)
 main.rs ─► lib.rs::run ─┬─► input.rs        src-tauri/src/main.rs
                         └─► ui.rs            │ dispatch / get_state (one mutex)
                                              ▼
 ┌──────────── anagram core (src/) ───────────────┐   session.rs ──► dto.rs ──► theme.rs
 │ app.rs ─► game.rs, dictionary.rs,              │        │
 │           signature_index.rs, workbench.rs,    │◄───────┘ (reduce, affix_hints)
 │           config.rs                            │
 │ affix_hints.rs ─► game.rs (Round)              │
 │ solve.rs ─► signature_index.rs                 │
 └────────────────────────────────────────────────┘

 ui-web/ (TypeScript) ── invoke("get_state" | "dispatch") ──► Tauri commands
```

## 3. Data

### 3.1 Embedded assets

Compiled in with `include_str!`.

| File | Bytes | Metadata | Embedded by | Used for |
|------|-------|----------|-------------|----------|
| `data/words.json` | 15,447,253 | `entry_count` 86,587 | `dictionary.rs` and `affix_hints.rs` (both `WORDS_JSON`) | Words, definitions, clues |
| `data/signature_index.json` | 22,805,242 | `entry_count` 86,587; `bucket_count` 78,278; `format_version` 1 | `signature_index.rs` | Anagram lookup |
| `data/affix_patterns.json` | 9,920 | `word_count` 86,587; `ngram_len` [2, 5] | `affix_hints.rs` | Corpus rank of prefixes and suffixes |
| `data/common_allowlist.txt` | 75,071 | — | Not embedded | Read by `enrich_wordnet.py` in `--mode common` |

`words.json` is embedded twice (§8, item 7). Test fixtures are in `data/fixtures/`. `data/ATTRIBUTION.md` credits WordNet.

### 3.2 Generation pipeline

`make data` runs these steps. It needs WordNet 3.x `dict/` at `WN_DIR` (default `/tmp/wn/dict`).

| Step | Command (Makefile) | Result |
|------|--------------------|--------|
| 1 | `python3 scripts/enrich_wordnet.py --mode wordnet-full --emit-manifest data/words.manifest.json --output data/words.json` | Lemmas with glosses, length 4–10 |
| 2 | Gate: entries ≥ `MIN_ENTRIES` (35000) | Fails the target otherwise |
| 3 | `python3 scripts/build_signature_index.py --input data/words.json --output data/signature_index.json` | Sorted-letter buckets |
| 4 | `python3 scripts/build_affix_patterns.py --input data/words.json --output data/affix_patterns.json` | Corpus n-gram ranks (2–5 letters, top 400 per side) |

`make data-index` runs steps 3 and 4 only. The scripts use the Python standard library. `enrich_wordnet.py` defaults to
`--mode common`; the Makefile overrides it.

## 4. Algorithms

### (a) Signature lookup

- `signature_key(word)` keeps ASCII letters, lowercases, and sorts: `listen` and `silent` → `eilnst`.
- `SignatureIndex { buckets: HashMap<String, Vec<WordEntry>> }`, loaded by `from_embedded` / `from_str`.
  `format_version` must be 1. `source.sha256` is parsed but not checked.
- `signature_from_str` returns `Empty` or `InvalidCharacter` for bad input. `lookup` returns `Matches(bucket clone)` or `NoMatch`.
- `same_length_anagram_alternates` looks up the solution, keeps same-length entries, and drops the solution itself.

### (b) Word selection and identity-avoiding shuffle

- `begin_round` calls `Dictionary::random_word_excluding(size, previous, rng)`, which skips the previous word when another
  candidate exists. No candidate → `Status::Error("No words of length N in dictionary.")`.
- Custom words use `begin_round_with_word` and `Dictionary::resolve_entry` (synthetic definition for unknown words).
- `Round::new` uppercases the word and calls `shuffle_pool_not_identity`, which reshuffles until the pool differs from
  the solution order (skipped when the word has one distinct letter). `reset_same_word` (Ctrl+R) reuses the same guard.
- `LetterWorkbench::from_solution` and `reset_grid` use a separate private `shuffle_not_identity`.

### (c) Reducer: `app::reduce(state, action, dict, index, rng)`

Phases: `Menu`, `SizeSelect`, `CustomWordInput`, `CustomWordPick`, `Playing`, `Help`, `Won`. The desktop session returns
an error if the reducer lands in `Menu` or `SizeSelect`.

| Action | Phases | Effect |
|--------|--------|--------|
| `NewPuzzle` | any | `begin_round(size)` → `Playing`, help closed |
| `SelectSize(n)` | any | `size = n.clamp(4, 10)`, then `begin_round` → `Playing` |
| `ResetRound` | `Playing`, `Help`, `Won` | `reset_same_word`; clues cleared; grid reshuffled; `Won` → `Playing` |
| `ToggleHelp` | `Playing` ↔ `Help` | Flips `help_open` and phase; in `Won` only `help_open` flips |
| `RequestClue` | `Playing`, `Help` | Clue cycle, then `auto_check_solution` |
| `TypeLetter`, `Backspace`, `CursorLeft/Right`, `CursorSet` | `Playing`, `Help` | Round edits (d, e) |
| `OpenCustomWord` | any | → `CustomWordInput`; input and choices cleared; help closed |
| `CustomWordType`, `CustomWordBackspace` | `CustomWordInput` | ASCII letters only; capped at `max_word_len` |
| `SubmitCustomWord` | `CustomWordInput` | Validate, then resolve (below) |
| `CustomPickDelta(d)`, `CustomPickSelect(i)` | `CustomWordPick` | Index wraps modulo N; select sets index if `i < N` |
| `ConfirmCustomPick` | `CustomWordPick` | Starts the chosen word |
| `CancelCustomWord` | custom phases | Clears input and choices; → `Menu` |
| `WorkbenchClickCell`, `WorkbenchClickPool`, `WorkbenchResetGrid`, `SetInputFocus` | play phases | Workbench (g) and focus |

**Clue cycle.** `clues_for_round(primary, alternates)` gives the list; budget = its length. `reduce_request_clue` applies
`clues[clue_index % budget]`, increments `clue_index`, and sets `clues_used = min(clue_index, budget)`. A primary entry with
three or more static clues uses those. Otherwise the list is the definitions of the solution and its alternates (up to 16
WordNet noun senses each), then `CorrectPositions`, then `FirstLetter`.

**Custom-word resolution.** `validate_custom_word` checks ASCII letters and length; failure keeps the phase and sets an error.
`resolve_scramble_or_word` runs `index.lookup(typed)`:
- `NoMatch` → the typed word is the solution.
- Matches: an entry spelled like the input wins; one match wins; several → `CustomWordPick`.

The round then sets `size = solution.len()` and phase `Playing`.

**Size bounds.** Core: `SIZE_MIN` 4, `SIZE_MAX` 10. Desktop `Session::build`: `lo = min_size.clamp(4, 10)`, `hi = max_size.clamp(4, 10)`;
fails if `lo > hi` or a length in `lo..=hi` has no word (`ensure_words_for_range`). `Session::clamp_action` clamps `selectSize`.

### (d) Win check and wrong-answer status

- `Round::try_type_letter` writes the cursor slot and moves to the next empty slot on the right. A failed type sets
  "Letter not in pool.".
- `auto_check_solution` runs after a successful type and after `RequestClue`. It does nothing unless every slot is full.
  A guess wins on an exact (case-insensitive) match of the solution or a same-length alternate → `Phase::Won`,
  `Status::Info("Correct!")`, help closed. Otherwise `Status::WrongAnswer`.
- `Backspace` clears `WrongAnswer` status. Text: "Not quite — check your letters." (`ui.rs`; `dto.rs` `WRONG_ANSWER_TEXT`).
- `Round::check_solution` (`Incomplete`/`Correct`/`Wrong`) is not used by the reducer (§8, item 10).

### (e) Letter pool bookkeeping

Invariant: `multiset(solution) == pool + filled slots` (`multiset_matches`; `debug_assert!` in `assert_invariant`).

| Operation | Pool | Answer |
|-----------|------|--------|
| Type | Removes the first matching letter | Writes `answer[cursor]`; cursor moves right |
| Backspace | Appends the cleared letter | Clears cursor slot; if empty, steps left and retries |
| First-letter clue (`reveal_first_letter`) | Removes the first matching letter | Writes slot 0 only if empty |
| Reset (`reset_same_word`) | Refills from solution, reshuffles | All slots cleared; cursor 0 |

The workbench has its own pool and cells; `debug_assert_invariant` checks cells + pool + held = length.

### (f) Affix hints (Patterns panel)

`affix_hints(round)`:
1. Count letters in pool and filled slots. `max_chunk = min(max(total, 4), 6)`.
2. `affixes_from_spellable_words`: for every lexicon word spellable from those letters (`multiset_can_spell`), count prefixes
   and suffixes of length 2 to `max_chunk`. Sort by hit count (desc), corpus rank (asc), length (desc), then lexicographic.
   Keep 22 starts and 22 ends.
3. `filter_patterns_ordered` tops up from `affix_patterns.json` with spellable, non-duplicate patterns no longer than `max_chunk`.
4. Suffixes are stored with a leading `-`.

The lexicon is parsed once (`OnceLock`); each call still scans all words. The TUI calls `affix_hints` on every frame
(`ui.rs` `draw_affix_hints_panel`). The desktop caches per solution (`Session::ensure_affix_cache`) and computes only while
`affix_visible` (workbench on; phase `Playing`, `Help`, or `Won`).

### (g) Letter workbench

State: `rows`, `cols` (4 or 5, square), `cells`, `pool`, `cursor`, `held`, private `solution`.

| Operation | Behavior |
|-----------|----------|
| `click_cell(idx)` | Out of range → `false`. Hand empty and cell empty → `false`. Hand empty and cell full → pick up. Hand full and cell empty → place. Both full → swap |
| `click_pool(idx)` | Out of range or hand full → `false` (D-1). Otherwise picks up `pool[idx]` |
| `pick_place_swap()` (TUI Space) | Like `click_cell` on the cursor; empty hand and empty cell takes `pool[0]` |
| `try_type_letter(ch)` | Empty cursor cell only; takes the first matching pool letter |
| `clear_cell()` (TUI Backspace) | Returns the cursor cell's letter to the end of `pool` |
| `reset_grid(rng)` | Gathers all letters, empties cells, reshuffles with the identity guard |

Desktop: pointer-only. A panel click first dispatches `workbenchSetFocus('grid')`, then the cell or pool action; IPC is
FIFO (D-15). The workbench never affects win state.

### (h) Config and colors

- `AppConfig::load(path)`: missing file → defaults; malformed TOML → error. `#[serde(default)]` fills missing keys; unknown keys are accepted.
- Defaults: `min_size` 4, `max_size` 10, `locale` "en-US", `theme` "default", four slot/pool colors (`light_magenta`,
  `light_yellow`, `light_cyan`, `light_green`), `workbench` true, `workbench_rows` 5, `workbench_cols` 5.
- `workbench_settings()`: rows ≠ cols, or either not 4 or 5 → 5×5.
- `parse_color_name`: trims, lowercases, 16 names (`light_green` and `lightgreen` both work); unknown → `Magenta`.
- `theme::color_to_css`: xterm table (`LightGreen` → `#00ff00`); `Reset`, `Rgb`, `Indexed` → `#cd00cd`.
- `theme_dto` converts the four color keys. The frontend never parses colors; `theme-apply.ts` sets CSS variables.

## 5. Desktop IPC design

- **Commands:** `dispatch(action) -> Result<UiState, String>` mutates; `get_state()` reads (called once at startup).
- **Lock:** `AppState { engine: Arc<Engine>, session: Mutex<Session> }`. `Engine` is read-only and unlocked. Both commands
  take the mutex; a poisoned lock returns `"internal error"`.
- **Inbound:** `ActionDto` is `#[serde(tag = "type", rename_all = "camelCase")]`. `into_actions()` is 1:1 except:
  - `cancelCustomWord` → `[CancelCustomWord, NewPuzzle]` (D-9). Outside custom phases, `Session::apply` makes it a no-op.
  - `workbenchClear` → `WorkbenchResetGrid`; `workbenchSetFocus` → `SetInputFocus`.
  - Non-letter `typeLetter` or `customWordType` → `DtoError::InvalidLetter` ("invalid letter: X").
  - There is no quit action.
- **Outbound:** `UiState` (camelCase): `phase`, `helpOpen`, `length`, `minSize`, `maxSize`, `clues {used, budget}`,
  `activeClue`, `status {kind, message}`, `round?`, `workbench?`, `custom {input, choices, pick}`, `theme`.
  `round` and `workbench` are omitted when absent.
- **Visibility:** `round.solution` only in `won`. `round.definition` only in `won` (`solved_definition`, from the entry for
  the typed letters). `round.correct` only while the correct-position clue is on. Custom phases hide `round`, `workbench`, and
  clues. `custom.choices` (word and definition only) only in `customWordPick`. `workbench.affixes` only when `affix_visible`;
  `affix_dto` drops hints whose letters equal the solution. `workbench.placed` is always empty.
- **Dev fixtures:** `ipc.ts` serves `?fixture=` only when `import.meta.env.DEV` is true and not inside Tauri. The production
  bundle contains no fixture strings (checked in `ui-web/dist`).
- **Capabilities:** `capabilities/default.json` grants `core:default` to window `main` only.

## 6. Frontend structure

| Concern | Module | Notes |
|---------|--------|-------|
| Render | `render/index.ts` | Template mounted once. Slider: `input` sets a draft, `change` dispatches. Answer slots `li.slot`; pool tiles `button.tile` send `typeLetter` |
| Input | `input/keymap.ts`, `input/index.ts` | `keyToAction` returns `ActionDto` or null. `escapeAction`: closes help, or cancels a custom phase; never quits. `isCtrlR` → `resetRound` in play phases |
| Overlays | `overlays/index.ts` | `guardedDispatch` confirms `newPuzzle`, `openCustomWord`, `selectSize` in `playing`/`help` (`GATED_*`, D-8). `won` never confirms |
| Workbench | `workbench/index.ts` | Panel click listener only; no key listeners. `style.ts` injects its `<style>` |
| A11y | `a11y.ts` | `applyA11y` sets ARIA; `trapTab` is a window capture `keydown` listener; `remember` tracks focus |
| State | `main.ts`, `state.ts` | `RenderCtx { state, local, dispatch, requestReplace, setLocal }`; each result replaces `state` and re-renders |

Frontend rule: `ui-web` renders `UiState` and sends `ActionDto`. It holds no word lists, answer checks, or game rules.

## 7. Testing layout

| Layer | Location | Observed (2026-11-25) | Command |
|-------|----------|-----------------------|---------|
| Core unit | `#[cfg(test)]` in `src/*.rs` | 125 passed | `make test` |
| Desktop unit | `src-tauri/src/*.rs` | 37 passed | `cargo test -p anagram-desktop` |
| Desktop integration | `src-tauri/tests/config.rs` (4), `ipc_leak.rs` (2) | 6 passed | same |
| Frontend | `ui-web/src/**/*.test.ts` | 80 passed (3 files) | `npm --prefix ui-web run test` |

Notable tests: `dto.rs` `playing_hides_solution`, `won_exposes_solution`, `definition_absent_in_playing_and_help`,
`custom_phases_hide_round`, `pick_exposes_choices_only`, `full_length_affix_hints_are_dropped`; `session.rs`
`reducer_equivalence_fixed_seed`, `cancel_outside_custom_is_noop`, `affix_cache_reuse`, `startup_rejects_empty_size_range`;
`game.rs` `shuffle_not_identity_when_multiple_distinct_letters`; `ipc_leak.rs` `ipc_leak_sweep` and `custom_phase_invariants`.
`ipc_leak.rs` drives seeded random actions through `lock_session` and `Session::dispatch`. Whole-word solution hits in
`activeClue` or `definition` are reported as findings, not failures, because WordNet glosses can contain the answer.

## 8. Known limitations and open issues

| # | Area | Issue | Where |
|---|------|-------|-------|
| 1 | Affix hints | Full-length matches of a short answer can appear. Desktop drops them; TUI shows them | `affix_hints.rs`; `dto.rs` `affix_dto` |
| 2 | Custom-word cancel | Desktop cancel sends `cancelCustomWord` then `newPuzzle`, replacing the unfinished round | `dto.rs` `into_actions` (D-9) |
| 3 | Keyboard access | Desktop workbench is pointer-only. Answer slots are `li` elements, not focusable | `workbench/index.ts`; `render/index.ts` |
| 4 | Workbench layout | Layout B is "pending decision" (SPEC-ui §3.4.1, Q5); only `grid` is implemented. Non-square or non-4/5 dims become 5×5 | `config.rs`; `dto.rs` `LayoutDto` |
| 5 | TUI config | `min_size > max_size` panics in `clamp_size`; exit 101 observed under a pseudo-terminal. Desktop returns a startup error | `config.rs` |
| 6 | Esc divergence | Terminal Esc quits (play, navigation, custom pick). Desktop Esc never quits | `input.rs`; `keymap.ts` |
| 7 | Duplicate data | `words.json` is embedded in two modules (about 15 MB each in the binary) | `dictionary.rs`, `affix_hints.rs` |
| 8 | Unused keys | `locale` and `theme` are parsed; no code reads them | `config.rs` |
| 9 | TUI cost | Patterns recomputed every frame while visible | `ui.rs` |
| 10 | Dead code | `app::solve_after_fill` is not called by the reducer | `app.rs` |
| 11 | Pick list | `CustomWordPick` sends all candidates (SPEC-ui §11 Q3) | `dto.rs` `CustomDto` |
| 12 | Confirm state | `local.pending` lives only in the frontend | `state.ts`; `overlays/index.ts` |
| 13 | Generated files | `src-tauri/gen/` is created by builds and is not ignored | repo root |
