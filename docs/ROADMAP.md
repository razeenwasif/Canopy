# Roadmap

Canopy is built in phases. Each phase is reviewed before the next begins.

## Phase 1 — Scaffolding ✅

Project structure, CLI, terminal lifecycle, module seams, dependency selection.

## Phase 2 — Editor ✅

- ropey-backed text buffer
- cursor movement (arrows, Home/End, PageUp/PageDown) with a sticky desired column
- editing (insert, newline, backspace, delete-forward)
- vertical scrolling that keeps the cursor visible
- save to disk
- file browser (navigate directories, open files)
- line-number gutter + hardware cursor positioning
- unit tests for the editing logic

## Phase 3 — Sandboxed compilation ✅

- `compile.rs`: bollard container lifecycle
- bind-mount the project dir, run the TeX engine with the security constraints
  (`network:none`, memory cap + swap off, timeout-with-kill, dropped caps,
  read-only rootfs + tmpfs `/tmp`, PID cap, non-root uid:gid)
- capture the compiler log; surface status/duration/first error in the UI

## Phase 4 — Inline PDF preview ✅

- `pdf.rs`: rasterize PDF pages with `pdftoppm` (poppler)
- `ui/preview.rs`: display inline via `ratatui-image` (graphics protocol +
  half-block fallback)
- auto-refresh the preview after a successful compile
- docked in the Overleaf-style three-pane workspace (editor | preview | AI)

  _Still to do here: multi-page navigation in the preview._

## Delivered extras

- Vim-style modal editing
- Pink theme modeled on Onyx
- Fuzzy file finder (`Ctrl-F`)
- LaTeX syntax highlighting
- Local AI assistant via Ollama (`Ctrl-A`)

## Possible later work

- Surface the full compiler log in a scrollable pane; jump-to-error (SyncTeX)
- search/replace, undo/redo, Visual mode
- multi-file projects with `\input`/`\include` awareness
- let the AI assistant apply edits to the buffer directly
