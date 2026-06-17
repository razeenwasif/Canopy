# 🌳 Canopy

A local, single-user **terminal LaTeX editor**. Edit `.tex` files in your
terminal, compile them inside an ephemeral, network-isolated Docker container,
and preview the resulting PDF inline. One self-contained Rust binary — no
server, no database, no network.

## Why Docker for compilation?

LaTeX is Turing-complete; `pdflatex` can read/write files and (mis)behave. Canopy
never runs it on the host. Each compile spins up a fresh TeX Live container with:

- `network: none` — no network access
- a hard memory limit (default 512 MiB, swap disabled)
- a hard timeout (default 40s)
- all Linux capabilities dropped, read-only root filesystem

The project directory is bind-mounted as the working dir; the PDF is read back
out and the container is destroyed.

## Install

```bash
cargo install --path .      # builds release, installs `canopy` to ~/.cargo/bin
```

## Usage

```bash
canopy                 # browse the current directory
canopy paper.tex       # open a file directly
canopy ./thesis/       # browse a directory
```

Options (also via env): `--texlive-image`, `--engine`, `--timeout-secs`,
`--memory-bytes`.

### Keys

The editor is **modal** (vim-style), with a pink-accented theme modeled on Onyx.

**Browser:** `j/k` move · `gg`/`G` top/bottom · `Enter`/`l` open · `h`/`Backspace` up · `q` quit

**Editor — Normal:** `h j k l` move · `w`/`b` word · `0`/`$` line ends · `gg`/`G` file ends ·
`i a I A` insert · `o`/`O` open line · `x` delete char · `dd` delete line · `D` delete to EOL ·
`Ctrl-D`/`Ctrl-U` half-page · `Ctrl-O` file browser · `:` command line

**Editor — Insert:** type to edit · `Esc` to Normal · arrows/Home/End/PgUp/PgDn move

**Command line:** `:w` write · `:q` quit · `:wq`/`:x` write & quit · `:q!` discard · `:e` browser · `:make` compile

**Any mode:** `Ctrl-S` save · `Ctrl-B` compile · `Ctrl-P` toggle preview · `Ctrl-C` quit

## Runtime requirements

- **Docker** — for compilation (the editor works without it; compile is disabled).
- **PDFium** shared library — for inline PDF preview.

## Project structure

```
src/
├── main.rs        # CLI + terminal lifecycle
├── config.rs      # compile settings (image, engine, timeout, memory)
├── app.rs         # state machine + event loop (input ⨉ compile results)
├── editor/mod.rs  # ropey-backed buffer: cursor, editing, scrolling, save
├── fs.rs          # file browser model
├── compile.rs     # sandboxed Docker compilation (bollard)
├── pdf.rs         # PDF rasterization for preview (pdfium-render)
└── ui/            # ratatui rendering: browser, editor, preview
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — process model, modules, security model
- [Roadmap](docs/ROADMAP.md) — phases and status
- [Contributing](CONTRIBUTING.md) — build, test, conventions
- [Changelog](CHANGELOG.md)

## Status

- **Phase 1** — scaffold ✅
- **Phase 2** — editor: buffer, cursor, editing, scrolling, save, file browser ✅
- **Phase 3** — sandboxed Docker compilation (`compile.rs` body) ⏳
- **Phase 4** — inline PDF preview (`pdf.rs` + `ui/preview.rs`)
