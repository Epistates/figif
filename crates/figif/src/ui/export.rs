//! Export dialog widget.

use crate::actions::ExportState;
use crate::app::App;
use crate::ui::format_duration;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Render the export dialog.
pub fn render(app: &App, frame: &mut Frame, state: &ExportState, area: Rect) {
    let theme = &app.theme;

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Export ")
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    // Path input with cursor
    let (before_cursor, after_cursor) = state.path.split_at(state.cursor.min(state.path.len()));
    lines.push(Line::from(vec![
        Span::styled("Output: ", theme.dim()),
        Span::styled(before_cursor, theme.normal()),
        Span::styled("_", theme.highlight().add_modifier(Modifier::SLOW_BLINK)),
        Span::styled(after_cursor, theme.normal()),
    ]));

    lines.push(Line::from(""));

    // File exists warning
    if state.file_exists {
        if state.confirmed_overwrite {
            lines.push(Line::from(vec![Span::styled(
                "  Press Enter again to overwrite",
                theme.warning(),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "  ⚠ File exists - will overwrite",
                theme.warning(),
            )]));
        }
        lines.push(Line::from(""));
    }

    // Operation summary
    let summary = &state.operation_summary;
    lines.push(Line::from(vec![Span::styled(
        "Changes Summary:",
        theme.highlight(),
    )]));

    if summary.segments_removed > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  • {} segment(s) removed", summary.segments_removed),
            theme.error(),
        )]));
    }

    if summary.segments_collapsed > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  • {} segment(s) collapsed", summary.segments_collapsed),
            theme.warning(),
        )]));
    }

    if summary.segments_scaled > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  • {} segment(s) scaled", summary.segments_scaled),
            theme.motion_label(),
        )]));
    }

    lines.push(Line::from(""));

    // Duration comparison
    let saved_ms = summary.original_duration_ms as i64 - summary.result_duration_ms as i64;
    let saved_percent = if summary.original_duration_ms > 0 {
        (saved_ms as f64 / summary.original_duration_ms as f64) * 100.0
    } else {
        0.0
    };

    lines.push(Line::from(vec![
        Span::styled("  Duration: ", theme.dim()),
        Span::styled(
            format_duration(summary.original_duration_ms),
            theme.normal(),
        ),
        Span::styled(" → ", theme.dim()),
        Span::styled(
            format_duration(summary.result_duration_ms),
            theme.highlight(),
        ),
        Span::styled(
            format!(" ({:+.1}%)", -saved_percent),
            if saved_ms > 0 {
                theme.success()
            } else {
                theme.error()
            },
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("  Frames:   ", theme.dim()),
        Span::styled(format!("{}", summary.original_frames), theme.normal()),
        Span::styled(" → ", theme.dim()),
        Span::styled(format!("{}", summary.result_frames), theme.highlight()),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Instructions
    lines.push(Line::from(vec![
        Span::styled("  Enter", theme.keybind()),
        Span::styled(": Save    ", theme.keybind_desc()),
        Span::styled("Esc", theme.keybind()),
        Span::styled(": Cancel    ", theme.keybind_desc()),
        Span::styled("Ctrl+D", theme.keybind()),
        Span::styled(": Discard", theme.keybind_desc()),
    ]));

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, inner);
}
