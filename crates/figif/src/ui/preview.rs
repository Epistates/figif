//! Preview and stats panel widget with optional image rendering.

use crate::app::App;
use crate::ui::format_duration;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui_image::{FilterType, Resize, StatefulImage};

/// Render the preview/stats panel with optional image display.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect, _focused: bool) {
    // If we have image support and analysis, show centered image preview
    if app.picker.is_some() && app.analysis.is_some() {
        render_centered_image(app, frame, area);
    } else {
        render_stats_only(app, frame, area);
    }
}

/// Render centered image preview that fills the available space.
fn render_centered_image(app: &mut App, frame: &mut Frame, area: Rect) {
    // Update image state if needed (handles magnification via cropping)
    app.update_preview_image();

    let theme = &app.theme;

    // Render image if we have state
    if let Some(ref mut image_state) = app.image_state {
        // Determine render area based on zoom level
        let render_area = if app.preview_scale < 1.0 {
            // Zoom out: shrink the render area proportionally, then center
            let scaled = Rect::new(
                area.x,
                area.y,
                (area.width as f32 * app.preview_scale).max(1.0) as u16,
                (area.height as f32 * app.preview_scale).max(1.0) as u16,
            );
            center_rect(scaled, area)
        } else {
            area
        };

        // Render image fitted to the render area (aspect ratio preserved, upscales to fill)
        let image_widget = StatefulImage::new().resize(Resize::Scale(Some(FilterType::Nearest)));
        frame.render_stateful_widget(image_widget, render_area, image_state);
    } else {
        // Show loading message centered
        let text = Paragraph::new("Loading preview...")
            .style(theme.dim())
            .alignment(Alignment::Center);
        frame.render_widget(text, area);
    }
}

/// Calculate a centered rect within a container.
fn center_rect(inner: Rect, outer: Rect) -> Rect {
    // Use the inner's dimensions, but center it within the outer
    let x = outer.x + (outer.width.saturating_sub(inner.width)) / 2;
    let y = outer.y + (outer.height.saturating_sub(inner.height)) / 2;

    Rect::new(
        x,
        y,
        inner.width.min(outer.width),
        inner.height.min(outer.height),
    )
}

/// Render stats-only view (fallback when no image support).
fn render_stats_only(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let mut lines = Vec::new();

    // Show why image preview isn't available
    if app.picker.is_none() {
        lines.push(Line::from(vec![Span::styled(
            "Image preview unavailable",
            theme.warning(),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "(Terminal doesn't support graphics)",
            theme.dim(),
        )]));
        lines.push(Line::from(""));
    }

    if let Some(stats) = app.get_preview_stats() {
        // Original stats
        lines.push(Line::from(vec![Span::styled(
            "--- Original ---",
            theme.dim(),
        )]));
        lines.push(Line::from(vec![
            Span::styled("  Frames:   ", theme.dim()),
            Span::styled(format!("{}", stats.original_frames), theme.normal()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Duration: ", theme.dim()),
            Span::styled(format_duration(stats.original_duration), theme.normal()),
        ]));

        lines.push(Line::from(""));

        // Modified stats
        lines.push(Line::from(vec![Span::styled(
            "--- After Changes ---",
            theme.highlight(),
        )]));
        lines.push(Line::from(vec![
            Span::styled("  Frames:   ", theme.dim()),
            Span::styled(format!("{}", stats.result_frames), theme.highlight()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Duration: ", theme.dim()),
            Span::styled(format_duration(stats.result_duration), theme.highlight()),
        ]));

        lines.push(Line::from(""));

        // Savings
        let saved_duration = stats.saved_duration();
        let saved_percent = stats.saved_percent();

        let (saved_style, saved_sign) = if saved_duration > 0 {
            (theme.success(), "-")
        } else if saved_duration < 0 {
            (theme.error(), "+")
        } else {
            (theme.dim(), "")
        };

        lines.push(Line::from(vec![Span::styled(
            "--- Savings ---",
            theme.dim(),
        )]));
        lines.push(Line::from(vec![
            Span::styled("  Time:     ", theme.dim()),
            Span::styled(
                format!(
                    "{}{}",
                    saved_sign,
                    format_duration(saved_duration.unsigned_abs())
                ),
                saved_style,
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Percent:  ", theme.dim()),
            Span::styled(format!("{:.1}%", saved_percent.abs() * 100.0), saved_style),
        ]));

        lines.push(Line::from(""));

        // Current segment details
        if let Some(analysis) = &app.analysis
            && app.selected_segment < analysis.segments.len()
        {
            let seg = &analysis.segments[app.selected_segment];

            lines.push(Line::from(vec![Span::styled(
                "--- Current Segment ---",
                theme.dim(),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  ID:       ", theme.dim()),
                Span::styled(format!("#{}", seg.id), theme.highlight()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Type:     ", theme.dim()),
                if seg.is_static {
                    Span::styled("STATIC", theme.static_label())
                } else {
                    Span::styled("MOTION", theme.motion_label())
                },
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Frames:   ", theme.dim()),
                Span::styled(
                    format!("{}-{}", seg.frame_range.start, seg.frame_range.end - 1),
                    theme.normal(),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Duration: ", theme.dim()),
                Span::styled(format_duration(seg.duration_ms() as u64), theme.normal()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Count:    ", theme.dim()),
                Span::styled(format!("{} frames", seg.frame_count()), theme.normal()),
            ]));

            // Show operation if applied
            if let Some(op) = app.operations.get(&seg.id) {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  Operation: ", theme.dim()),
                    Span::styled(
                        format!("{:?}", op),
                        theme.warning().add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "No file loaded",
            theme.dim(),
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Usage: figif <file.gif>",
            theme.highlight(),
        )]));
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}
