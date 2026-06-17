//! Lightweight LaTeX syntax highlighting.
//!
//! A fast, per-line tokenizer (no `syntect`/tree-sitter dependency) that colors
//! the constructs that matter for readability:
//!   * `\commands` and control symbols — accent
//!   * `% comments` — subtle/dim
//!   * `$…$` inline math (and its contents) — accent_alt
//!   * `{ } [ ]` grouping — dim
//!
//! It is intentionally stateless per line. Inline `$…$` math is tracked within a
//! line; multi-line math environments (`\[ … \]`, `equation`) are not state-
//! tracked across lines — a deliberate trade for speed and simplicity.

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::theme::Theme;

/// Does this file extension get LaTeX highlighting?
pub fn is_tex(ext: Option<&str>) -> bool {
    matches!(
        ext.map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("tex" | "latex" | "sty" | "cls" | "bib" | "dtx")
    )
}

/// Tokenize one line into styled spans. Total character count is preserved, so
/// callers can still map columns to screen cells 1:1.
pub fn highlight_line(line: &str, theme: &Theme) -> Vec<Span<'static>> {
    let normal = theme.s_normal();
    let cmd_style = Style::default().fg(theme.accent);
    let comment_style = Style::default()
        .fg(theme.fg_subtle)
        .add_modifier(Modifier::ITALIC);
    let math_style = Style::default().fg(theme.accent_alt);
    let brace_style = Style::default().fg(theme.fg_dim);

    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut in_math = false;
    let mut i = 0;

    // Flush the plain-text buffer with the appropriate style.
    macro_rules! flush {
        () => {
            if !buf.is_empty() {
                let style = if in_math { math_style } else { normal };
                spans.push(Span::styled(std::mem::take(&mut buf), style));
            }
        };
    }

    while i < n {
        let c = chars[i];
        match c {
            '%' => {
                flush!();
                let rest: String = chars[i..].iter().collect();
                spans.push(Span::styled(rest, comment_style));
                return spans;
            }
            '\\' => {
                flush!();
                // Control sequence: `\` + letters, or `\` + a single symbol.
                let mut j = i + 1;
                if j < n && chars[j].is_ascii_alphabetic() {
                    while j < n && chars[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                } else if j < n {
                    j += 1; // escaped symbol like \%, \{, \\
                }
                let cs: String = chars[i..j].iter().collect();
                spans.push(Span::styled(cs, cmd_style));
                i = j;
            }
            '$' => {
                flush!();
                spans.push(Span::styled("$".to_string(), math_style));
                in_math = !in_math;
                i += 1;
            }
            '{' | '}' | '[' | ']' => {
                flush!();
                spans.push(Span::styled(c.to_string(), brace_style));
                i += 1;
            }
            _ => {
                buf.push(c);
                i += 1;
            }
        }
    }
    flush!();
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tex_extensions() {
        assert!(is_tex(Some("tex")));
        assert!(is_tex(Some("TEX")));
        assert!(is_tex(Some("bib")));
        assert!(!is_tex(Some("rs")));
        assert!(!is_tex(None));
    }

    #[test]
    fn preserves_character_count() {
        let theme = Theme::pink();
        let line = "\\section{Intro} % a $x$ note";
        let total: usize = highlight_line(line, &theme)
            .iter()
            .map(|s| s.content.chars().count())
            .sum();
        assert_eq!(total, line.chars().count());
    }

    #[test]
    fn comment_runs_to_end_of_line() {
        let theme = Theme::pink();
        let spans = highlight_line("text % comment $here$", &theme);
        // Last span is the whole comment tail.
        let last = spans.last().unwrap();
        assert_eq!(last.content.as_ref(), "% comment $here$");
    }
}
