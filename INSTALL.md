# Install and Build

Anagram ships a terminal app (`anagram`) and a macOS desktop app (`Anagram Desktop`).
Both use the same word data and config file. Architecture: [docs/architecture.md](docs/architecture.md).

## 1. Prerequisites

| Tool | Required version | Observed on main | Used for |
|------|------------------|-------------------------|----------|
| Rust / Cargo | root crate `rust-version = "1.75"`; desktop `rust-version = "1.90"` | cargo 1.95.0 | All Rust builds and tests |
| Node.js | `docs/SPEC-ui.md` §9 says "LTS"; TODO: pin an exact line | v26.4.0 | Desktop frontend (`ui-web/`) |
| npm | ships with Node | 11.17.0 | `ui-web/` install and scripts |
| Tauri CLI | `^2` (`cargo install tauri-cli --version "^2" --locked`) | tauri-cli 2.12.6 | `cargo tauri dev` / `build` |
| Xcode Command Line Tools | any recent version | `/Library/Developer/CommandLineTools` | macOS linker and SDK |
| GNU make | any | — | `Makefile` targets |
| Python 3 | standard library only | — | `make data` only (optional) |

Install Rust with `rustup`. Desktop builds need 1.90 or newer. The root crate declares 1.75, but a 1.75
toolchain build has not been tested. TODO: verify.

Install the Tauri CLI once. This takes a long time to compile:

```sh
cargo install tauri-cli --version "^2" --locked
```

## 2. Clone

```sh
git clone git@github.com:inventum/anagram.git
cd anagram
```

This needs SSH access to `github.com:inventum/anagram`. The desktop app is on `main`.

## 3. Terminal app

```sh
make test                      # cargo test (root crate; 125 tests observed)
cargo build --release --locked
./target/release/anagram       # release binary (about 39 MB observed)
cargo run --                   # debug build, same app
```

The TUI needs an interactive terminal. Run it in a terminal emulator, not through a pipe.

Commands and flags (from `src/cli.rs`):

| Command | Behavior |
|---------|----------|
| `anagram` or `anagram play` | Interactive puzzle, menu first |
| `anagram play --word WORD` | Starts a puzzle for `WORD`, skipping the menu |
| `anagram solve SCRAMBLE` | Prints `word<TAB>definition` for each anagram of `SCRAMBLE`, sorted |
| `--config PATH` (global) | Config file path (default `~/.config/anagram/config.toml`) |
| `--plain` (global) | Loads config, then exits without starting the TUI. Reserved; no other behavior |

`solve` exit codes (`src/solve.rs`): `0` matches found, `1` no match, `2` invalid input.

```sh
cargo run -- solve eilnst
echo $?
```

## 4. Desktop app

Run from the repository root. Install frontend dependencies (`RUNBOOK_UI.md` uses `install`; `npm ci` in `ui-web/` is
preferred for an exact lock-file install):

```sh
npm --prefix ui-web install
```

Development run. Tauri runs `npm run dev` in `ui-web/` (`beforeDevCommand`), serves port 1420, then opens the window:

```sh
cargo tauri dev
```

Browser-only frontend (no Tauri): `npm --prefix ui-web run dev`, then open `http://localhost:1420/?fixture=playing`.

Production build. Tauri runs `npm run build` in `ui-web/` (`beforeBuildCommand`):

```sh
cargo tauri build --bundles app
```

Output: `target/release/bundle/macos/Anagram Desktop.app`.

Frontend checks:

```sh
npm --prefix ui-web run typecheck
npm --prefix ui-web run test
npm --prefix ui-web run build
```

Desktop Rust tests:

```sh
cargo test -p anagram-desktop
```

`cargo test` at the root covers only the terminal crate (`default-members = ["."]`). Use `cargo test --workspace` for both.
`RUNBOOK_UI.md` runs the same commands with `--locked`.

## 5. Configuration

Both front ends read `~/.config/anagram/config.toml`. The TUI accepts `--config PATH`; the desktop app always uses
the default path. A missing file means defaults.

| Key | Type | Default | Notes |
|-----|------|---------|-------|
| `min_size` | integer | `4` | Desktop clamps to 4–10 and requires `min_size` ≤ `max_size`. TUI uses the value as given |
| `max_size` | integer | `10` | As above. TUI with `min_size > max_size` exits with code 101 |
| `locale` | string | `"en-US"` | Parsed; no code reads it yet |
| `theme` | string | `"default"` | Parsed; no code reads it yet |
| `empty_slot_color` | color name | `light_magenta` | Empty answer slot |
| `filled_slot_color` | color name | `light_yellow` | Filled answer slot |
| `pool_color` | color name | `light_cyan` | Letter tiles |
| `correct_position_color` | color name | `light_green` | Correct-position highlight |
| `workbench` | bool | `true` | Shows the letter workbench and Patterns panel |
| `workbench_rows` | integer | `5` | Only square 4×4 or 5×5 are honored; anything else becomes 5×5 |
| `workbench_cols` | integer | `5` | As above |

Color names (case-insensitive): `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `gray`/`grey`,
`darkgray`/`dark_gray`/`darkgrey`, `lightred`/`light_red`, `lightgreen`/`light_green`, `lightyellow`/`light_yellow`,
`lightblue`/`light_blue`, `lightmagenta`/`light_magenta`, `lightcyan`/`light_cyan`. Unknown names fall back to magenta.

Example:

```toml
min_size = 5
max_size = 8
workbench = true
workbench_rows = 4
workbench_cols = 4
pool_color = "light_cyan"
```

Malformed TOML: the TUI exits with status 1 and prints the parse error. The desktop app shows a native dialog titled
"Anagram cannot start" and exits with status 1.

## 6. Regenerate word data (optional)

The committed files in `data/` are what the builds use. Regenerate only when you intend to change the word list.
`make data` overwrites `data/words.json`, `data/signature_index.json`, `data/affix_patterns.json`, and `data/words.manifest.json`.

Requirements:

- Python 3 (standard library only).
- WordNet 3.x `dict/` directory. Default `/tmp/wn/dict`; override with `WN_DIR`. Download per the script docstring:

```sh
curl -LO https://wordnetcode.princeton.edu/wn3.1.dict.tar.gz
mkdir -p /tmp/wn && tar -xzf wn3.1.dict.tar.gz -C /tmp/wn
```

```sh
make data                      # full pipeline; fails if entries < MIN_ENTRIES (default 35000)
make data-index                # rebuilds only signature_index.json and affix_patterns.json
make test
```

The committed `words.manifest.json` records 86,587 entries. TODO: `make data` was not run during this guide, so the
output of a fresh regeneration is unverified.

## 7. Troubleshooting

| Symptom | Cause and fix |
|---------|---------------|
| `Anagram cannot start` dialog | Malformed config, `min_size > max_size`, or no dictionary word for a length in range. Fix `config.toml` |
| TUI exits with code 101 | `min_size > max_size` in config (observed under a pseudo-terminal). Fix both keys |
| `cargo test` shows only the terminal crate | Expected: `default-members` is the root crate. Use `cargo test -p anagram-desktop` or `cargo test --workspace` |
| `cargo tauri` not found | Run `cargo install tauri-cli --version "^2" --locked` |
| `Operation not permitted` during a build | Write permission problem on `target/` or `ui-web/node_modules`. Fix ownership, or run outside a sandbox |
| Vite fails on port 1420 | Port in use. Vite uses `strictPort`; stop the other process |
| `src-tauri/gen/` appears in `git status` after a build | Generated by Tauri; referenced by `capabilities`. Do not commit it. No `.gitignore` covers it yet |
| `make data` fails with "entry count below MIN_ENTRIES" | WordNet source incomplete. Check the contents of `WN_DIR` |
| Desktop GUI behavior (`cargo tauri dev`, generic dock icon) | TODO: GUI runs were not executed in the verification pass. Verified: frontend build, Rust tests, release CLI build, `.app` bundle exists |
