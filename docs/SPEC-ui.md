# Anagram Desktop UI — Product & Technical Specification (Tauri)

## 1. Overview & Status

**Status:** Draft (v1 scope)  
**Product name (working):** Anagram Desktop  
**Platform:** macOS first (see §8)  
**Implementation:** Tauri v2 desktop app — Rust backend over the existing `anagram` core crate, TypeScript frontend  
**Goal:** Offer the terminal game's experience in a resizable desktop window. The Rust game core is reused; the GUI adds presentation and input only.

**Relationship to the TUI:** The terminal app ([SPEC.md](SPEC.md)) remains supported and its behavior is unchanged. The GUI is a second front end over the same terminal-independent modules (`app`, `game`, `dictionary`, `signature_index`, `workbench`, `affix_hints`, `solve`, `cli`, `config`).

**Shared rules:** Puzzle generation, the letter-pool/answer invariant, win check, clue order, and per-round clue budget are defined in [SPEC.md](SPEC.md) §3–§4.4 and [SPEC_LETTER_WORKBENCH.md](SPEC_LETTER_WORKBENCH.md). They are not restated here.

**Controls source of truth:** `src/input.rs`. Where [SPEC.md](SPEC.md) §4.4–§4.5 differs (for example, `r` and `c` are typed letters during play; reset during play is Ctrl+R), `src/input.rs` governs this spec. Exception: the desktop workbench is pointer-first (§3.4, §3.6).

---

## 2. Goals & Non-Goals

| Goals | Non-Goals |
|-------|-----------|
| Feature parity with TUI v1 gameplay (§3) | Mobile (iOS / Android) |
| Reuse the `anagram` core crate with no change to game rules | Web or hosted deployment |
| One source of game logic: Rust `app::reduce` | Multiplayer / networking |
| Keyboard controls matching TUI bindings (§3.6) | Changing game rules |
| Responsive layout in a resizable window | Terminal-size behavior (80×24, resize reflow) |
| Answer never sent to the UI before a win (§5) | Reducing word data size (≈37 MB embedded list accepted as-is) |
| Config file shared with the TUI (§6) | Accounts, timers, leaderboards |

---

## 3. Feature Parity with TUI

### 3.1 Puzzle sizes

- Word length N comes from config `min_size` / `max_size` (default 4–10).
- **No start screen.** The app launches straight into a puzzle at `config.clamp_size(5)` (§4.3). Length selection and new-puzzle start live in the main play window.
- **Top toolbar** (a row at the top of the play panel, above the board):
  - **Length slider** from `min_size` to `max_size` (default 4–10), showing the current puzzle length as a numeric value.
  - **Custom word** button.
  - **New puzzle** button.
  - The board, pool, answer row, clue, workbench, and footer stay below the toolbar.
- **Slider semantics.** The slider always reflects the current puzzle's length. A custom word sets the slider to that word's length. Moving the slider starts a new random puzzle of the chosen length. The new puzzle starts on pointer release; if the user cancels the confirmation, the slider returns to the current length.
- **Custom word** (toolbar button; no keyboard shortcut during Playing, see §3.6): same validation (letters A–Z, length within bounds), same scramble resolution via `SignatureIndex`, and the same pick list when several words match.
- **Puzzle replacement rule (confirmation).** Any action that replaces the current puzzle shows a confirmation dialog **when the current puzzle is unfinished** (not yet won):
  - Actions: **New puzzle**, **Custom word** (button press), and **slider move** (on release).
  - Dialog: "Confirm abandon?" with **Cancel** (no change) and **Abandon** (proceed).
  - **Won puzzle:** no dialog; the action proceeds immediately.
  - The check reads `UiState.phase` (not `won`) in the frontend. This is presentation-only input gating; the reducer is unchanged.
  - Cancel-after-abandon: because Custom word replaces the board when its input opens, Cancelling or pressing Esc in the Custom word input starts a new random puzzle at the current length (§3.6). The abandoned puzzle is not restored.

### 3.2 Gameplay

- Answer slots, scrambled pool that shrinks and grows, cursor-based fill, auto-check when all slots are full, and a Won banner. Rules: [SPEC.md](SPEC.md) §3.2–§3.4.
- Reset (same word, reshuffled pool) clears answer and clue state.

### 3.3 Clues

- The active clue is shown in a clue line. The clue budget is `Dictionary::clue_budget_round`; the header shows `used/total`.
- The next clue is requested with **Tab** during play (see §3.6 parity notes).
- Clue kinds and order come from `Dictionary::clues_for_round` (core, unchanged). The first-letter clue reveals a letter and preserves the pool/answer invariant.

### 3.4 Letter workbench and affix hints

- Enabled by config `workbench` (default `true`). Grid size is `workbench_rows` × `workbench_cols` via `AppConfig::workbench_settings()`, which clamps invalid values to 5×5.
- **Pointer-only.** The workbench has no keyboard focus or grid shortcuts. Hover highlights, click focuses and acts, drag is optional. Pointer table in §3.6.
- The workbench has **its own letter pool** (`WorkbenchDto.pool`), separate from the round pool (`RoundDto.pool`). Both start as independent shuffles of the solution letters. Moving letters in one never changes the other.
- The workbench is **cue only**: it never changes answer or win state.
- **Patterns (cue)** panel: prefix and suffix patterns from `affix_hints::affix_hints(&Round)`, shown inside the workbench panel.
- The pattern set depends only on the letter multiset of pool ∪ answer, which is invariant within a round. **The backend computes it once per round and caches it** (§4.3). The TUI recomputes it per frame; the GUI must not.

#### 3.4.1 Workbench layout — pending decision

Two options are under consideration. **Neither is final.**

- **Option A — free-form canvas.** An empty canvas. Letters can be placed anywhere (free x, y). No fixed cells.
- **Option B — sparse 5×5 grid (proposed).** A fixed 5×5 grid, mostly empty. Tiles can move to any empty cell, up, down, or across.
  - Cells keep identity: each cell has a fixed index and holds at most one tile. No overlap.
  - Placing a tile on an occupied cell swaps, as in the TUI.
  - Config may set 4×4 (§6). The proposal assumes the 5×5 default.
  - **Clear** button returns all tiles (cells and hand) to the workbench pool and reshuffles it.

**Recommendation:** B — *proposed, pending decision*.

Trade-offs:

- **A** is more expressive for spatial arrangement. It needs a new model (positions, not cells), so `LetterWorkbench` cannot be reused unchanged. It also needs overlap rules and a bounded or scrolling canvas.
- **B** reuses the existing `LetterWorkbench` model (`rows`, `cols`, `cells`, `pool`, `held`), so the core stays unchanged. Hit-testing and drag-to-snap are simple. The cost is less freedom and a sparse grid that may look empty.
- Both options keep the cue-only rule: layout never affects answer or win state.
- The layout choice does not change the affix (Patterns) panel.

### 3.5 Theme colors

Four config keys set the colors of empty answer slots, filled answer letters, the scrambled pool, and correct-position highlights. Translation to CSS is defined in §6.

### 3.6 Keyboard controls

Authoritative source: `src/input.rs`. Keys are case-insensitive for letters.

**Global**

| Key | Action | Where |
|-----|--------|-------|
| Esc | Quit (close window) | All phases |
| ⌘Q / Ctrl+C | Quit (close window) | All phases |

**Playing / Help — answer focus**

| Key | Action |
|-----|--------|
| `a`–`z` | Type letter into the cursor slot (ignored if not in pool) |
| `←` / `→` | Move cursor |
| Backspace | Clear cursor slot, or step left and clear |
| Tab | Next clue |
| `?` | Toggle help overlay |
| Ctrl+R | Reset round (same word, reshuffled) |

**Workbench — pointer** (Playing, Help, Won; no keyboard bindings)

| Pointer action | Effect |
|----------------|--------|
| Hover tile or cell | Hover highlight (frontend only; no dispatch) |
| Click workbench panel | Focus workbench (`focus = grid`; focus ring) |
| Click answer row | Focus answer (`focus = answer`) |
| Click tile in workbench pool | Pick it up (`held`); removed from workbench pool |
| Click empty cell while holding | Place held tile (Layout B; Layout A: at click point) |
| Click filled cell, hand empty | Pick up that tile |
| Click filled cell while holding | Swap |
| Drag tile (optional) | Same as pick, then place |
| **Clear** button | Return all workbench tiles (cells and hand) to workbench pool, reshuffled |

**Won — answer focus**

| Key | Action |
|-----|--------|
| `n` | New puzzle (same size) |
| `i` | Custom word input |
| `r` / Ctrl+R | Reset round (returns to Playing) |
| `?` | Toggle help overlay |

**Custom word input** (opened from the toolbar or Won)

| Key | Action |
|-----|--------|
| `a`–`z` | Type (capped at max length) |
| Enter | Submit |
| Backspace | Delete last letter |
| Esc | Cancel, return to the play window with a new random puzzle at the current length (§3.1) |

**Custom word pick**

| Key | Action |
|-----|--------|
| `↑` / `↓` or `k` / `j` | Move highlight |
| Enter | Start puzzle with highlighted word |
| `u` | Cancel, return to the play window with a new random puzzle at the current length (§3.1) |
| Esc | Quit (TUI parity; see §11) |

**Confirm abandon** (dialog; shown only per the §3.1 rule): pointer only, **Cancel** and **Abandon** buttons. Keyboard binding is open (§11).

**Parity notes**

- `c` maps to "next clue" in `input.rs`, but the reducer ignores clue requests outside Playing / Help. Tab is the only effective clue key, in both TUI and GUI.
- During Playing / Help, `r` and `c` are letters. Reset is Ctrl+R.
- Workbench keyboard bindings exist only in the TUI (`src/input.rs`). The desktop workbench is pointer-first and binds none of them.
- **Intentional deviation (Size screen removed):** the GUI omits the Size-screen keys (`s`, and digits `4`–`9`, `0` that start a puzzle of a size). Length is set by the toolbar slider (§3.1). Keyboard length selection is open (§11).
- During Playing, `n` and `i` are letters, so New puzzle, Custom word, and the slider have no keyboard shortcut there. Won keeps `n`, `i`, and `r`.
- The GUI keeps Esc / ⌘Q as quit in every focus state (intentional deviation).
- The GUI ignores unlisted Ctrl/Meta chords. The TUI types the letter for Ctrl+letter in custom input (intentional deviation).

---

## 4. Architecture

### 4.1 Repository layout (target)

```
Cargo.toml              # [workspace] added; anagram package unchanged
src/                    # anagram lib + bin (TUI, behavior unchanged)
src-tauri/              # Tauri v2 app crate (anagram-desktop), path dependency on ".."
  tauri.conf.json       # window size limits, bundle, capabilities
  src/main.rs           # setup(): load config and data, manage state, register commands
  src/dto.rs            # UiState / ActionDto and mapping from core types
  src/theme.rs          # config color names → CSS colors
ui-web/                 # Vite + TypeScript frontend (renders UiState only)
  package.json
  src/
docs/SPEC-ui.md         # this file
```

The frontend directory is **`ui-web/`**.

### 4.2 Cargo workspace

- Root `Cargo.toml` adds `[workspace] members = [".", "src-tauri"]` and `default-members = ["."]`. Plain `cargo test` and `make test` therefore keep covering only the TUI crate; use `--workspace` for the GUI crate.
- `src-tauri` depends on `anagram = { path = ".." }`. There is one `Cargo.lock`, at the root.
- The root crate keeps `rust-version = "1.75"`. Tauri v2 needs a newer toolchain. `src-tauri` declares its own `rust-version`, matching the pinned Tauri release (verify at implementation time).

### 4.3 Backend (`src-tauri`)

**Startup (`setup`)**

1. `AppConfig::load(None)`. On error, show a native error dialog and exit (matches TUI, which fails on bad config).
2. `Dictionary::from_embedded()` and `SignatureIndex::from_embedded()`. Both are read-only, so they are managed as `Arc` with **no lock**. Loaded once.
3. Initial state, as in `lib.rs::run_play`: `GameState::with_bounds(config.clamp_size(5), config.min_size, config.max_size)` with `workbench_settings = config.workbench_settings()`. No start screen: immediately start a random puzzle at `config.clamp_size(5)`.
4. `StdRng::from_entropy()`.
5. Managed as `Mutex<Session>`, where `Session { state: GameState, rng: StdRng, affix_cache: Option<(String, AffixHints)> }`. The cache is keyed by the internal solution string, which is never serialized.

**Commands** (the only two)

| Command | Behavior |
|---------|----------|
| `dispatch(action: ActionDto) -> UiState` | Sole mutation entry point. Locks the session, converts to `app::Action`, calls `app::reduce`, rebuilds `UiState`, returns it. `Quit` is never sent to `reduce`; the shell closes the window. |
| `get_state() -> UiState` | Read-only. Used once for the initial render. |

**Concurrency:** all dispatches are serialized by the mutex. The frontend sends keys in order and awaits each response.

**Affix cache:** `affix_hints` scans the full lexicon on every call. It is computed when a round starts and reused for the round's lifetime, and it is recomputed only if the cache key changes.

**Capabilities:** the frontend gets no filesystem, shell, or network plugin permissions. Config is read in Rust. Word data never leaves the backend.

### 4.4 Frontend (`ui-web`)

- Vite + TypeScript. A UI framework is optional and not required for v1.
- Renders `UiState` only. Holds ephemeral UI state (focus rings, animation, dialog open/close).
- Translates `KeyboardEvent` → `ActionDto` using the keyboard table in §3.6, and workbench pointer events using the pointer table in §3.6. This is input translation, not game logic. Phase and focus are read from the last `UiState`.
- **No game logic in JS:** no word lists, no answer checks, no clue selection, no pool/answer rules. Presentation-only derived text (for example, which banner to show) is allowed.

---

## 5. Data Contract (IPC)

### 5.1 Rules

1. **The DTO is the only shape that crosses IPC.** Core types (`GameState`, `Round`, `WordEntry`, `LetterWorkbench`, `Action`) do not derive `Serialize`.
2. **Answer-revealing data is excluded until `Phase::Won`:**
   - `Round.solution` is sent only when `phase == won`.
   - `GameState.current_word` (contains `word`, `definition`, `noun_senses`, clue list) is never sent. Only the rendered active-clue string and the clue budget are sent.
   - `GameState.anagram_alternates` is never sent. Alternate words are valid answers.
   - Per-slot correctness flags are sent only while `highlight_correct_positions` is on. Backend computes them with `Round::slot_is_correct`.
3. Candidate words in `CustomWordPick` are sent (§11, decision 5).
4. Affix patterns are sent during play (§11, decision 6).

### 5.2 Why DTO mapping, not `Serialize` on core types

- Deriving `Serialize` on core types that hold answer data makes any future field leak by default. A DTO is an explicit allow-list, so leaks require an intentional change.
- `Action` has tuple variants and internal shapes. The wire format needs a stable tagged enum that the TypeScript side can produce.
- The core crate's types and public API stay unchanged, which keeps the TUI untouched.

### 5.3 `UiState` (shape sketch)

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    pub phase: PhaseDto,          // customWordInput | customWordPick | playing | help | won (no menu / sizeSelect in GUI)
    pub help_open: bool,
    pub size: u8,
    pub min_len: u8,
    pub max_len: u8,
    pub clues: ClueDto,           // { used, budget }
    pub active_clue: Option<String>,
    pub status: StatusDto,        // { kind: none | error | info | wrongAnswer, message }
    pub round: Option<RoundDto>,
    pub workbench: Option<WorkbenchDto>,
    pub custom: CustomDto,        // { input, choices: [{ word, definition }], pick }
    pub theme: ThemeDto,          // four CSS color strings
}

pub struct RoundDto {
    pub length: u8,
    pub pool: Vec<char>,
    pub answer: Vec<Option<char>>,
    pub cursor: usize,
    pub correct: Option<Vec<bool>>,   // Some only while highlight_correct_positions
    pub solution: Option<String>,     // Some only when phase == Won
}

pub struct WorkbenchDto {
    pub layout: LayoutDto,            // grid | free (§3.4.1; proposed: grid, pending decision)
    pub rows: u8,                     // grid layout only
    pub cols: u8,                     // grid layout only
    pub cells: Vec<Option<char>>,     // grid layout only: rows × cols, row-major
    pub placed: Vec<PlacedDto>,       // free layout only (option A): { ch, x, y }
    pub pool: Vec<char>,              // workbench's own pool; not round.pool
    pub cursor: usize,                // last clicked cell index; hover is frontend-only
    pub held: Option<char>,           // tile picked up by click
    pub focus: FocusDto,              // answer | grid; set by click
    pub affixes: AffixDto,            // { starts, ends }, cached per round
}
```

### 5.4 `ActionDto`

- Serde tagged enum: `#[serde(tag = "type", rename_all = "camelCase")]`.
- One variant per `app::Action` variant, except `Quit` (handled by the shell).
- Example: `{ "type": "typeLetter", "ch": "a" }`.
- Conversion `TryFrom<ActionDto> for app::Action` rejects non-ASCII-alphabetic characters in letter variants. `reduce` also guards.
- Workbench pointer actions (proposed): `workbenchClickCell { idx }`, `workbenchClickPool { idx }`, `workbenchSetFocus { focus }`, and `workbenchClear` (maps to `WorkbenchResetGrid`). The first three have no `app::Action` equivalent yet. Layout A replaces `idx` with `{ x, y }`.

### 5.5 Visibility by phase

| Field | Custom input | Custom pick | Playing / Help | Won |
|-------|:------------:|:-----------:|:--------------:|:---:|
| `round.solution` | — | — | — | ✔ |
| `round.pool`, `round.answer` | — | — | ✔ | ✔ |
| `round.correct` | — | — | if highlight on | if highlight on |
| `custom.choices` | — | ✔ | — | — |
| `workbench.affixes` | — | — | if workbench on | if workbench on |

---

## 6. Configuration

**File:** `~/.config/anagram/config.toml`, the same file and schema as the TUI. Loaded with `AppConfig::load(None)`. The GUI has no `--config` flag in v1 (apps launched from Finder cannot pass arguments). The file is read once at startup; there is no hot reload.

**Missing file:** all defaults. **Malformed file:** startup error dialog (§4.3).

| Key | Type | Default | GUI use |
|-----|------|---------|---------|
| `min_size` | u8 | 4 | Lower bound of puzzle sizes; slider minimum |
| `max_size` | u8 | 10 | Upper bound of puzzle sizes; slider maximum |
| `locale` | string | `en-US` | Parsed; not used in v1 (same as TUI) |
| `theme` | string | `default` | Parsed; not used in v1 (same as TUI) |
| `empty_slot_color` | string | `light_magenta` | Empty answer slots |
| `filled_slot_color` | string | `light_yellow` | Filled answer letters |
| `pool_color` | string | `light_cyan` | Scrambled pool letters |
| `correct_position_color` | string | `light_green` | Correct-position highlight |
| `workbench` | bool | `true` | Show workbench |
| `workbench_rows` | u8 | 5 | Grid rows (clamped with `workbench_cols`) |
| `workbench_cols` | u8 | 5 | Grid columns |

**Color translation (backend, `src-tauri/src/theme.rs`):**

1. Parse each name with `anagram::ui::parse_color_name`. It is case-insensitive and accepts the same aliases as the TUI.
2. Map the resulting terminal color to a CSS color string using the xterm palette. The frontend receives plain CSS colors and never parses names.
3. Unknown names fall back to magenta, matching the TUI.

Defaults in the xterm palette: `light_magenta` → `#ff00ff`, `light_yellow` → `#ffff00`, `light_cyan` → `#00ffff`, `light_green` → `#00ff00`. Exact values can be tuned for legibility on the GUI background, but the mapping must stay in one table.

---

## 7. Layout & Window

- **No 80×24 constraint.** Terminal-size behavior does not apply.
- **Resizable window.** `tauri.conf.json` declares `minWidth` / `minHeight` (proposed: **640 × 480**) and a default size (proposed: **960 × 640**). Tune both during manual testing.
- **Responsive layout (CSS):**
  - The top toolbar (§3.1: length slider, Custom word, New puzzle) spans the play panel.
  - Board (header, pool, answer row) is centered.
  - Workbench panel sits beside the board when the window is wide enough and stacks below it otherwise.
  - Pool letters wrap. Clue text wraps.
  - Help is a modal overlay with the same content as the TUI help overlay.
- **Status text:** errors and wrong-answer feedback use text, not color alone (matching the TUI accessibility rule).

---

## 8. Platforms

- **macOS first.** This is the only platform with manual test coverage in v1.
- Linux and Windows are not blocked by design. Avoid platform-specific APIs; use `std::env::home_dir` through the existing config path. They are not tested in v1.

---

## 9. Build & Run

**Prerequisites:** Rust toolchain (version per §4.2), Node.js LTS, Tauri CLI v2 (`cargo install tauri-cli --version "^2"`).

| Task | Command |
|------|---------|
| Dev run (window + Vite HMR) | `cargo tauri dev` (from repo root) |
| Release bundle (macOS `.app`) | `cargo tauri build` |
| Frontend only | `npm --prefix ui-web run dev` / `npm --prefix ui-web run build` |
| TUI tests (unchanged) | `cargo test` (root, via `default-members`) |
| GUI Rust tests | `cargo test -p anagram-desktop` |

`cargo tauri dev` and `build` invoke the `ui-web` npm scripts through `tauri.conf.json` (`beforeDevCommand` / `beforeBuildCommand`).

`make` targets for GUI build and run are **to be added later**.

---

## 10. Testing

### 10.1 Rust unit tests (`src-tauri`)

| Area | Tests |
|------|-------|
| DTO hiding | For Custom input, Playing, Help: serialized `UiState` contains no `solution` key and no string equal to the solution |
| DTO win exposure | In Won, `round.solution` equals the solution |
| Correctness flags | `correct` is `None` unless highlight is on; when on, flags match `slot_is_correct` |
| Alternates hidden | Round with alternates: no alternate word appears in any serialized state |
| ActionDto mapping | Every `ActionDto` variant maps to the matching `app::Action`; non-letter `ch` is rejected |
| Reducer integration | Fixed-seed sequence of `dispatch` calls produces the same state as calling `app::reduce` directly |
| Affix cache | Same round → cached result reused; new round → recomputed |
| Theme mapping | All 16 names map to expected CSS; unknown name → magenta |
| Config | Missing file → defaults; malformed file → load error propagates |

Core crate tests stay as they are (`cargo test` at root).

### 10.2 Frontend tests

Optional for v1. If added, cover the key → `ActionDto` table in §3.6 (pure function, no DOM).

### 10.3 Manual parity checklist (macOS)

- [ ] Launch: a puzzle starts immediately at the default length; no start screen
- [ ] Toolbar: slider shows current length; release starts a puzzle of that length; custom word sets slider to its length
- [ ] Confirm abandon: New puzzle, Custom word, and slider on an unfinished puzzle show the dialog; Cancel changes nothing; no dialog after a win
- [ ] Playing: letters fill cursor slot; Backspace returns letter to pool; arrows move cursor
- [ ] Playing: pool shrinks on type and grows on delete; invariant holds
- [ ] Tab shows clues in order; header counter matches budget; clue budget cycles
- [ ] Ctrl+R resets same word with reshuffled pool
- [ ] Correct full answer → Won banner; wrong full answer → text feedback; backspace recovers
- [ ] Won: `n`, `i`, `r` behave per §3.6
- [ ] Custom word: valid word starts puzzle; invalid shows error; multiple matches show pick list; `u` and Esc behave per §3.6
- [ ] Workbench (pointer): hover highlight; click panel focuses; click pool tile then empty cell places; click filled cell swaps while holding; Clear returns all tiles to pool; Patterns panel visible
- [ ] Colors match config; changing a color key and restarting updates the UI
- [ ] Malformed config shows error dialog; missing config uses defaults
- [ ] Window resize: layout stays usable at minimum size; no clipped controls
- [ ] DevTools → Network/IPC inspection: no response contains the solution before Won
- [ ] First round with Patterns panel: acceptable latency (affix lexicon scan)

---

## 11. Open Questions

1. **Expose `solve` in the GUI?** Default: **no**. `solve` is a CLI tool that prints TSV to stdout.
2. **Add a `tui` Cargo feature to gate `ratatui` / `crossterm`?** Default: **yes, recommended**, but optional for v1. Gating would touch `ui`, `input`, `terminal`, and `lib::run`. Without it, `src-tauri` compiles the terminal stack transitively.
3. **Separate binary or second target?** Default: **separate `src-tauri` crate** (`anagram-desktop`). The `anagram` binary is untouched.
4. **Esc semantics.** The TUI quits from every non-play screen and quits from Custom pick. Default: **TUI parity**. Alternative: Esc goes back (Pick → Input).
5. **Candidate words in `CustomWordPick`.** Default: **send** them. The user typed the scramble, and the TUI shows the matches. This is not a round answer.
6. **Affix patterns during play.** They are derived from the lexicon and may reveal a prefix or suffix of the answer. Default: **send** (TUI parity; cue by design). Alternative: hide until Won.
7. **Final window minimum size.** Default: proposed 640 × 480, tune during manual testing (§7).
8. **Workbench layout.** Option A (free-form canvas) or option B (sparse 5×5 grid). Default: **B, proposed, pending decision** (§3.4.1).
9. **Workbench focus and answer keys.** Default (proposed): focus is a visual ring set by click and does not block answer keys. Alternative: a workbench click moves all key input to the grid.
10. **Solved-puzzle confirmation.** Should New puzzle, Custom word, and slider moves also confirm when the current puzzle is already won? Default: **no** (§3.1).
11. **Keyboard length and new-puzzle shortcuts.** The Size-screen keys (`s`, digits `4`–`9`, `0`) were removed with the start screen, and the slider has no keyboard binding. Should length selection or New puzzle get a keyboard shortcut during Playing? Default: **no** (pointer-only toolbar; §3.6 parity notes).
