//! Theme — the color palette and style helpers used across the UI.
//!
//! Modeled on the Onyx app's theme system (rounded theme-colored borders,
//! focused panes get a bold accent border + accent title), with a pink accent.

use ratatui::style::{Color, Modifier, Style};

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

pub struct Theme {
    // Surfaces
    pub bg: Color,
    pub bg_alt: Color,
    pub bg_sel: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_subtle: Color,
    // Accents
    pub accent: Color,
    pub accent_alt: Color,
    // Semantic (success/warning/error reserved for compile-status UI)
    #[allow(dead_code)]
    pub success: Color,
    #[allow(dead_code)]
    pub warning: Color,
    #[allow(dead_code)]
    pub error: Color,
    pub info: Color,
    // Borders
    pub border: Color,
    pub border_focus: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::pink()
    }
}

impl Theme {
    /// "Canopy Pink" — dark surfaces with a vivid pink accent.
    pub fn pink() -> Self {
        Self {
            bg: rgb(0x1b, 0x1b, 0x21),
            bg_alt: rgb(0x23, 0x22, 0x29),
            bg_sel: rgb(0x3a, 0x2a, 0x38),
            fg: rgb(0xec, 0xe6, 0xf0),
            fg_dim: rgb(0xa3, 0x9b, 0xb0),
            fg_subtle: rgb(0x6e, 0x68, 0x78),
            accent: rgb(0xff, 0x6a, 0xc1),     // pink
            accent_alt: rgb(0xc7, 0x92, 0xea), // soft purple
            success: rgb(0x7e, 0xe7, 0x87),
            warning: rgb(0xf0, 0xc6, 0x74),
            error: rgb(0xff, 0x6b, 0x6b),
            info: rgb(0x7a, 0xd9, 0xf5),
            border: rgb(0x3a, 0x33, 0x40),
            border_focus: rgb(0xff, 0x6a, 0xc1),
        }
    }

    // ─── Style helpers ────────────────────────────────────────────────────

    pub fn s_normal(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn s_dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    pub fn s_subtle(&self) -> Style {
        Style::default().fg(self.fg_subtle)
    }

    pub fn s_accent(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn s_border(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn s_border_focus(&self) -> Style {
        Style::default()
            .fg(self.border_focus)
            .add_modifier(Modifier::BOLD)
    }

    pub fn s_selection(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg_sel)
            .add_modifier(Modifier::BOLD)
    }

    pub fn s_gutter(&self) -> Style {
        Style::default().fg(self.fg_subtle)
    }

    pub fn s_gutter_active(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }
}
