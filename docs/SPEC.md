# Anagrams TUI — Product & Technical Specification

## 1. Overview

**Product name (working):** Anagram TUI  
**Platform:** Terminal (macOS, Linux, Windows via modern terminal emulators)  
**Implementation:** Rust CLI binary with a fullscreen TUI  
**Goal:** Let users solve anagram puzzles by moving letters from a scrambled pool into answer slots, with configurable word length and optional clues. No time pressure.

**Related (optional follow-on):** [SPEC_LETTER_WORKBENCH.md](./SPEC_LETTER_WORKBENCH.md) — spatial **cue grid** with a separate grid pool; does not change answer or win semantics.

---

## 2. Goals & Non-Goals

| Goals | Non-Goals |
|-------|-----------|
| Clear visual link between scrambled letters and answer dashes | Multiplayer / networking |
| Letter pool shrinks/grows as user types/deletes | Timed modes or leaderboards (v1) |
| User picks puzzle size (letter count) | Full dictionary validation of every partial guess (v2+) |
| Optional clues | Graphical UI |
| Works in standard 80×24+ terminals | Account/login |
| Unit-tested core game logic (see §6.6) | End-to-end terminal integration tests (v1) |

---

## 3. Core Gameplay

### 3.1 Puzzle generation

1. User selects **word length** N (default **4–10** via `config.toml` `min_size` / `max_size`).
2. App picks a word of exactly N letters from a curated word list (locale-aware, e.g. `en-US`).
3. Letters are **shuffled** into a **scrambled pool** (multiplicity preserved; duplicate letters shown separately).
4. **Answer row** shows N slots, initially empty (displayed as dashes or blank cells).

### 3.2 Letter movement (primary interaction)

| User action | Scrambled pool | Answer slots |
|-------------|----------------|--------------|
| Types a letter that exists in the pool | One instance of that letter is **removed** from the pool (pool visually **shrinks**) | Letter fills the **cursor** slot if empty (see §4.2) |
| Backspace / Delete | Clears the **cursor** slot (or steps cursor left when empty) | Letter **returned** to the pool (appended) |
| Arrow keys | — | Move cursor between slots (`←` / `→`) |

**Invariant:** At all times, multiset of letters in pool ∪ answer = multiset of letters in the solution word (no letters created or destroyed).

### 3.3 Pool presentation

- **Shrink/grow:** Re-render pool with fewer/more glyphs; optional compact layout (e.g. `[ A C E R T ]` → `[ A C E R ]`).
- **Order on return:** Letters removed from the answer via backspace are **appended at the end** of the pool (predictable undo behavior).
- **Case:** Display uppercase; input accepts either case.

### 3.4 Winning and submission

- **Cursor fill:** User moves cursor; typing places a pool letter in the focused slot when empty.
- **Auto-check:** When all N slots are filled, compare answer to solution immediately (case-insensitive). No separate Submit key in v1.
- **Correct:** success message; option to play again (same size / new size).
- **Incorrect:** gentle feedback; user edits via backspace without breaking pool/answer invariants.

### 3.5 No time limit

- No countdown, no auto-fail on idle.
- Optional idle hint nudge is out of scope for v1.

---

## 4. Terminal UI

### 4.1 Layout (example)

```
┌─ Anagram ─────────────────────────────────────┐
│ Length: 6          Clues: 1/3 used            │
├─────────────────────────────────────────────┤
│ Scrambled:  [ R  E  C  A  T  E ]              │
│ Answer:     _  _  _  _  _  _                  │
│               ↑ cursor (underline)            │
├─────────────────────────────────────────────┤
│ Clue: A place where you might buy books.      │
├─────────────────────────────────────────────┤
│ [n] New  [s] Size  Tab Clue  Esc Quit         │
└─────────────────────────────────────────────┘
```

Minimum terminal: **80 columns × 24 rows**; degrade gracefully (wrap clue text).

### 4.2 Focus & cursor

- **Answer row** is the input focus for printable characters and backspace (unless [letter workbench](./SPEC_LETTER_WORKBENCH.md) grid focus is active).
- **v1 interaction model:** **movable cursor** — `←` / `→` select a slot; typing fills that slot when empty; backspace clears the cursor slot and returns the letter to the pool.
- **Optional letter workbench:** separate grid pool and cue grid; toggle with `[g]`; does not change answer/win rules ([SPEC_LETTER_WORKBENCH.md](./SPEC_LETTER_WORKBENCH.md)).

### 4.3 Size selection

**Flow:**

1. Main menu or in-game `[s]` → **Size** screen.
2. Presets: 4, 5, 6, 7, 8, 9, 10 + custom (within min/max).
3. Confirm → generate new puzzle; reset pool, answer, clue usage for that round.

**Persistence (optional v1.1):** remember last chosen size in config file.

### 4.4 Clues

| Clue type | Behavior |
|-----------|----------|
| **Definition** | WordNet gloss(es); one clue step per `noun_senses` entry when present, else single `definition` |
| **Correct positions** | Highlights answer slots that already match the solution (no letters moved) |
| **First letter** | Fills slot 0 with the solution’s first letter; removes that letter from pool if still present |

Optional serialized clues on `WordEntry` (`vowel_count`, `category`, …) override the generated list when present.

**UX:**

- **`Tab`** or **`[c]`** requests the next clue in order.
- Budget = `Dictionary::clue_budget` (definitions + 2 helpers); header shows `Clues: used/total`.
- Letter-revealing clues must preserve §3.2 pool ∪ answer invariant.

**Content:** Bundled `data/words.json`: `word`, `definition`, optional `noun_senses`, optional `clues[]`.

### 4.5 Global commands

| Key | Action |
|-----|--------|
| Printable `a-z` | Place letter from pool into cursor slot when empty |
| `←` / `→` | Move answer cursor |
| Backspace | Clear cursor slot; return letter to pool |
| `n` | New puzzle (same size) |
| `s` | Change size |
| `Tab` / `c` | Next clue |
| `r` | Reset puzzle (re-shuffle same word, clear answer) |
| `i` | Custom letters (menu or won screen) |
| `Esc` / ⌘Q / Ctrl+C | Quit |
| `?` | Help overlay |

---

## 5. Data & Content

### 5.1 Word list

- Plain text or JSON: `word`, optional `difficulty`, `clues`.
- Filter by length N at runtime.
- Exclude offensive words; optional allow-list for kids mode.

### 5.2 Shuffling

- Fair shuffle of letter indices; avoid identity shuffle when the word has more than one distinct letter.

---

## 6. Technical Architecture

### 6.1 Language & libraries

| Layer | Choice |
|-------|--------|
| **Language** | **Rust** (edition 2021; MSRV 1.75+) |
| **TUI** | [**ratatui**](https://github.com/ratatui-org/ratatui) — widgets, layout, styling, event loop |
| **Terminal I/O** | [**crossterm**](https://github.com/crossterm-rs/crossterm) — raw mode, keyboard, resize, color (ratatui backend) |
| **CLI** | [**clap**](https://github.com/clap-rs/clap) — `--help`, optional `--plain` debug mode, config path overrides |
| **Config file** | [**toml**](https://github.com/toml-rs/toml) + [**serde**](https://serde.rs/) — parse `~/.config/anagram/config.toml` |
| **Word list** | [**serde**](https://serde.rs/) + [**serde_json**](https://github.com/serde-rs/json) — bundled `words.json` (or equivalent) |
| **Shuffle** | [**rand**](https://github.com/rust-random/rand) — fair letter permutation per round |
| **Errors** | [**anyhow**](https://github.com/dtolnay/anyhow) — top-level error reporting; restore terminal on exit |

**Rationale:** Rust + ratatui/crossterm is a common, maintained stack for interactive terminals: predictable performance, portable key/resize handling, and clean shutdown so the shell is never left in raw mode.

**Cargo manifest (v1 dependencies):**

```toml
[dependencies]
ratatui = { version = "0.29", default-features = false, features = ["crossterm"] }
crossterm = "0.28"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
rand = "0.8"
anyhow = "1"
```

**Project layout (reference):**

```
src/main.rs          # entry, terminal setup/teardown, run loop
src/app.rs           # GameState, input → actions, phase transitions (+ unit tests for pure logic)
src/ui.rs            # ratatui draw: pool, answer, clues, footer
src/game.rs          # shuffle, pool/answer invariants, win check (+ unit tests)
src/dictionary.rs    # load words by length, clue metadata (+ unit tests)
data/words.json      # embedded or shipped beside binary
data/signature_index.json   # derived; optional until signature index ships (see SPEC_SIGNATURE_INDEX.md)
```

**Derived lexical index (draft):** Letter-multiset → words + meanings, built once at data-pipeline time, O(1) scramble lookup. See [SPEC_SIGNATURE_INDEX.md](./SPEC_SIGNATURE_INDEX.md) and [RUNBOOK_SIGNATURE_INDEX.md](./RUNBOOK_SIGNATURE_INDEX.md).

### 6.2 Application state

| Concern | Implementation |
|---------|----------------|
| State | `GameState`: `Round` (pool, answer, cursor), `size`, `clues_used`, `phase`, optional workbench (see SPEC_LETTER_WORKBENCH) |
| Config | `~/.config/anagram/config.toml` — locale, min/max size, color theme |

### 6.3 State machine

```
Menu ──► SizeSelect ──► Playing ◄──► Help
  │           │              │
  │           │              ├──► Won ──► (n/new, i/custom, s/size)
  │           │              └── Quit
  ├──► CustomWordInput ──► Playing (or CustomWordPick ──► Playing)
  └──► CustomWordInput via [i] from Menu / Won
```

### 6.4 Rendering

- Full redraw on each input event (TUI standard).
- Styles: scrambled pool (accent), dashes (muted), filled slots (bold), status bar (dim).

### 6.5 Input handling

- Raw mode; map key events to actions.
- Ignore typed letters not present in the pool (optional beep/flash).
- Handle terminal resize: reflow clue text, avoid corrupt layout.

### 6.6 Unit tests

v1 **must** include automated unit tests. They are a shipping requirement, not optional polish.

| Requirement | Detail |
|-------------|--------|
| Runner | Rust built-in test harness: `cargo test` (see `#[test]`, `#[cfg(test)]` modules) |
| CI / local gate | All unit tests pass before a release or merge |
| Scope | **Pure logic** without a real terminal: `game`, `dictionary`, and any deterministic `app` action handlers |
| Out of scope (v1) | Full TUI snapshot tests, scripted crossterm integration, or headless terminal drivers |

**Modules and behaviors to cover:**

| Area | Examples |
|------|----------|
| Letter multiset invariant | After any sequence of typed letters and backspaces, pool ∪ answer matches the solution letters |
| Cursor fill | Valid pool letters fill cursor slot; invalid letters are no-ops |
| Backspace | Clears cursor slot (or moves left); **appends** letter to the pool |
| Duplicate letters | Multiple instances in pool; typing removes exactly one |
| Win check | Auto-check when full; case-insensitive compare; wrong full answer does not win |
| Shuffle | Output length N; same multiset as solution; not identical order when word has >1 distinct letter (use fixed RNG seed in tests) |
| Dictionary | Load/filter by length N; clue ordering; malformed entry handling |
| Clues | Clues that reveal letters update pool and answer without breaking invariants |

**Test design:**

- Prefer **table-driven** tests and small helpers (e.g. `assert_pool_answer_consistent(&state)`).
- Use **injected or seeded RNG** in `game` so shuffle tests are deterministic.
- Keep `ui` and `main` thin; untested display code should delegate to tested functions.

**Optional dev-dependencies** (only if needed): e.g. `rand = { ..., features = ["std_rng"] }` for seeded shuffles in tests.

---

## 7. Edge Cases & Rules

| Case | Behavior |
|------|----------|
| Duplicate letters in word | Multiple glyphs in pool; typing removes one instance only |
| User types letter not in pool | No-op + brief error indicator |
| Pool empty but answer incomplete | Only backspace allowed |
| All slots full, wrong word | “Not quite”; edit via backspace |
| Very long clues | Wrap; max lines with “…” if needed |
| Tiny terminal | Error if below min size; suggest resize |

---

## 8. Accessibility & UX

- Do not rely on color alone for errors (use `!` or reverse video).
- High contrast theme default; optional no-color mode.
- Screen reader: limited in TUI; provide `--plain` log mode for debugging (optional).

---

## 9. Acceptance Criteria (v1)

1. User can select word length and receive a scrambled pool of exactly N letters matching a hidden word.
2. Typing a valid pool letter removes it from the displayed pool and fills the **cursor** slot; backspace clears the cursor slot and appends the letter to the pool.
3. Pool visually reflects fewer letters after typing and more after delete.
4. No timer or time-based failure.
5. Clue sequence (definition → correct positions → first letter, plus extra definitions when `noun_senses` exist) respects per-round clue budget.
6. When all slots are full, the solution is checked automatically; correct shows win; incorrect allows correction via backspace.
7. Quit and new game work without leaving the terminal corrupted (restore canonical mode).
8. `cargo test` passes with unit tests covering the behaviors listed in §6.6.

---

## 10. Future Enhancements (post-v1)

- Validate partial words against dictionary (“is this a real word so far?”).
- Explicit Submit key in addition to auto-check when full.
- ~~Letter workbench~~ — **implemented** ([SPEC_LETTER_WORKBENCH.md](./SPEC_LETTER_WORKBENCH.md)).
- Difficulty tiers, themed packs, daily puzzle.
- Statistics (solved count, no timing).
- i18n word lists.

---

## 11. Resolved Decisions (v1)

| Decision | Choice |
|----------|--------|
| Answer interaction | **Cursor** slots with `←` / `→` (see `game.rs`) |
| Pool order on backspace | **Append** returned letter to end of pool |
| Solution check | **Auto-check** when all N slots are filled |
| Clue keys | **Tab** and **`[c]`** |
| Quit | **Esc**, **⌘Q**, **Ctrl+C** (not bare `q` on menu) |
| Language & TUI | **Rust** with **ratatui** + **crossterm** (see §6.1) |
| Testing | **Required** unit tests via `cargo test` (see §6.6) |
