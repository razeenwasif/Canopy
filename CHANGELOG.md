# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Terminal text editor backed by a ropey buffer: cursor movement (arrows,
  Home/End, PageUp/PageDown) with a sticky desired column, insert/newline/
  backspace/delete editing, vertical scrolling, and save to disk.
- Vim-style modal editing (Normal / Insert / Command) with motions (`h j k l`,
  `w`, `b`, `0`, `$`, `gg`, `G`), edits (`i a I A`, `o`, `O`, `x`, `dd`, `D`),
  and a `:` command line (`:w`, `:q`, `:wq`, `:q!`, `:e`, `:make`).
- Pink-accented theme modeled on the Onyx app: rounded theme-colored pane
  borders (bold accent when focused), a top title bar, and a bottom mode line
  with a colored mode block.
- File browser: navigate directories and open files (vim keys).
- Built-in fzf-style fuzzy file finder (`Ctrl-F`): a gitignore-aware walk (the
  `ignore` crate) scoped to the enclosing git repo, SkimMatcherV2 ranking in a
  centered popup, with `Ctrl-P`/`Ctrl-N` navigation. Skips hidden files, VCS
  dirs, virtualenvs, and build output so it stays focused on project source.
- LaTeX syntax highlighting: a lightweight per-line tokenizer coloring commands,
  comments, inline math, and grouping with the theme palette.
- Sandboxed Docker compilation (Phase 3): runs the TeX engine in an ephemeral
  container with `network:none`, a memory cap (swap off), a hard timeout enforced
  by killing the container, all capabilities dropped, a read-only root filesystem
  with a tmpfs `/tmp`, a PID cap, and execution as the host uid:gid. Captures the
  log and reports status/duration/first-error.
- Local AI assistant (`Ctrl-A`) backed by Ollama (default `gemma4:12b-it-qat`):
  a streaming chat overlay that includes the current document as context.
- Line-number gutter (active line in accent) and hardware cursor positioning.
- Sandboxed-compilation scaffolding (`compile.rs`) with a Docker reachability
  probe and the security constraints expressed in config.
- Inline PDF preview scaffolding (`pdf.rs`, `ui/preview.rs`).
- Project documentation (README, architecture, roadmap, contributing).
- Unit tests for the editor model.

### Notes
- This is a pre-1.0 scaffold. Sandboxed Docker compilation (Phase 3) and inline
  PDF preview (Phase 4) are not yet functional.

[Unreleased]: https://github.com/razeenwasif/Canopy
