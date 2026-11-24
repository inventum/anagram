# Letter Workbench (Spatial Cue Grid) — Product & Technical Specification

**Status:** Implemented  
**Related:** [SPEC.md](./SPEC.md) (core TUI, answer line & win), [SPEC_SIGNATURE_INDEX.md](./SPEC_SIGNATURE_INDEX.md) (lexicon), [RUNBOOK_LETTER_WORKBENCH.md](./RUNBOOK_LETTER_WORKBENCH.md) (implementation runbook v1.5)  
**Version:** 1.2

---

## 1. Overview

**Purpose:** Add an optional **letter workbench** — a small fixed **cell grid** (layout cue only, not a game board with rules) where the player can **rearrange letters in space** to explore prefixes, suffixes, and letter clusters. The **authoritative answer** remains the existing **answer row** (dashes + slots) driven by `Round` in `src/game.rs` (cursor-based slots, separate scrambled pool).

**Problem solved:** Many solvers think by **moving tiles on a desk**, not only typing into slots. The workbench supports that habit without changing multiset/win logic on the answer path.

**Metaphor:** “Tic-tac-toe **grid**” refers to **rows and columns of cells**, not turns, lines, or opponents.

---

## 2. Goals & Non-Goals

| Goals | Non-Goals |
|-------|-----------|
| Separate **grid pool** and **answer pool** (no shared inventory) | Grid state affects win check or `Round::check_solution` |
| Default **5×5** grid (configurable **4×4**); empty cells are normal | Partial-word dictionary lookup from grid layout (v2+) |
| **Focus toggle** between answer row and grid | Touch / mouse drag (terminal keyboard only in v1) |
| **Pick / place / swap** via **Space** on the grid | Typing `a`–`z` to fill grid cells (v1: Space only on grid focus) |
| Initialize / reset workbench with every **new round** | Persisting workbench layout across sessions |
| Available when `phase ∈ { Playing, Help, Won }` | Workbench on menu / size-select / custom-word screens |
| Unit-tested **workbench state machine** (pure Rust) | Grid letters committed into answer slots automatically |

---

## 3. Concepts

### 3.1 Two parallel tracks (normative)

At round start the solution word has **N** letters. The UI holds **two independent copies** of that multiset (**2N** letter instances total):

| Track | Pool | Slots | Count per track |
|-------|------|--------|-----------------|
| **Answer** | `Round::pool` | `Round::answer` | N instances |
| **Grid (cue)** | `LetterWorkbench::pool` | `LetterWorkbench::cells` + optional `held` | N instances |

| Pool | Owner | Used for |
|------|--------|----------|
| **Answer pool** | `Round::pool` | Filling answer slots (`Round` invariants in [SPEC.md](./SPEC.md) §3.2) |
| **Grid pool** | `LetterWorkbench::pool` | Placing letters into grid cells only |

**Rules:**

1. **Answer track:** Each instance is in exactly one of `Round::pool` or `Round::answer[i]` (unchanged from core game).
2. **Grid track:** Each instance is in exactly one of `grid.pool`, a grid cell, or `held` (at most one letter in hand).
3. **No cross-track transfer** — letters never move between answer and grid.
4. At round start, each pool is filled with **N** letters (same multiset as `solution`). **Independent shuffle** per pool (orders may differ).

### 3.2 Grid

- Fixed dimensions **ROWS × COLS** (v1 default **5×5** = 25 cells).
- Each cell: empty or one **uppercase** letter (matches `Round` display convention).
- **25 cells ≥ N** for bundled `max_size` 10; extra cells stay empty.
- Optional **4×4** when `workbench_cols = 4` in config, or auto when terminal height &lt; 28 rows (see §4.1).

### 3.3 Input focus (normative)

| Focus | Arrow keys | `a`–`z` | Backspace | Space | Tab |
|-------|------------|---------|-----------|-------|-----|
| **Answer** (default) | Move answer **cursor** (`Round::move_cursor`) | Type into **underlined** slot from **answer pool** | Clear slot under cursor; return letter to **answer pool** | *(unused)* | **Request clue** (existing binding) |
| **Grid** | Move **grid cursor** | **No-op** in v1 | Clear cell under cursor → append letter to **grid pool** | **Pick / place / swap** (§4.2) | **No-op** (clues require answer focus) |

Toggle focus: **`[g]`** only (do **not** use Tab; Tab is clue in answer focus).

On **Won:** answer typing is disabled (`map_navigation` phase, not `map_playing`). **Clue** keys are ignored by `reduce_request_clue` (no effect). **`[g]`** toggles focus (must be routed before navigation). **`[n]` / `[s]` / `[i]` / `[r]`** use navigation map as today.

### 3.4 Alignment with implemented core game

The shipped game does **not** use strict left-to-right linear fill. Implement against **`src/game.rs`**:

- Movable **cursor** on answer slots; `←` / `→` move cursor.
- Typing fills the **cursor** slot when empty and removes one matching letter from **answer pool**.
- Backspace clears the **cursor** slot (or steps left), returning the letter to **answer pool**.

Authoritative reference: [SPEC.md](./SPEC.md) §4.2 (aligned with `game.rs`).

---

## 4. User Experience

### 4.1 Layout (Playing)

Split `draw_play_screen` **horizontally**: workbench **left**, scrambled / answer board **right**.

```
┌─ Workbench (cue only) ────────────────────────┐
│ Grid pool:  [ L  I  S  T  E  N ]             │
│   .   .   .   .   .                         │
│   .   E   .   .   .                         │
│   ...                                       │
│ Focus: Grid   [g] toggle   Space: move tile │
└──────────────────────────────────────────────┘
```

| Terminal | Behavior |
|----------|----------|
| **≥ 80×28** | Full workbench (default 5×5) |
| **80×24–27** | 4×4 workbench or collapsed strip (implementation picks one; must not crash) |
| **`workbench = false`** in config | Hidden; no `GameState.workbench` |

### 4.2 Space semantics (grid focus only)

Let **cell** = letter under grid cursor, **hand** = `held`.

| hand | cell | grid pool | Action on Space |
|------|------|-----------|-----------------|
| empty | empty | non-empty | Remove **leftmost** pool letter → place in cell |
| empty | empty | empty | No-op |
| empty | filled | * | Cell letter → hand |
| filled | empty | * | Hand letter → cell; hand empty |
| filled | filled | * | Swap hand ↔ cell |

No separate “swap” key; swap is the last row.

### 4.3 Other grid keys

| Key | Effect (grid focus) |
|-----|---------------------|
| **Backspace** | If cell filled: letter → **end** of grid pool; if empty: no-op |
| **Ctrl+G** | `LetterWorkbench::reset_grid`: clear all cells, `held = None`, grid pool = full **N**-letter multiset (new shuffle) |
| **`[g]`** | Toggle answer ↔ grid focus (**either** focus; not grid-only) |

### 4.4 Answer path (unchanged)

- Clues, win detection, wrong-answer banner, custom-word flow: **unchanged**.
- Workbench is **ignored** by `auto_check_solution` and `SolveResult`.

### 4.5 Round lifecycle

| Event | Workbench |
|-------|-----------|
| `begin_round` / `Round::new` | `LetterWorkbench::from_solution(word, rows, cols, rng)` |
| `ResetRound` | Re-init workbench + answer pool (same word, new shuffles) |
| `NewPuzzle` / size change | New workbench |
| Custom word submit (`start_round_solution`) | New workbench (same as `begin_round`) |
| **Won** | Workbench visible; grid editable; answer track read-only |

---

## 5. State Model (Rust)

### 5.1 `LetterWorkbench` (new, e.g. `src/workbench.rs`)

```rust
pub struct LetterWorkbench {
    pub rows: u8,
    pub cols: u8,
    pub cells: Vec<Option<char>>,   // rows * cols, row-major
    pub pool: Vec<char>,
    pub cursor: usize,               // cell index
    pub held: Option<char>,
}
```

**Invariants (grid track only):**

- `count(cells) + pool.len() + held.count()` = **N**.
- Letters are ASCII alphabetic; stored uppercase (match `Round`).
- `cursor < cells.len()`.

**API (minimum):**

- `from_solution(word, rows, cols, rng) -> Self`
- `move_cursor(dx, dy) -> bool`
- `pick_place_swap() -> bool` — Space semantics (§4.2)
- `clear_cell() -> bool` — Backspace
- `reset_grid(rng)` — Ctrl+G

Pure logic only; no `Dictionary` / signature index.

### 5.2 `GameState` extension

```rust
pub enum InputFocus {
    Answer,
    Grid,
}
```

| Field | Type | Notes |
|-------|------|--------|
| `workbench` | `Option<LetterWorkbench>` | `None` if disabled or terminal too small |
| `input_focus` | `InputFocus` | Default `Answer` |

**Config** (flat keys in `config.toml`, same style as `min_size`):

```toml
workbench = true
workbench_rows = 5
workbench_cols = 5
```

Defaults: `workbench = true`, `workbench_rows = 5`, `workbench_cols = 5`.  
v1 presets: **5×5** or **4×4** only — set **both** `workbench_rows` and `workbench_cols` to the same value (4 or 5); other combinations are undefined in v1.

### 5.3 Actions & input routing

| `Action` | Binding | When |
|----------|---------|------|
| `ToggleWorkbenchFocus` | `[g]` | Playing, Help, Won (if workbench present) |
| `WorkbenchMove(dx, dy)` | arrows | Grid focus + same phases |
| `WorkbenchPickPlace` | Space | Grid focus |
| `WorkbenchClearCell` | Backspace | Grid focus |
| `WorkbenchResetGrid` | Ctrl+G | Grid focus |

**Routing rule:** In `input.rs`, treat **`Playing | Help | Won`** as one bucket for:

- Existing **answer** bindings when `input_focus == Answer` (Won: no typing, no clue; arrows may move cursor read-only optional).
- **Workbench** bindings when `input_focus == Grid`.

Do not rely on `map_navigation` for workbench keys on **Won** (today `i` / `n` stay on navigation map; workbench is separate).

---

## 6. UI (`src/ui.rs`)

- `draw_workbench(...)` below answer board when `workbench` is `Some`.
- Grid pool label: **“Grid pool (cue only)”** — distinct from **“Scrambled — pick from these letters”** (answer pool).
- Grid cursor: reverse video; empty cells: `.` dimmed.
- Footer when playing: `Focus: Answer | Grid` and **`[g]`** hint (extend `footer_line`).

---

## 7. Testing

| Layer | Tests |
|-------|--------|
| `workbench.rs` | Invariant after Space/clear/reset; cursor bounds; 4×4 / 5×5 |
| `input.rs` | `[g]` toggles focus; Tab still clue in answer focus; Space no-op in answer focus |
| `app.rs` | New puzzle / reset re-inits workbench; grid edits do not change `Round::pool` |

**Scenario:** 6-letter word; place 3 letters on grid; fill answer from **answer pool only**; win iff answer correct.

---

## 8. Implementation Phases

| Phase | Deliverable | Est. |
|-------|-------------|------|
| **W0** | Spec v1.2 + config fields | — |
| **W1** | `LetterWorkbench` + unit tests | 0.5 d |
| **W2** | Draw grid + pool; `[g]` focus (no Space yet) | 0.5 d |
| **W3** | §4.2 Space + Backspace + Ctrl+G + input routing | 1 d |
| **W4** | Terminal height / 4×4 degrade, footer, help | 0.5 d |

**Total:** ~2–3 days.

---

## 9. Resolved Decisions (was open in v1.0)

| Topic | Decision |
|-------|----------|
| Pick from grid pool | **Space** on empty cell with empty hand pulls **leftmost** pool letter into cell (§4.2) |
| Tab vs focus | **Tab = clue** only; **`[g]` = focus** only |
| Won + workbench | Show grid; answer track read-only; grid still editable |
| Letter keys on grid | **No-op** in v1 (Space-only tile motion) |

---

## 10. Changelog

| Version | Date | Notes |
|---------|------|--------|
| 1.0 | 2026-11-14 | Initial draft |
| 1.1 | 2026-11-14 | Dual-track invariants; Space state machine; cursor answer alignment; config/input/Won routing; closed open questions |
| 1.2 | 2026-11-14 | Stale cross-refs; Won/clue routing; `[g]` toggle; 4×4 config; custom-word lifecycle |