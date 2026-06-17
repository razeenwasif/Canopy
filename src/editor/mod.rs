//! The text editor model: a ropey-backed buffer with a cursor, scrolling, and
//! editing operations. Rendering lives in `ui::editor`; this module is pure
//! state + logic so it can be reasoned about (and later tested) on its own.
//!
//! The cursor is stored as a single character index into the rope. Line/column
//! are derived on demand — this keeps every edit O(log n) and avoids keeping a
//! separate (line, col) in sync with the buffer.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ropey::Rope;

mod modes;
pub use modes::Mode;

pub struct Editor {
    rope: Rope,
    /// Cursor position as a character index into the rope.
    cursor: usize,
    /// Topmost visible line (vertical scroll offset).
    scroll_row: usize,
    /// "Sticky" column for vertical movement: when moving up/down across short
    /// lines, the cursor remembers the column it wants to return to.
    desired_col: usize,
    /// Backing file, if this buffer is associated with one.
    path: Option<PathBuf>,
    /// Unsaved changes since the last load/save.
    dirty: bool,
    /// Current modal-editing mode (vim-style).
    mode: Mode,
}

impl Editor {
    /// An empty scratch buffer with no backing file.
    pub fn scratch() -> Self {
        Self {
            rope: Rope::new(),
            cursor: 0,
            scroll_row: 0,
            desired_col: 0,
            path: None,
            dirty: false,
            mode: Mode::Normal,
        }
    }

    /// Load a file into a new editor buffer.
    pub fn open(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        Ok(Self {
            rope: Rope::from_str(&text),
            cursor: 0,
            scroll_row: 0,
            desired_col: 0,
            path: Some(path.to_path_buf()),
            dirty: false,
            mode: Mode::Normal,
        })
    }

    /// Write the buffer back to its file. Errors if there is no backing path.
    pub fn save(&mut self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("buffer has no file path to save to")?;
        fs::write(path, self.rope.to_string())
            .with_context(|| format!("writing {}", path.display()))?;
        self.dirty = false;
        Ok(())
    }

    // ─── Accessors used by the renderer ──────────────────────────────────

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn scroll_row(&self) -> usize {
        self.scroll_row
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Zero-based (line, column) of the cursor.
    pub fn cursor_line_col(&self) -> (usize, usize) {
        let line = self.rope.char_to_line(self.cursor);
        let col = self.cursor - self.rope.line_to_char(line);
        (line, col)
    }

    // ─── Movement ─────────────────────────────────────────────────────────

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        self.sync_desired_col();
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.rope.len_chars() {
            self.cursor += 1;
        }
        self.sync_desired_col();
    }

    pub fn move_up(&mut self) {
        let (line, _) = self.cursor_line_col();
        if line > 0 {
            self.cursor = self.char_at(line - 1, self.desired_col);
        }
    }

    pub fn move_down(&mut self) {
        let (line, _) = self.cursor_line_col();
        if line + 1 < self.rope.len_lines() {
            self.cursor = self.char_at(line + 1, self.desired_col);
        }
    }

    pub fn move_home(&mut self) {
        let (line, _) = self.cursor_line_col();
        self.cursor = self.rope.line_to_char(line);
        self.desired_col = 0;
    }

    pub fn move_end(&mut self) {
        let (line, _) = self.cursor_line_col();
        let len = self.line_len(line);
        self.cursor = self.rope.line_to_char(line) + len;
        self.desired_col = len;
    }

    pub fn page(&mut self, rows: isize) {
        let (line, _) = self.cursor_line_col();
        let last = self.rope.len_lines().saturating_sub(1);
        let target = (line as isize + rows).clamp(0, last as isize) as usize;
        self.cursor = self.char_at(target, self.desired_col);
    }

    // ─── Editing ──────────────────────────────────────────────────────────

    pub fn insert_char(&mut self, ch: char) {
        self.rope.insert_char(self.cursor, ch);
        self.cursor += 1;
        self.dirty = true;
        self.sync_desired_col();
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.rope.remove(self.cursor - 1..self.cursor);
            self.cursor -= 1;
            self.dirty = true;
            self.sync_desired_col();
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor < self.rope.len_chars() {
            self.rope.remove(self.cursor..self.cursor + 1);
            self.dirty = true;
        }
    }

    // ─── Scrolling ────────────────────────────────────────────────────────

    /// Adjust the scroll offset so the cursor stays within a viewport of the
    /// given height. Called by the renderer once the pane size is known.
    pub fn ensure_visible(&mut self, viewport_rows: usize) {
        if viewport_rows == 0 {
            return;
        }
        let (line, _) = self.cursor_line_col();
        if line < self.scroll_row {
            self.scroll_row = line;
        } else if line >= self.scroll_row + viewport_rows {
            self.scroll_row = line - viewport_rows + 1;
        }
    }

    // ─── Vim normal-mode operations ───────────────────────────────────────

    /// `i` — insert before the cursor.
    pub fn enter_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    /// `Esc` — leave insert mode, nudging the cursor back one column (vim).
    pub fn exit_insert(&mut self) {
        self.mode = Mode::Normal;
        let (_, col) = self.cursor_line_col();
        if col > 0 {
            self.cursor -= 1;
        }
        self.sync_desired_col();
    }

    /// `a` — insert after the cursor.
    pub fn enter_insert_after(&mut self) {
        let (line, _) = self.cursor_line_col();
        if self.cursor < self.rope.line_to_char(line) + self.line_len(line) {
            self.cursor += 1;
        }
        self.mode = Mode::Insert;
        self.sync_desired_col();
    }

    /// `A` — insert at end of line.
    pub fn enter_insert_eol(&mut self) {
        self.move_end();
        self.mode = Mode::Insert;
    }

    /// `I` — insert at start of line.
    pub fn enter_insert_bol(&mut self) {
        self.move_home();
        self.mode = Mode::Insert;
    }

    /// `o` — open a new line below and insert.
    pub fn open_below(&mut self) {
        self.move_end();
        self.rope.insert_char(self.cursor, '\n');
        self.cursor += 1;
        self.dirty = true;
        self.mode = Mode::Insert;
        self.sync_desired_col();
    }

    /// `O` — open a new line above and insert.
    pub fn open_above(&mut self) {
        let (line, _) = self.cursor_line_col();
        let line_start = self.rope.line_to_char(line);
        self.rope.insert_char(line_start, '\n');
        self.cursor = line_start; // sit on the new empty line
        self.dirty = true;
        self.mode = Mode::Insert;
        self.sync_desired_col();
    }

    /// `x` — delete the character under the cursor.
    pub fn delete_under(&mut self) {
        let (line, _) = self.cursor_line_col();
        // Don't delete the line's trailing newline with `x`.
        if self.cursor < self.rope.line_to_char(line) + self.line_len(line) {
            self.rope.remove(self.cursor..self.cursor + 1);
            self.dirty = true;
            self.clamp_to_line();
        }
    }

    /// `D` — delete from the cursor to the end of the line.
    pub fn delete_to_eol(&mut self) {
        let (line, _) = self.cursor_line_col();
        let eol = self.rope.line_to_char(line) + self.line_len(line);
        if self.cursor < eol {
            self.rope.remove(self.cursor..eol);
            self.dirty = true;
        }
    }

    /// `dd` — delete the current line.
    pub fn delete_line(&mut self) {
        let (line, _) = self.cursor_line_col();
        let start = self.rope.line_to_char(line);
        let end = if line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line + 1)
        } else {
            // Last line: also consume the preceding newline if present.
            self.rope.len_chars()
        };
        self.rope.remove(start..end);
        if start > 0 && line + 1 >= self.rope.len_lines() {
            // Removed the last line; drop the now-trailing newline.
            let last = self.rope.len_chars();
            if last > 0 && self.rope.char(last - 1) == '\n' {
                self.rope.remove(last - 1..last);
            }
        }
        self.dirty = true;
        self.cursor = start.min(self.rope.len_chars());
        self.clamp_to_line();
        self.sync_desired_col();
    }

    /// `w` — move to the start of the next word.
    pub fn move_word_forward(&mut self) {
        let len = self.rope.len_chars();
        let mut i = self.cursor;
        // Skip the current run of word characters, then any whitespace.
        while i < len && !self.rope.char(i).is_whitespace() {
            i += 1;
        }
        while i < len && self.rope.char(i).is_whitespace() {
            i += 1;
        }
        self.cursor = i;
        self.sync_desired_col();
    }

    /// `b` — move to the start of the previous word.
    pub fn move_word_back(&mut self) {
        let mut i = self.cursor;
        while i > 0 && self.rope.char(i - 1).is_whitespace() {
            i -= 1;
        }
        while i > 0 && !self.rope.char(i - 1).is_whitespace() {
            i -= 1;
        }
        self.cursor = i;
        self.sync_desired_col();
    }

    /// `gg` — jump to the first line.
    pub fn move_first_line(&mut self) {
        self.cursor = self.char_at(0, self.desired_col);
    }

    /// `G` — jump to the last line.
    pub fn move_last_line(&mut self) {
        let last = self.rope.len_lines().saturating_sub(1);
        self.cursor = self.char_at(last, self.desired_col);
    }

    // ─── Internals ────────────────────────────────────────────────────────

    /// Character count of a line, excluding the trailing newline.
    fn line_len(&self, line: usize) -> usize {
        let slice = self.rope.line(line);
        let mut len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' {
            len -= 1;
        }
        len
    }

    /// Char index of (line, col), clamping col to the line's length.
    fn char_at(&self, line: usize, col: usize) -> usize {
        let col = col.min(self.line_len(line));
        self.rope.line_to_char(line) + col
    }

    /// Remember the current column for subsequent vertical moves.
    fn sync_desired_col(&mut self) {
        let (_, col) = self.cursor_line_col();
        self.desired_col = col;
    }

    /// Keep the cursor within the current line's text (used after deletes in
    /// normal mode so it never lands on/after the trailing newline).
    fn clamp_to_line(&mut self) {
        let (line, col) = self.cursor_line_col();
        let max = self.line_len(line);
        if col > max {
            self.cursor = self.rope.line_to_char(line) + max;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> Editor {
        let mut e = Editor::scratch();
        for ch in s.chars() {
            if ch == '\n' {
                e.insert_newline();
            } else {
                e.insert_char(ch);
            }
        }
        e
    }

    #[test]
    fn insert_tracks_text_and_cursor() {
        let e = typed("hi\nx");
        assert_eq!(e.rope().to_string(), "hi\nx");
        assert_eq!(e.cursor_line_col(), (1, 1));
        assert!(e.is_dirty());
    }

    #[test]
    fn vertical_move_keeps_desired_column() {
        let mut e = typed("hi\nx");
        e.move_end(); // (1,1) already at end of "x"
        e.move_up(); // up to "hi"; desired col 1 → (0,1)
        assert_eq!(e.cursor_line_col(), (0, 1));
        e.move_end(); // (0,2), desired col 2
        e.move_down(); // "x" is shorter; clamp to (1,1)
        assert_eq!(e.cursor_line_col(), (1, 1));
    }

    #[test]
    fn home_and_end() {
        let mut e = typed("hello");
        e.move_home();
        assert_eq!(e.cursor_line_col(), (0, 0));
        e.move_end();
        assert_eq!(e.cursor_line_col(), (0, 5));
    }

    #[test]
    fn backspace_joins_lines() {
        let mut e = typed("hi\nx"); // cursor at (1,1)
        e.backspace(); // remove 'x'
        assert_eq!(e.rope().to_string(), "hi\n");
        assert_eq!(e.cursor_line_col(), (1, 0));
        e.backspace(); // remove newline, joining lines
        assert_eq!(e.rope().to_string(), "hi");
        assert_eq!(e.cursor_line_col(), (0, 2));
    }

    #[test]
    fn delete_forward_removes_under_cursor() {
        let mut e = typed("ab");
        e.move_home();
        e.delete_forward();
        assert_eq!(e.rope().to_string(), "b");
        assert_eq!(e.cursor_line_col(), (0, 0));
    }

    #[test]
    fn vim_insert_mode_and_x() {
        let mut e = typed("abc"); // default mode is Normal; cursor at (0,3)
        e.enter_insert();
        assert_eq!(e.mode(), Mode::Insert);
        e.exit_insert(); // back to Normal, nudging left: col 3 → 2
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.cursor_line_col(), (0, 2));
        e.move_home();
        e.delete_under(); // x on 'a'
        assert_eq!(e.rope().to_string(), "bc");
    }

    #[test]
    fn vim_dd_deletes_line() {
        let mut e = typed("one\ntwo\nthree");
        e.exit_insert();
        e.move_first_line(); // gg
        e.delete_line(); // dd
        assert_eq!(e.rope().to_string(), "two\nthree");
        assert_eq!(e.cursor_line_col(), (0, 0));
    }

    #[test]
    fn vim_open_below_inserts_line() {
        let mut e = typed("top");
        e.exit_insert();
        e.open_below(); // o
        assert_eq!(e.mode(), Mode::Insert);
        e.insert_char('x');
        assert_eq!(e.rope().to_string(), "top\nx");
        assert_eq!(e.cursor_line_col(), (1, 1));
    }

    #[test]
    fn vim_word_motion() {
        let mut e = typed("alpha beta gamma");
        e.exit_insert();
        e.move_first_line();
        e.move_home();
        e.move_word_forward(); // → start of "beta"
        assert_eq!(e.cursor_line_col(), (0, 6));
        e.move_word_back(); // → start of "alpha"
        assert_eq!(e.cursor_line_col(), (0, 0));
    }

    #[test]
    fn open_edit_save_roundtrips() {
        let dir = std::env::temp_dir().join(format!("canopy-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.tex");
        std::fs::write(&path, "alpha\nbeta").unwrap();

        let mut e = Editor::open(&path).unwrap();
        assert_eq!(e.rope().to_string(), "alpha\nbeta");
        assert!(!e.is_dirty());

        e.insert_char('Z'); // at (0,0)
        assert!(e.is_dirty());
        e.save().unwrap();
        assert!(!e.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Zalpha\nbeta");

        std::fs::remove_dir_all(&dir).ok();
    }
}
