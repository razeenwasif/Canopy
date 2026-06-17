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
