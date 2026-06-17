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

The workspace is **Overleaf-style**: the editor on the left, the **PDF preview**
in the center-right, and the **AI assistant** docked on the right. The editor is
**modal** (vim-style) with a pink-accented theme modeled on Onyx, **LaTeX syntax
highlighting**, and a built-in **fuzzy file finder** (`Ctrl-F`).

`Ctrl-W` cycles focus between the panes; toggle them with `:pdf` and `:ai` (or
`Ctrl-P` for the preview). The highlights are below — see
[docs/KEYBINDINGS.md](docs/KEYBINDINGS.md) for the complete reference.

**Browser:** `j/k` move · `gg`/`G` top/bottom · `Enter`/`l` open · `h`/`Backspace` up · `/` or `Ctrl-F` fuzzy find · `q` quit

**Fuzzy finder (`Ctrl-F`):** type to filter · `↑/↓` or `Ctrl-P`/`Ctrl-N` move · `Enter` open · `Esc` close

**Editor — Normal:** `h j k l` move · `w`/`b` word · `0`/`$` line ends · `gg`/`G` file ends ·
`i a I A` insert · `o`/`O` open line · `x` delete char · `dd` delete line · `D` delete to EOL ·
`Ctrl-F` fuzzy find · `Ctrl-D`/`Ctrl-U` half-page · `Ctrl-O` file browser · `:` command line

**Editor — Insert:** type to edit · `Esc` to Normal · arrows/Home/End/PgUp/PgDn move

**Command line:** `:w` write · `:q` quit · `:wq`/`:x` write & quit · `:q!` discard · `:e` browser · `:make` compile

**PDF preview (`Ctrl-W` to focus):** `j/k/h/l` or arrows scroll · `+`/`-` zoom · `n`/`p` next/prev page · `g`/`G` first/last · `0` reset · `Esc` back to editor

**AI assistant (`Ctrl-A` to focus):** type a question · `Enter` send · `PgUp/PgDn` scroll · `Esc` (or `Ctrl-A`) back to editor · `Esc` while streaming stops it

**Any mode:** `Ctrl-S` save · `Ctrl-B` compile · `Ctrl-P` toggle preview · `Ctrl-F` find · `Ctrl-A` AI · `Ctrl-C` quit

## Runtime requirements

- **Docker** — for compilation (the editor works without it; compile is disabled).
- **Ollama** — for the AI assistant. Runs on `http://localhost:11434` with the
  model `gemma4:12b-it-qat` by default (override with `--ai-model` /
  `--ollama-host` or `CANOPY_AI_MODEL` / `CANOPY_OLLAMA_HOST`).
- **poppler-utils** (`pdftoppm`) — to rasterize the PDF for the inline preview.
  Best visual results in a terminal with a graphics protocol (Kitty/iTerm2/Sixel);
  otherwise it falls back to a half-block rendering that works anywhere.

## Project structure

```
src/
├── main.rs        # CLI + terminal lifecycle
├── config.rs      # compile + AI settings
├── app.rs         # state machine + modal dispatch + event loop
├── editor/        # ropey-backed buffer + vim modes
├── finder.rs      # fuzzy file finder (SkimMatcherV2)
├── syntax.rs      # LaTeX syntax highlighting
├── compile.rs     # sandboxed Docker compilation (bollard)
├── ai.rs          # local Ollama assistant (streaming)
├── pdf.rs         # PDF rasterization for preview (pdfium-render)
└── ui/            # ratatui rendering: title_bar, browser, editor, finder, ai, preview, status
```

## Documentation

- [Keybindings](docs/KEYBINDINGS.md) — full key reference for every context
- [Architecture](docs/ARCHITECTURE.md) — process model, modules, security model
- [Roadmap](docs/ROADMAP.md) — phases and status
- [Contributing](CONTRIBUTING.md) — build, test, conventions
- [Changelog](CHANGELOG.md)

## Status

- **Phase 1** — scaffold ✅
- **Phase 2** — editor: buffer, cursor, editing, scrolling, save, file browser ✅
- **Phase 3** — sandboxed Docker compilation (`compile.rs`) ✅
- **Phase 4** — inline PDF preview (`pdf.rs` + `ui/preview.rs`) ✅
- **Extras** — vim keybindings, pink theme, fuzzy finder, LaTeX highlighting, Ollama AI assistant, Overleaf-style 3-pane layout ✅
