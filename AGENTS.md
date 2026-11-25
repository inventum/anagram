# AGENTS.md

Guidance for AI coding agents working in this repository. Humans: see [INSTALL.md](INSTALL.md) and [docs/architecture.md](docs/architecture.md).

## 1. Purpose

Anagram is a word puzzle. The player gets a scrambled word and types the answer.
The repo has:

- A terminal app (`src/`, crate `anagram`): ratatui + crossterm.
- A macOS desktop app (`src-tauri/` crate `anagram-desktop` and `ui-web/`): Tauri 2 + TypeScript.
- Shared core logic in `src/`. Both front ends depend on it.
- Pre-generated word data in `data/` (embedded at compile time).

## 2. Specs and runbooks

Read the relevant document before changing behavior. Code is the source of truth when docs disagree.

| Document | Scope |
|----------|-------|
| `docs/SPEC.md` | Terminal game: rules, §4.5 global keys, §5.2 shuffle identity rule |
| `docs/SPEC-ui.md` | Desktop app: §3.6 keyboard, §4.3 backend, §5.5 visibility by phase, §9 build commands, §11 open questions |
| `docs/SPEC_SIGNATURE_INDEX.md` | Signature index format and data pipeline |
| `docs/SPEC_LETTER_WORKBENCH.md` | Letter workbench behavior |
| `docs/RUNBOOK.md`, `RUNBOOK_LETTER_WORKBENCH.md`, `RUNBOOK_SIGNATURE_INDEX.md` | Terminal app build and workflow steps |
| `docs/RUNBOOK_UI.md` | Desktop work packages and decisions (D-1 to D-18) |

Known drift (code wins): `RUNBOOK_UI.md` names the branch `ui-design-docs` and a baseline of 108 tests; the current
baseline is in §4. `RUNBOOK_LETTER_WORKBENCH.md` says Ctrl+G resets the grid; in code Ctrl+G toggles focus and Ctrl+Shift+G resets.

## 3. Commands

All commands are copied from `Makefile`, `ui-web/package.json`, `Cargo.toml`, or `docs/RUNBOOK_UI.md`. Wrap long-running commands in `timeout`.

| Purpose | Command |
|---------|---------|
| Terminal tests (root crate) | `make test` (`cargo test`) |
| Root crate tests, locked | `cargo test --locked` |
| All workspace tests | `cargo test --workspace --locked` |
| Desktop Rust tests | `cargo test -p anagram-desktop --locked` |
| Terminal release build | `cargo build --release --locked` |
| Solve CLI | `cargo run -- solve eilnst` |
| Frontend install | `npm --prefix ui-web install` (`npm ci` in `ui-web/` for lock-exact installs) |
| Frontend typecheck | `npm --prefix ui-web run typecheck` (`tsc --noEmit`) |
| Frontend tests | `npm --prefix ui-web run test` (`vitest run`) |
| Frontend build | `npm --prefix ui-web run build` (`tsc --noEmit && vite build`) |
| Frontend dev server | `npm --prefix ui-web run dev` (port 1420) |
| Desktop dev run | `cargo tauri dev` |
| Desktop bundle | `cargo tauri build --bundles app` |
| Regenerate data | `make data` (needs WordNet; overwrites committed `data/`; do not run casually) |
| Regenerate indexes only | `make data-index` |

Install the Tauri CLI with `cargo install tauri-cli --version "^2" --locked` (`RUNBOOK_UI.md` P-1).

## 4. Hard invariants

Do not break these without an explicit user decision:

1. **Test baseline (observed 2026-11-25):** root 125 passing; desktop 43 (37 lib + 4 `tests/config.rs` + 2 `tests/ipc_leak.rs`); frontend 80 in 3 files. A change that lowers a count must say why.
2. **No `Serialize` on core types.** Core types in `src/` must not derive `Serialize`. The desktop DTOs in `src-tauri/src/dto.rs` do the serialization.
3. **The DTO hides the answer.** `round.solution` is sent only in phase `won`. Pool and answer slots are sent; the solution string is not. `ipc_leak.rs` checks this with random action sequences.
4. **The DTO hides the definition until won.** `round.definition` is sent only in `won`.
5. **Custom phases hide round data.** In `customWordInput` and `customWordPick`, `round`, `workbench`, and clue data are absent.
6. **The frontend has no game logic.** `ui-web/` renders `UiState` and sends `ActionDto`. No word lists, answer checks, or shuffles in TypeScript.
7. **Core reducer is the only state mutator.** Desktop changes state through `Session::dispatch` → `app::reduce`, never by editing `GameState` directly from the DTO layer.
8. **Lock files.** Use `--locked` with cargo commands. Review `Cargo.lock` and `ui-web/package-lock.json` diffs; a Tauri dependency change should only add what the task needs.
9. **Root `rust-version = "1.75"` stays.** The desktop crate requires 1.90. Do not raise the root value to satisfy a desktop dependency without asking.
10. **Embedded data stays valid.** `signature_index.json` must keep `format_version` 1. `words.json` must load through `Dictionary::from_embedded`.

## 5. Conventions

### 5.1 Adding a core action (terminal and desktop)

1. Add a variant to `Action` in `src/app.rs`.
2. Add a `reduce_*` handler and a match arm in `reduce`. Guard by phase the same way neighboring handlers do.
3. If the terminal has a key for it, add the mapping in `src/input.rs` (`map_playing`, `map_navigation`, and so on) and a test in that file.
4. Add reducer tests in `src/app.rs` (`#[cfg(test)]` module).
5. If the desktop can send it, continue with §5.2.

### 5.2 Adding a desktop action (`ActionDto`)

1. Add a variant to `ActionDto` in `src-tauri/src/dto.rs`. The serde tag is `type`, names are camelCase.
2. Map it in `ActionDto::into_actions`. Validate letters with the existing `letter()` helper; reject non-letters with `DtoError::InvalidLetter`.
3. Add the same shape to `ui-web/src/types.ts` (`ActionDto`).
4. Bind it in `ui-web/src/input/keymap.ts` or in the component that sends it. Keep `keyToAction` pure.
5. Add or extend a test: `actions_map_one_to_one` or `wire_format_deserializes` in `dto.rs`, and a keymap test in `ui-web/src/input/keymap.test.ts`.
6. If the action replaces an unfinished puzzle, add it to `GATED_ACTIONS` in `ui-web/src/overlays/index.ts`.

### 5.3 Adding a UI component (frontend)

1. Put the component under the matching folder (`render/`, `overlays/`, `workbench/`) and export a `render*` function that takes `(root, ctx: RenderCtx)`.
2. Read data only from `ctx.state` (`UiState`). Send changes with `ctx.dispatch` or `ctx.requestReplace` (for actions that need confirmation).
3. Add ARIA attributes in the component, or extend `applyA11y` in `a11y.ts`. Keep focus handling inside the existing dialog trap.
4. Add a fixture entry in `ui-web/src/fixtures.ts` if the component needs a new visual state (dev only, via `?fixture=`).
5. Add a unit test next to the component (`*.test.ts`). Run `npm --prefix ui-web run test` and `typecheck`.

### 5.4 Changing the DTO shape

1. Update `dto.rs`, `ui-web/src/types.ts`, and the `ipc_leak.rs` or `dto.rs` tests in the same change.
2. Check `docs/SPEC-ui.md` §5.5 (visibility by phase) before adding a field that could expose answer data.

## 6. Gotchas

| Area | Fact | Source |
|------|------|--------|
| Esc (terminal) | Quits in play, navigation, and custom pick. Cancels in custom input. | `src/input.rs` |
| Esc (desktop) | Never quits. Closes help, or cancels a custom phase. | `ui-web/src/input/keymap.ts` `escapeAction` |
| Ctrl+G (terminal) | Toggles focus between answer and grid. Does not reset. | `src/input.rs` `workbench_ctrl_g_toggles` |
| Ctrl+Shift+G (terminal) | Resets the workbench grid. | `src/input.rs` `grid_map` |
| Ctrl+R | Resets the round in play phases. Desktop: `isCtrlR` (no Shift, no Meta). | `keymap.ts`, `input.rs` |
| Workbench (desktop) | Pointer-only. No keyboard bindings. | `ui-web/src/workbench/index.ts` |
| Answer slots | `li.slot` elements, not focusable. | `ui-web/src/render/index.ts` |
| Affix hints | Core `affix_hints` can include a full-length match of a short answer. Desktop drops exact full-length matches in `affix_dto`. Terminal does not filter. | `src/affix_hints.rs`, `src-tauri/src/dto.rs` |
| Custom-word cancel (desktop) | Sends `cancelCustomWord` then `newPuzzle`. The current round is replaced. | `dto.rs` `into_actions` (D-9) |
| WordNet glosses | A definition can contain the answer word. `ipc_leak.rs` reports this as a finding, not a failure. | `src-tauri/tests/ipc_leak.rs` |
| Terminal min > max | `min_size > max_size` panics in `AppConfig::clamp_size` (exit 101 observed). Desktop returns a startup error. | `src/config.rs` |
| Unused config keys | `locale` and `theme` are parsed but not read. | `src/config.rs` |
| Duplicated data | `words.json` is embedded twice (`dictionary.rs`, `affix_hints.rs`). | `src/` |
| Generated files | `src-tauri/gen/` appears after Tauri builds. It is not ignored. | `git status` |
| Tauri CLI edits | `cargo tauri build` may rewrite features in `src-tauri/Cargo.toml` (commit `83fc4da` is an earlier CLI change). TODO: reproduce; review the diff after any `cargo tauri` run. | git history |
| Frontend fixtures | `?fixture=` works only in dev outside Tauri. The production bundle has no fixture data. | `ui-web/src/ipc.ts` |
| Build sandbox | Build scripts and Vite can fail with "Operation not permitted" under a sandbox. Rerun with escalated permission when the task allows it. | observed |

## 7. Git workflow

- Work on a feature branch. The current task branch is `tauri-ui`. Never commit to or push `main`.
- Stage paths explicitly: `git add <paths>`. Do not use `git add -A` or `git add .`.
- Never commit: `target/`, `ui-web/node_modules/`, `ui-web/dist/`, `src-tauri/gen/`.
- Commit messages: short imperative summary, matching recent history (for example "Click answer slot to move cursor; click pool tile to place letter").
- Push with `git push origin <branch>`. Verify with `git ls-remote origin <branch>`.
- Worktrees are optional. `RUNBOOK_UI.md` uses them for parallel work; run `npm ci` inside each new worktree.

## 8. Pre-finish checklist

Run these before reporting a task done. Use `timeout` on each.

1. `make test` and `cargo test --workspace --locked` (root 125, desktop 43 observed).
2. `npm --prefix ui-web run typecheck`, `npm --prefix ui-web run test` (80 observed), `npm --prefix ui-web run build`.
3. If a DTO, action, or keymap changed: confirm `ipc_leak.rs` and the keymap tests pass.
4. If a shared component changed: check all consumers (`reduce` callers, `ui_state` callers, `render*` callers).
5. `git status --short` shows only intended paths. No `src-tauri/gen/`, `target/`, or `ui-web/dist/` entries.
6. Reread every changed file once after editing.
7. Report what you could not run (for example, GUI runs of `cargo tauri dev`). Mark it as TODO; do not claim it passed.
