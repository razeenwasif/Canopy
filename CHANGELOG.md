# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Terminal text editor backed by a ropey buffer: cursor movement (arrows,
  Home/End, PageUp/PageDown) with a sticky desired column, insert/newline/
  backspace/delete editing, vertical scrolling, and save to disk.
- File browser: navigate directories and open files.
- Line-number gutter and hardware cursor positioning.
- Sandboxed-compilation scaffolding (`compile.rs`) with a Docker reachability
  probe and the security constraints expressed in config.
- Inline PDF preview scaffolding (`pdf.rs`, `ui/preview.rs`).
- Project documentation (README, architecture, roadmap, contributing).
- Unit tests for the editor model.

### Notes
- This is a pre-1.0 scaffold. Sandboxed Docker compilation (Phase 3) and inline
  PDF preview (Phase 4) are not yet functional.

[Unreleased]: https://github.com/razeenwasif/Canopy
