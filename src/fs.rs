//! File browser model: list a directory, navigate in/out, and pick a file to
//! open. Kept separate from rendering (`ui::browser`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct Browser {
    cwd: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
}

/// Outcome of activating (Enter on) the selected entry.
pub enum Activate {
    /// The user descended into / went up to a directory; UI should redraw.
    Navigated,
    /// The user chose a file to open in the editor.
    OpenFile(PathBuf),
}

impl Browser {
    pub fn new(start: &Path) -> Result<Self> {
        // If started on a file, browse its parent directory.
        let cwd = if start.is_dir() {
            start.to_path_buf()
        } else {
            start.parent().unwrap_or(Path::new(".")).to_path_buf()
        };
        let mut browser = Self {
            cwd,
            entries: Vec::new(),
            selected: 0,
        };
        browser.refresh()?;
        Ok(browser)
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1).min(self.entries.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Activate the current selection: descend into a directory or open a file.
    pub fn activate(&mut self) -> Result<Activate> {
        let Some(entry) = self.entries.get(self.selected) else {
            return Ok(Activate::Navigated);
        };
        if entry.is_dir {
            self.cwd = entry.path.clone();
            self.selected = 0;
            self.refresh()?;
            Ok(Activate::Navigated)
        } else {
            Ok(Activate::OpenFile(entry.path.clone()))
        }
    }

    /// Go up to the parent directory.
    pub fn go_up(&mut self) -> Result<()> {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.selected = 0;
            self.refresh()?;
        }
        Ok(())
    }

    /// Re-read the current directory. Directories sort first, then files, both
    /// alphabetically. Hidden entries (dotfiles) are skipped.
    fn refresh(&mut self) -> Result<()> {
        let mut entries = Vec::new();
        let read = std::fs::read_dir(&self.cwd)
            .with_context(|| format!("reading directory {}", self.cwd.display()))?;
        for item in read {
            let item = item?;
            let name = item.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
            entries.push(Entry {
                name,
                path: item.path(),
                is_dir,
            });
        }
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        self.entries = entries;
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        Ok(())
    }
}
