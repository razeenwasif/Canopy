# Architecture

Canopy is a **local, single-user terminal LaTeX editor**: one self-contained
Rust binary that edits `.tex` files, compiles them in a sandboxed Docker
container, and previews the resulting PDF inline. There is no server, database,
or network dependency.

> **History.** Canopy began as a design for a multi-user, web-based Overleaf
> clone (React + Yjs + Node + Postgres). It was deliberately collapsed to a
> personal, offline tool. The collaboration stack (Yjs/`yrs`, WebSocket sync,
> the API server, Postgres/Redis/S3) was removed — none of it is needed for a
> single user. What survived is the part that matters on any machine: running
> untrusted LaTeX inside a locked-down container.

## Process model

A single binary with a `tokio` runtime. The main loop multiplexes two async
sources with `tokio::select!`:

```
            ┌──────────────── App event loop (main.rs → app.rs) ────────────────┐
            │                                                                    │
 keystrokes │   crossterm EventStream ──► handle_*_key ──► mutate Editor/Browser │
            │                                          └──► draw (ui::render)    │
            │                                                                    │
  compile   │   mpsc::Receiver<Result<CompileOutcome>> ──► handle_compile_result │
  results   │        ▲                                                           │
            └────────┼───────────────────────────────────────────────────────────┘
                     │ tokio::spawn
              compile::compile() ──► Docker Engine API (bollard)
```

Editing is synchronous and instant. Compilation is offloaded to a spawned task
and reports back over a channel, so a slow compile never blocks typing.

## Modules

| Module | Responsibility |
| --- | --- |
| `main.rs` | CLI parsing (clap), terminal lifecycle (`ratatui::init/restore`), wiring |
| `config.rs` | Compile settings: image, engine, timeout, memory limit |
| `app.rs` | State machine (Browser ⨉ Editor) + the `select!` event loop |
| `editor/` | The text model: ropey buffer, cursor, movement, editing, scrolling, save |
| `fs.rs` | File browser model: directory listing + navigation |
| `compile.rs` | Sandboxed Docker compilation (bollard) + Docker reachability probe |
| `pdf.rs` | PDF page rasterization for preview (pdfium-render) |
| `ui/` | Pure rendering: `browser`, `editor`, `preview`, status bar |

The `ui` layer is **pure**: it reads `App` state and paints a frame, never
mutating. All state transitions happen in `app.rs`.

## The editor model

The cursor is a single **character index** into the rope. Line and column are
derived on demand (`char_to_line` / `line_to_char`). This keeps every edit
O(log n) and avoids the classic bug of a `(line, col)` pair drifting out of sync
with the buffer. Vertical movement preserves a **sticky desired column** so
moving down through short lines and back up returns to the original column.

## Compilation security model

LaTeX is Turing-complete and `\write18`/file I/O make `pdflatex` effectively
arbitrary code execution. Canopy **never** runs it on the host. Each compile
runs in a fresh, ephemeral TeX Live container configured with:

- `NetworkMode: "none"` — no network, no data exfiltration
- a hard **memory limit** (default 512 MiB, swap disabled)
- a hard **wall-clock timeout** (default 40s; the container is killed on elapse)
- **all Linux capabilities dropped** and a **read-only root filesystem**

The project directory is bind-mounted read-write as `/work`; the engine writes
`<stem>.pdf` there, which the host reads back before the container is destroyed.
These limits are defined in `config.rs` and applied in `compile.rs`.

## PDF preview

The compiler's output PDF is rasterized one page at a time with `pdfium-render`
and displayed inline by `ratatui-image`, which negotiates the best available
terminal graphics protocol (Kitty → iTerm2 → Sixel → halfblocks fallback).

## Key dependencies

| Crate | Why |
| --- | --- |
| `ratatui` + `crossterm` | Terminal UI + input/event stream |
| `ropey` | Efficient rope data structure for the editor buffer |
| `bollard` | Async Docker Engine API client |
| `pdfium-render` + `ratatui-image` | PDF rasterization + inline terminal display |
| `tokio` | Async runtime (compilation, event multiplexing) |
| `clap` | CLI argument parsing |
| `anyhow` / `thiserror` | Error handling |
