# chair_game: notes for coding assistants

A first-person game in Rust on Bevy (edition 2024). The crates change often: people try a
controller, physics or input crate and swap it for another. Read `Cargo.toml` for what is in use
today rather than assuming.

## Bevy and crate versions

Bevy's API changes every release, and so do the crates built on it. What you remember is probably
from older versions. Before using a type, system parameter or plugin you are not sure of, read the
version in `Cargo.lock`: its source and examples, not your memory.

## Checks

- `cargo test --features betty` builds the game with the test harness and runs every test. Add
  headless tests for gameplay you write: an `App` with `MinimalPlugins`, the systems under test,
  `TimeUpdateStrategy::ManualDuration` for time, and assertions on components. No window or GPU.
- `cargo clippy --all-targets` and `cargo fmt --check`, where those components are installed.

## Seeing the game

`cargo run --features betty -- betty/walk.txt` opens the game for a few seconds, plays the script
(keys held, mouse moves, screenshots), prints `betty:` lines and exits by itself. Screenshots land in
`betty/shots/<name>.png` (git-ignored). The script format is at the top of `src/betty.rs`. Time
advances 1/60 s per frame and the real keyboard and mouse are ignored, so the same script gives the
same pictures. `dump` prints every camera's position, yaw and pitch (and any entity with a `Name`),
which shows whether movement worked without needing a picture.

The harness knows nothing about the game: it sends the same raw input messages a real window does,
so it keeps working when the controller or input crate changes. Its one hook is in `main()`:

```rust
#[cfg(feature = "betty")]
mod betty;
// in main(), after DefaultPlugins:
#[cfg(feature = "betty")]
app.add_plugins(betty::BettyPlugin);
```

If `main()` is rewritten, keep those lines. A scripted run that prints no `betty:` line and never
exits means the hook is missing. Scripts may need updating when controls change (a new key, a
click to capture the mouse): write new ones in `betty/` for what you need to see, and keep
`walk.txt` working as a smoke test.

The `betty` feature is off in normal builds: nothing in `src/betty.rs` is compiled into the game
the others play.

## Style

Follow the existing code: short `//` comments saying why, one system per job, constants at the top.
