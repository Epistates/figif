//! Help overlay widget.

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Render the help overlay.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Help - Keybindings ")
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_items: &[(&str, &str, bool)] = &[
        // Timeline Navigation
        ("", "-- Timeline Navigation --", true),
        ("← / h", "Previous item", false),
        ("→ / l", "Next item", false),
        ("Home / g", "First item", false),
        ("End / G", "Last item", false),
        ("↑ / k / Enter", "Zoom in (view frames)", false),
        ("↓ / j", "Zoom out (view segments)", false),
        ("", "", false),
        // Selection
        ("", "-- Selection --", true),
        ("Space", "Toggle selection", false),
        ("a", "Select all", false),
        ("A", "Deselect all", false),
        ("s", "Select all static segments", false),
        ("m", "Select all motion segments", false),
        ("d", "Select duplicate frames (in frame view)", false),
        ("", "", false),
        // Operations
        ("", "-- Operations --", true),
        ("r", "Remove (toggle)", false),
        ("x", "Clear operation", false),
        ("c", "Cap duration (prompts for max ms)", false),
        ("C", "Collapse to single frame (prompts for ms)", false),
        ("o", "Open optimize menu", false),
        ("u", "Undo last change", false),
        ("U", "Reset ALL operations", false),
        ("/ (frames)", "Split after current frame", false),
        ("", "", false),
        // Playback
        ("", "-- Preview --", true),
        ("p", "Play/pause preview", false),
        ("+/=", "Zoom preview in", false),
        ("-", "Zoom preview out", false),
        ("0", "Reset preview zoom", false),
        ("", "", false),
        // Settings
        ("", "-- Settings --", true),
        (
            "H",
            "Cycle hash algorithm (dHash → pHash → blockHash)",
            false,
        ),
        ("", "", false),
        // File & Other
        ("", "-- File & System --", true),
        ("e", "Export dialog (when operations pending)", false),
        ("Esc", "Close dialog/modal", false),
        ("?", "Toggle this help", false),
        ("q", "Quit application", false),
    ];

    let lines: Vec<Line> = help_items
        .iter()
        .map(|item| {
            if item.2 {
                // Section header
                Line::from(vec![Span::styled(
                    format!("  {}", item.1),
                    theme.highlight(),
                )])
            } else if item.0.is_empty() && item.1.is_empty() {
                // Spacer
                Line::from("")
            } else {
                // Normal entry
                Line::from(vec![
                    Span::styled(format!("{:>12}", item.0), theme.keybind()),
                    Span::styled("  ", theme.dim()),
                    Span::styled(item.1, theme.keybind_desc()),
                ])
            }
        })
        .collect();

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, inner);
}
