# Contributing

## Prerequisites

- **Rust** (stable, 2021 edition or newer) — install via [rustup](https://rustup.rs)
- **Docker** — required to run compilation (the editor itself works without it)
- **PDFium** shared library — required for inline PDF preview

## Build, test, run

```bash
cargo build              # debug build
cargo test               # run the test suite
cargo run -- paper.tex   # run against a file
cargo clippy             # lints
cargo fmt                # format

cargo install --path .   # build release + install `canopy` to ~/.cargo/bin
```

## Code layout

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the module breakdown. The
short version:

- `app.rs` owns all state transitions.
- `ui/` is pure rendering — it must never mutate state.
- `editor/` is the text model and is unit-tested in isolation.

## Conventions

- Keep the `ui` layer free of side effects; thread state changes through `app.rs`.
- Prefer deriving line/column from the rope over storing them separately.
- New behavior in `editor/` should come with a unit test.
- Run `cargo fmt` and `cargo clippy` before opening a PR.

## Phase discipline

Work proceeds in the phases described in [docs/ROADMAP.md](docs/ROADMAP.md).
Functions slated for a later phase are marked with `TODO(phase-N)`.
