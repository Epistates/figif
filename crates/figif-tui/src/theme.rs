//! Color theme for the TUI.

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

/// Application color theme using Tailwind-inspired colors.
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub highlight: Color,
    pub highlight_bg: Color,
    pub static_segment: Color,
    pub motion_segment: Color,
    pub selected: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub border: Color,
    pub border_focused: Color,
    pub header_bg: Color,
    pub footer_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::Rgb(226, 232, 240),            // slate-200
            fg_dim: Color::Rgb(148, 163, 184),        // slate-400
            highlight: Color::Rgb(34, 211, 238),      // cyan-400
            highlight_bg: Color::Rgb(8, 51, 68),      // cyan-950
            static_segment: Color::Rgb(251, 191, 36), // amber-400
            motion_segment: Color::Rgb(96, 165, 250), // blue-400
            selected: Color::Rgb(52, 211, 153),       // emerald-400
            error: Color::Rgb(248, 113, 113),         // red-400
            success: Color::Rgb(74, 222, 128),        // green-400
            warning: Color::Rgb(251, 191, 36),        // amber-400
            border: Color::Rgb(71, 85, 105),          // slate-600
            border_focused: Color::Rgb(34, 211, 238), // cyan-400
            header_bg: Color::Rgb(30, 41, 59),        // slate-800
            footer_bg: Color::Rgb(30, 41, 59),        // slate-800
        }
    }
}

impl Theme {
    /// Style for normal text.
    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg)
    }

    /// Style for dimmed text.
    pub fn dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    /// Style for highlighted text.
    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for selected item.
    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .bg(self.highlight_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for static segment label.
    pub fn static_label(&self) -> Style {
        Style::default().fg(self.static_segment)
    }

    /// Style for motion segment label.
    pub fn motion_label(&self) -> Style {
        Style::default().fg(self.motion_segment)
    }

    /// Style for border (unfocused).
    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Style for border (focused).
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    /// Style for header.
    pub fn header(&self) -> Style {
        Style::default().bg(self.header_bg).fg(self.fg)
    }

    /// Style for footer.
    pub fn footer(&self) -> Style {
        Style::default().bg(self.footer_bg).fg(self.fg_dim)
    }

    /// Style for error text.
    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Style for success text.
    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Style for keybinding hints.
    pub fn keybind(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for keybinding description.
    pub fn keybind_desc(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    /// Style for warning text.
    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }
}
