//! fzf-style fuzzy file finder.
//!
//! Walks the project root once on open, then fuzzy-filters the file list as the
//! user types (using the same SkimMatcherV2 the Onyx switcher uses). Matches are
//! ranked by score; the display path is relative to the root.

use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ignore::WalkBuilder;

/// Heavy directories pruned regardless of `.gitignore` (build output, deps,
/// virtualenvs) so the finder stays focused on source files. `.git` and hidden
/// entries are already skipped by the `ignore` walker.
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "venv",
    ".venv",
    "envs",
    "__pycache__",
    "dist",
    "build",
    ".next",
];

/// Cap the candidate list so very large trees stay responsive.
const MAX_CANDIDATES: usize = 50_000;
/// Cap the number of results rendered.
const MAX_MATCHES: usize = 200;

pub struct Finder {
    pub query: String,
    root: PathBuf,
    /// All candidate files (absolute paths).
    candidates: Vec<PathBuf>,
    /// Current filtered+ranked matches (absolute paths).
    matches: Vec<PathBuf>,
    pub selected: usize,
}

impl Finder {
    /// Build a finder rooted at `root`, walking it for files. Respects
    /// `.gitignore`, skips hidden files and VCS dirs (via the `ignore` crate),
    /// and additionally prunes heavy build/dep/venv directories.
    pub fn new(root: &Path) -> Self {
        let mut candidates = Vec::new();
        let walker = WalkBuilder::new(root)
            .hidden(true) // skip dotfiles/dirs
            .git_ignore(true) // respect .gitignore
            .git_global(true)
            .git_exclude(true)
            .parents(true) // honor .gitignore in parent dirs
            .filter_entry(|e| !is_skipped(e.file_name().to_str()))
            .build();

        for entry in walker.flatten() {
            if entry.file_type().is_some_and(|t| t.is_file()) {
                candidates.push(entry.into_path());
                if candidates.len() >= MAX_CANDIDATES {
                    break;
                }
            }
        }
        candidates.sort();

        let mut finder = Self {
            query: String::new(),
            root: root.to_path_buf(),
            candidates,
            matches: Vec::new(),
            selected: 0,
        };
        finder.refilter();
        finder
    }

    pub fn push(&mut self, ch: char) {
        self.query.push(ch);
        self.refilter();
    }

    pub fn pop(&mut self) {
        self.query.pop();
        self.refilter();
    }

    pub fn move_down(&mut self) {
        if !self.matches.is_empty() {
            self.selected = (self.selected + 1).min(self.matches.len() - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.matches.get(self.selected).map(|p| p.as_path())
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Display paths (relative to root) for the current matches.
    pub fn display_matches(&self) -> Vec<String> {
        self.matches.iter().map(|p| self.rel(p)).collect()
    }

    fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root)
            .unwrap_or(p)
            .to_string_lossy()
            .to_string()
    }

    fn refilter(&mut self) {
        let q = self.query.trim();
        if q.is_empty() {
            self.matches = self.candidates.iter().take(MAX_MATCHES).cloned().collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(i64, &PathBuf)> = self
                .candidates
                .iter()
                .filter_map(|p| {
                    let rel = self.rel(p);
                    matcher.fuzzy_match(&rel, q).map(|score| (score, p))
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.matches = scored
                .into_iter()
                .take(MAX_MATCHES)
                .map(|(_, p)| p.clone())
                .collect();
        }
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }
}

fn is_skipped(name: Option<&str>) -> bool {
    name.is_some_and(|n| SKIP_DIRS.contains(&n))
}
