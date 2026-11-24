# Anagram Desktop UI — Product & Technical Specification (Tauri)

## 1. Overview & Status

**Status:** Draft (v1 scope)  
**Product name (working):** Anagram Desktop  
**Platform:** macOS first (see §8)  
**Implementation:** Tauri v2 desktop app — Rust backend over the existing `anagram` core crate, TypeScript frontend  
**Goal:** Offer the terminal game's experience in a resizable desktop window. The Rust game core is reused; the GUI adds presentation and input only.

**Relationship to the TUI:** The terminal app ([SPEC.md](SPEC.md)) remains supported and its behavior is unchanged. The GUI is a second front end over the same terminal-independent modules (`app`, `game`, `dictionary`, `signature_index`, `workbench`, `affix_hints`, `solve`, `cli`, `config`).

**Shared rules:** Puzzle generation, the letter-pool/answer invariant, win check, clue order, and per-round clue budget are defined in [SPEC.md](SPEC.md) §3–§4.4 and [SPEC_LETTER_WORKBENCH.md](SPEC_LETTER_WORKBENCH.md). They are not restated here.

**Controls source of truth:** `src/input.rs`, except the intentional desktop deviations in §3.6 (Esc, ⌘Q, dropped TUI bindings, pointer-only workbench). Where [SPEC.md](SPEC.md) §4.4–§4.5 differs (for example, `r` and `c` are typed letters during play; reset during play is Ctrl+R), this spec governs.

**Visual reference:** [wireframe-ui.html](wireframe-ui.html). Fig. 1 play window (toolbar, board, workbench); Fig. 2a wrong answer; Fig. 2b custom word pick; Fig. 2c won; Fig. 2d help overlay; Fig. 2e confirm abandon. The wireframe must match this spec. It has no start or size screen.

---

## 2. Goals & Non-Goals

| Goals | Non-Goals |
|-------|-----------|
| Feature parity with TUI v1 gameplay (§3) | Mobile (iOS / Android) |
| Reuse the `anagram` core crate with no change to game rules | Web or hosted deployment |
| One source of game logic: Rust `app::reduce` | Multiplayer / networking |
| Keyboard controls matching TUI bindings where they do not conflict with the toolbar and pointer design (§3.6) | Changing game rules |
| Responsive layout in a resizable window | Terminal-size behavior (80×24, resize reflow) |
| Answer never sent to the UI before a win (§5) | Reducing word data size (≈37 MB embedded list accepted as-is) |
| Config file shared with the TUI (§6) | Accounts, timers, leaderboards |

---

## 3. Feature Parity with TUI

### 3.1 Puzzle sizes

- Word length N comes from config `min_size` / `max_size` (default 4–10).
- **No start screen.** Launch starts a random puzzle at `config.clamp_size(5)` (length 5 unless the config bounds exclude it; §4.3). Length selection and new-puzzle start live in the main play window.
- **Top toolbar** (a row at the top of the play panel, above the board):
  - **Length slider** from `min_size` to `max_size` (default 4–10), showing the current puzzle length as a numeric value.
  - **Custom word** button.
  - **New puzzle** button.
  - The board, pool, answer row, clue, workbench, and footer stay below the toolbar.
- **Slider semantics.** The slider always reflects the current puzzle's length. A custom word sets the slider to that word's length. Moving the slider starts a new random puzzle of the chosen length. The new puzzle starts on pointer release; if the user cancels the confirmation, the slider returns to the current length.
- **Custom word** (toolbar button; no keyboard shortcut during Playing, see §3.6): same validation (letters A–Z, length within bounds), same scramble resolution via `SignatureIndex`, and the same pick list when several words match. The desktop shows the pick-list status text from `SubmitCustomWord`, never the `start_custom_word` text ("pick one in the TUI"), which is CLI-only.
- **Puzzle replacement rule (confirmation).** Any action that replaces the current puzzle shows a confirmation dialog **when the current puzzle is unfinished** (not yet won):
  - Actions: **New puzzle**, **Custom word** (button press), and **slider move** (on release).
  - Dialog: "Confirm abandon?" with **Cancel** (no change) and **Abandon** (proceed). Esc = Cancel.
  - **Won puzzle:** no dialog; the action proceeds immediately.
  - The check reads `UiState.phase` (not `won`) in the frontend. This is presentation-only input gating; the reducer is unchanged.
- **Cancel after abandon (core constraint).** `reduce_open_custom_word` leaves the current round in place, and `reduce_cancel_custom_word` returns to `Menu`, which the GUI never shows. The desktop therefore runs `CancelCustomWord` followed by `NewPuzzle` (same size) inside one backend command. Cancel (button, Esc, or `u`) always starts a new random puzzle at the current length. The abandoned puzzle is not restored. While custom input or pick is open, the DTO hides the round (§5.5).

### 3.2 Gameplay

- Answer slots, scrambled pool that shrinks and grows, cursor-based fill, auto-check when all slots are full, and a Won banner. Rules: [SPEC.md](SPEC.md) §3.2–§3.4.
- Reset (same word, reshuffled pool) clears answer and clue state.

### 3.3 Clues

- The active clue is shown in a clue line. The clue budget is `Dictionary::clue_budget_round`; the header shows `used/total`.
- The next clue is requested with **Tab** during play (see §3.6 parity notes).
- Clue kinds and order come from `Dictionary::clues_for_round` (core, unchanged). The first-letter clue reveals a letter and preserves the pool/answer invariant.

### 3.4 Letter workbench and affix hints

- Enabled by config `workbench` (default `true`). Grid size is `workbench_rows` × `workbench_cols` via `AppConfig::workbench_settings()`. A 4×4 or 5×5 grid is used as configured. Any other combination (including mismatched rows and cols) becomes 5×5 (§6).
- **Pointer-only.** The workbench has no keyboard bindings: no Ctrl+G, no Ctrl+Shift+G, no grid focus mode, and no Space, arrows, letters, or Backspace in the grid. Hover highlights, click acts, and drag is optional. Clicking the workbench sets a visual focus ring only; answer keys always go to the answer row. Pointer table in §3.6.
- The workbench has **its own letter pool** (`WorkbenchDto.pool`), separate from the round pool (`RoundDto.pool`). Both start as independent shuffles of the solution letters. Moving letters in one never changes the other.
- The workbench is **cue only**: it never changes answer or win state.
- **Patterns (cue)** panel: prefix and suffix patterns (`affix_hints::affix_hints(&Round)` → `starts` / `ends`), shown inside the workbench panel.
- The pattern set depends only on the letter multiset of pool ∪ answer, which is invariant within a round. **The backend computes it once per round and caches it** (§4.3). The TUI recomputes it per frame; the GUI must not.

#### 3.4.1 Workbench layout — pending decision

Two options are under consideration. **Neither is final.** This is the only place the layouts are described; other sections refer here.

- **Option A — free-form canvas.** An empty canvas. Letters can be placed anywhere (free x, y). No fixed cells.
- **Option B — sparse grid (proposed).** A fixed grid, mostly empty: 5×5 by default, or 4×4 when config sets it (§3.4, §6). Tiles can move to any empty cell, up, down, or across.
  - Cells keep identity: each cell has a fixed index and holds at most one tile. No overlap.
  - Placing a tile on an occupied cell swaps, as in the TUI.
  - B uses the configured size. The wireframe shows the 5×5 case.
  - **Clear** button returns all tiles (cells and hand) to the workbench pool and reshuffles it.

**Recommendation:** B — *proposed, pending decision* (§11 Q5).

Trade-offs:

- **A** is more expressive for spatial arrangement. It needs a new model (positions, not cells), so `LetterWorkbench` cannot be reused unchanged. It also needs overlap rules and a bounded or scrolling canvas.
- **B** reuses the existing `LetterWorkbench` model (`rows`, `cols`, `cells`, `pool`, `held`), so the core stays unchanged. Hit-testing and drag-to-snap are simple. The cost is less freedom and a sparse grid that may look empty.
- Both options keep the cue-only rule: layout never affects answer or win state.
- The layout choice does not change the affix (Patterns) panel.

### 3.5 Theme colors

Four config keys set the colors of empty answer slots, filled answer letters, the scrambled pool, and correct-position highlights. Translation to CSS is defined in §6.

### 3.6 Keyboard controls

Authoritative source: `src/input.rs`, with the desktop deviations below. Keys are case-insensitive for letters.

**Global**

| Key | Action | Where |
|-----|--------|-------|
| Esc | Close the open overlay, dialog, or custom-word input: Help → close; Custom word input or pick → Cancel (§3.1); Confirm dialog → Cancel. With nothing open, no-op. Esc never quits. | All phases |
| ⌘Q | Quit (close window). macOS only (§8). | All phases |

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
| Click workbench panel | Visual focus ring on the workbench (`focus = grid`); answer keys unaffected |
| Click answer row | Visual focus ring on the answer (`focus = answer`) |
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
| Esc | Cancel: new random puzzle at the current length (§3.1) |

**Custom word pick**

| Key | Action |
|-----|--------|
| `↑` / `↓` or `k` / `j` | Move highlight |
| Enter | Start puzzle with highlighted word |
| Click a row | Highlight that row (`customPickSelect`); does not start the puzzle |
| `u` | Cancel: new random puzzle at the current length (§3.1) |
| Esc | Cancel, same as `u` |

**Confirm abandon** (dialog; shown only per the §3.1 rule): **Cancel** and **Abandon** buttons. Esc = Cancel. Abandon has no key binding (§11 Q8).

**Intentionally dropped TUI bindings** (the desktop app binds none of these):

- `Esc` as quit (all phases, including custom pick) and `Ctrl+C` as quit. Quit is ⌘Q only.
- `s` (Size screen), digits `4`–`9` and `0`, and `s` in Won. Length is set by the slider only.
- `c` as the clue key. Tab is the only clue key.
- `Ctrl+G`, `Ctrl+Shift+G`, Space, and arrows, letters, or Backspace in the workbench grid (grid focus mode). The workbench is pointer-only.
- `[g]` (workbench toggle, [SPEC.md](SPEC.md) §4.2). No desktop binding.

**Parity notes**

- During Playing / Help, `n`, `i`, `s`, `r`, `c`, and `q` are letters. New puzzle, Custom word, length, and reset therefore have no bare-letter shortcut there. Reset is Ctrl+R; New puzzle, Custom word, and the slider are toolbar-only.
- Tab is the only effective clue key. `c` reaches `RequestClue` only in Won, where the reducer ignores it.
- Won keeps `n`, `i`, `r`, and `?`. Ctrl+R in Won also resets, as in the TUI, because the TUI's navigation handler ignores modifiers.
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

- Root `Cargo.toml` adds `[workspace] members = [".", "src-tauri"]` and `default-members = ["."]`. Plain `cargo test` and `make test` therefore keep covering only the TUI crate. Use `cargo test -p anagram-desktop` for the GUI crate (or `cargo test --workspace` for both).
- `src-tauri` depends on `anagram = { path = ".." }`. There is one `Cargo.lock`, at the root.
- The root crate keeps `rust-version = "1.75"`. Tauri v2 needs a newer toolchain. `src-tauri` declares its own `rust-version`, matching the pinned Tauri release (verify at implementation time).

### 4.3 Backend (`src-tauri`)

**Startup (`setup`)**

1. `AppConfig::load(None)`. On error, show a native error dialog and exit (matches TUI, which fails on bad config).
2. `Dictionary::from_embedded()` and `SignatureIndex::from_embedded()`. Both are read-only, so they are managed as `Arc` with **no lock**. Loaded once.
3. Initial state, as in `lib.rs::run_play`: `GameState::with_bounds(config.clamp_size(5), config.min_size, config.max_size)` with `workbench_settings = config.workbench_settings()`. Unlike the TUI, setup then runs `NewPuzzle` so the first `UiState` is in Playing. No Menu phase is exposed.
4. `StdRng::from_entropy()`.
5. Managed as `Mutex<Session>`, where `Session { state: GameState, rng: StdRng, affix_cache: Option<(String, AffixHints)> }`. The cache is keyed by the internal solution string, which is never serialized.

**Commands** (the only two)

| Command | Behavior |
|---------|----------|
| `dispatch(action: ActionDto) -> UiState` | Sole mutation entry point. Locks the session, converts to `app::Action`, calls `app::reduce`, rebuilds `UiState`, returns it. `Quit` is never sent to `reduce`; the shell closes the window. `cancelCustomWord` runs `CancelCustomWord` then `NewPuzzle` under the same lock (§3.1), so Menu is never returned. |
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
3. Candidate words in `CustomWordPick` are sent (§11 Q3).
4. Affix patterns are sent during play (§11 Q4).

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
    pub help_open: bool,          // overlay visible; `?` or Esc closes it (§3.6)
    pub length: u8,               // current puzzle length (slider value)
    pub min_size: u8,             // config min_size (slider minimum)
    pub max_size: u8,             // config max_size (slider maximum)
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
    pub focus: FocusDto,              // answer | grid; visual ring only (§3.4)
    pub affixes: AffixDto,            // { starts, ends } = Prefix / Suffix, cached per round
}
```

**Wireframe mapping** (names must match the sketch above):

| Wireframe element | Field |
|-------------------|-------|
| Toolbar slider and value (Fig. 1) | `length`, `min_size`, `max_size` |
| Title "ANAGRAM · 6 letters" | `length` |
| Clue counter: "Clues 1/3" (Fig. 1), "Clues used 2 of 3" (Fig. 2c) | `clues.used`, `clues.budget` |
| Clue text | `active_clue` |
| Scrambled pool | `round.pool` |
| Answer slots and cursor | `round.answer`, `round.cursor` |
| Status line | `status.kind`, `status.message` |
| Wrong-answer slots (Fig. 2a) | `round.answer` with `status.kind == wrongAnswer` |
| Workbench pool | `workbench.pool` |
| Workbench grid cells | `workbench.cells`, `workbench.rows`, `workbench.cols` |
| Held tile | `workbench.held` |
| Prefix / Suffix (Patterns) | `workbench.affixes.starts` / `.ends` |
| Custom word scramble field (Fig. 2b) | `custom.input` |
| Pick rows and highlight (Fig. 2b) | `custom.choices`, `custom.pick` |
| Solved slots and solution (Fig. 2c) | `round.answer` (all green when `phase == won`), `round.solution` |
| Help overlay (Fig. 2d) | `help_open` |
| Confirm dialog (Fig. 2e) | none; frontend-only state |

### 5.4 `ActionDto`

- Serde tagged enum: `#[serde(tag = "type", rename_all = "camelCase")]`.
- Keyboard and toolbar variants map 1:1 to `app::Action`:
  - `toggleHelp`, `newPuzzle`, `resetRound`, `requestClue`, `backspace`, `cursorLeft`, `cursorRight`, `typeLetter { ch }`
  - `selectSize { n }` (slider release)
  - `openCustomWord`, `customWordType { ch }`, `customWordBackspace`, `submitCustomWord`
  - `cancelCustomWord` (the backend also runs `newPuzzle`, §3.1)
  - `customPickDelta { delta }`, `customPickSelect { idx }` (click row; no `app::Action` equivalent yet), `confirmCustomPick`
- Workbench pointer variants (no `app::Action` equivalent yet): `workbenchClickCell { idx }`, `workbenchClickPool { idx }`, `workbenchSetFocus { focus }`, `workbenchClear` (maps to `WorkbenchResetGrid`). Layout A replaces `idx` with `{ x, y }` (§3.4.1).
- Not in `ActionDto`: `quit` (shell); `openSizeSelect` / `cancelSizeSelect` (no size screen); `toggleWorkbenchFocus`, `workbenchMove`, `workbenchPickPlace`, `workbenchClearCell`, `workbenchTypeLetter` (TUI keyboard-only bindings, §3.6).
- **Confirm flow:** the dialog is frontend-only. Abandon dispatches the pending action (`newPuzzle`, `openCustomWord`, or `selectSize { n }`). Cancel and Esc dispatch nothing; the slider returns to `UiState.length`.
- Example: `{ "type": "typeLetter", "ch": "a" }`.
- Conversion `TryFrom<ActionDto> for app::Action` rejects non-ASCII-alphabetic characters in letter variants. `reduce` also guards.

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
| `workbench_rows` | u8 | 5 | Grid rows: 4 or 5. If rows ≠ cols, or either is not 4 or 5, both become 5 (`workbench_settings()`) |
| `workbench_cols` | u8 | 5 | Grid columns (see `workbench_rows`) |

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
  - Board (pool, answer row, status) sits in the left play column. At content width ≥ 880 px, the clue panel is the right column (about 280 px) beside the board.
  - The letter workbench is a full-width panel below the board and clue, above the footer. Its pool row sits on top; below it the 5×5 grid (at most 260 px square) is on the left and Hand, then Prefix/Suffix patterns, on the right. Below about 520 px panel width, the grid stacks above the side column.
  - Below 880 px, the board, clue, workbench, and footer stack in one column.
  - Tile size clamps with viewport width (about 40–56 px) so letter rows fit at 640 px.
  - Pool letters wrap. Clue text wraps.
  - Footer key hints sit at the very bottom of the window.
  - Help is a modal overlay that lists the desktop key table (§3.6). Close it with `?`, Esc, or ×.
- **Status text:** errors and wrong-answer feedback use text, not color alone (matching the TUI accessibility rule).

---

## 8. Platforms

- **macOS first.** This is the only platform with manual test coverage in v1.
- **Quit:** ⌘Q on macOS. Other platforms use their native quit or close convention; not tested in v1.
- Linux and Windows are not blocked by design. Avoid platform-specific APIs; use `std::env::home_dir` through the existing config path. They are not tested in v1.

---

## 9. Build & Run

**Prerequisites:** Rust toolchain (version per §4.2), Node.js LTS, Tauri CLI v2 (`cargo install tauri-cli --version "^2"`).

| Task | Command |
|------|---------|
| Dev run (window + Vite HMR) | `cargo tauri dev` (from repo root) |
| Release bundle (macOS `.app`) | `cargo tauri build` |
| Frontend only | `npm --prefix ui-web run dev` / `npm --prefix ui-web run build` |
| TUI tests (unchanged) | `cargo test` (root, via `default-members`; also `make test`) |
| GUI Rust tests | `cargo test -p anagram-desktop` |

`cargo tauri dev` and `build` invoke the `ui-web` npm scripts through `tauri.conf.json` (`beforeDevCommand` / `beforeBuildCommand`).

`make` targets for GUI build and run are **to be added later**.

---

## 10. Testing

### 10.1 Rust unit tests (`src-tauri`)

| Area | Tests |
|------|-------|
| DTO hiding | For Custom input, Playing, Help: serialized `UiState` contains no `solution` key and no string equal to the solution. Custom pick is excluded: candidates include the solution by design (§5.5). |
| DTO win exposure | In Won, `round.solution` equals the solution |
| Correctness flags | `correct` is `None` unless highlight is on; when on, flags match `slot_is_correct` |
| Alternates hidden | Round with alternates: no alternate word appears in any serialized state outside `custom.choices` during Pick |
| ActionDto mapping | Every keyboard and toolbar `ActionDto` variant maps to its `app::Action`; `cancelCustomWord` yields `CancelCustomWord` then `NewPuzzle`; non-letter `ch` is rejected |
| Reducer integration | Fixed-seed sequence of `dispatch` calls produces the same state as calling `app::reduce` directly |
| Affix cache | Same round → cached result reused; new round → recomputed |
| Theme mapping | All 16 names map to expected CSS; unknown name → magenta |
| Config | Missing file → defaults; malformed file → load error propagates; invalid workbench dims → 5×5 |

Core crate tests stay as they are (`cargo test` at root).

### 10.2 Frontend tests

Optional for v1. If added, cover the key → `ActionDto` table in §3.6 (pure function, no DOM).

### 10.3 Manual parity checklist (macOS)

- [ ] Launch: a random puzzle starts immediately at the default length; no start screen
- [ ] Toolbar: slider shows current length; release starts a puzzle of that length; custom word sets slider to its length
- [ ] Confirm abandon: New puzzle, Custom word, and slider on an unfinished puzzle show the dialog; Cancel changes nothing; no dialog after a win
- [ ] Esc: closes Help, custom input, and custom pick (as Cancel); Cancel in the confirm dialog; no-op in Playing and Won; never quits; ⌘Q quits
- [ ] Custom word Cancel (button, Esc, or `u`) starts a random puzzle at the current length; the Menu is never shown
- [ ] Playing: letters fill cursor slot; Backspace returns letter to pool; arrows move cursor
- [ ] Playing: pool shrinks on type and grows on delete; invariant holds
- [ ] Tab shows clues in order; header counter matches budget; clue budget cycles
- [ ] Ctrl+R resets same word with reshuffled pool
- [ ] Correct full answer → Won banner; wrong full answer → text feedback; backspace recovers
- [ ] Won: `n`, `i`, `r` behave per §3.6
- [ ] Custom word: valid word starts puzzle; invalid shows error; multiple matches show pick list; `u` and Esc cancel per §3.6
- [ ] Workbench (pointer): hover highlight; click pool tile then empty cell places; click filled cell swaps while holding; Clear returns all tiles to pool; Patterns panel visible; no keyboard response in the workbench
- [ ] Colors match config; changing a color key and restarting updates the UI
- [ ] Malformed config shows error dialog; missing config uses defaults
- [ ] Window resize: layout stays usable at minimum size; no clipped controls
- [ ] DevTools → Network/IPC inspection: before Won, no response contains the `solution` field or the solution string, except `custom.choices` during Pick
- [ ] First round with Patterns panel: acceptable latency (affix lexicon scan)

---

## 11. Open Questions

Each question states the default in use. Resolved items were removed (see §4.1 for the separate crate, §3.6 for Esc and workbench keys, §7 for window size).

1. **Expose `solve` in the GUI?** Default: **no**. `solve` is a CLI tool that prints TSV to stdout.
2. **Add a `tui` Cargo feature to gate `ratatui` / `crossterm`?** Default: **yes, recommended**, but optional for v1. Gating would touch `ui`, `input`, `terminal`, and `lib::run`. Without it, `src-tauri` compiles the terminal stack transitively.
3. **Candidate words in `CustomWordPick`.** Default: **send** them (§5.1 rule 3). The user typed the scramble, and the TUI shows the matches. This is not a round answer.
4. **Affix patterns during play.** They are derived from the lexicon and may reveal a prefix or suffix of the answer. Default: **send** (§5.1 rule 4; TUI parity, cue by design). Alternative: hide until Won.
5. **Workbench layout.** Option A (free-form canvas) or option B (sparse grid). Default: **B, proposed, pending decision** (§3.4.1).
6. **Solved-puzzle confirmation.** Should New puzzle, Custom word, and slider moves also confirm when the current puzzle is already won? Default: **no** (§3.1).
7. **Keyboard length and new-puzzle shortcuts.** The Size-screen keys (`s`, digits `4`–`9`, `0`) were removed with the start screen, and the slider has no keyboard binding. Should length selection or New puzzle get a keyboard shortcut during Playing? Default: **no** (pointer-only toolbar; §3.6 parity notes).
8. **Confirm dialog keyboard.** Abandon is a button with no key binding; Esc = Cancel. Should Enter or Tab reach Abandon? Default: **no** (keyboard users can cancel but not abandon; §3.6).
