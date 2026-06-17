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

## Phase 3 — Sandboxed compilation ⏳ (next)

- `compile.rs`: bollard container lifecycle
- bind-mount the project dir, run the TeX engine with the security constraints
  (`network:none`, memory cap, timeout, dropped caps, read-only rootfs)
- stream/capture the compiler log
- surface status + errors in the UI

## Phase 4 — Inline PDF preview

- `pdf.rs`: rasterize PDF pages with pdfium-render
- `ui/preview.rs`: display inline via ratatui-image, with page navigation
- auto-refresh the preview after a successful compile

## Possible later work

- LaTeX syntax highlighting in the editor
- jump-to-error from the compiler log (SyncTeX)
- search/replace, undo/redo
- multi-file projects with `\input`/`\include` awareness
- configurable keybindings
