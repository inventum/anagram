# anagram

An anagram puzzle game. You get a scrambled word and move its letters from the pool into the answer slots.
Two front ends share one Rust core: a terminal TUI and a macOS desktop app built with Tauri.

## Quick start: terminal

```sh
make test                      # run the test suite
cargo run --                   # start the TUI (menu first)
cargo run -- play              # start the TUI, same as above
cargo run -- solve eilnst      # list anagrams of the letters (word<TAB>definition)
```

The TUI needs an interactive terminal. Run it in a terminal emulator, not through a pipe.

## Quick start: desktop (macOS)

Install the Tauri CLI once: `cargo install tauri-cli --version "^2" --locked`.

```sh
npm --prefix ui-web install            # frontend dependencies
cargo tauri dev                        # development run
cargo tauri build --bundles app        # output: target/release/bundle/macos/Anagram Desktop.app
```

See [INSTALL.md](INSTALL.md) for prerequisites, configuration, and troubleshooting.

## Documentation

- [docs/SPEC.md](docs/SPEC.md): terminal game specification
- [docs/SPEC-ui.md](docs/SPEC-ui.md): desktop app specification
- [docs/architecture.md](docs/architecture.md): module map and how the two front ends share the core
- [INSTALL.md](INSTALL.md): build, run, configuration, and troubleshooting
- [AGENTS.md](AGENTS.md): guidance for AI coding agents working in this repo
- [docs/RUNBOOK_UI.md](docs/RUNBOOK_UI.md): desktop work packages and decisions
