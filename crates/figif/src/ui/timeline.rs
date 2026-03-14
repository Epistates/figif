//! Horizontal timeline widget for segment and frame visualization.

use crate::app::App;
use crate::theme::Theme;
use crate::ui::format_duration;
use figif_core::prelude::{FrameOp, SegmentOp};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Zoom level for timeline view.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ZoomLevel {
    /// View all segments (default)
    #[default]
    Segments,
    /// View frames within a specific segment
    Frames { segment_id: usize },
}

/// Render the timeline widget.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;

    // Determine zoom level from app view mode
    let zoom = match app.view_mode {
        crate::app::ViewMode::Segments => ZoomLevel::Segments,
        crate::app::ViewMode::Frames { segment_id } => ZoomLevel::Frames { segment_id },
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 10 {
        return; // Not enough space
    }

    let Some(analysis) = &app.analysis else {
        let empty = Paragraph::new("No GIF loaded").style(theme.dim());
        frame.render_widget(empty, inner);
        return;
    };

    match zoom {
        ZoomLevel::Segments => {
            render_segments_timeline(app, frame, inner, analysis);
        }
        ZoomLevel::Frames { segment_id } => {
            render_frames_timeline(app, frame, inner, analysis, segment_id);
        }
    }
}

/// Render segment-level timeline.
fn render_segments_timeline(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    analysis: &figif_core::Analysis<img_hash::ImageHash>,
) {
    let theme = &app.theme;
    let segments = &analysis.segments;

    if segments.is_empty() {
        return;
    }

    // Calculate total duration for proportional widths
    let total_duration: u64 = segments.iter().map(|s| s.duration_ms() as u64).sum();
    if total_duration == 0 {
        return;
    }

    // Layout: title line, timeline bar, cursor line, info line
    let title_area = Rect { height: 1, ..area };
    let bar_area = Rect {
        y: area.y + 1,
        height: 1,
        ..area
    };
    let cursor_area = Rect {
        y: area.y + 2,
        height: 1,
        ..area
    };
    let info_area = Rect {
        y: area.y + 3,
        height: area.height.saturating_sub(3),
        ..area
    };

    // Title line with zoom indicator
    let static_count = segments.iter().filter(|s| s.is_static).count();
    let motion_count = segments.len() - static_count;
    let title = Line::from(vec![
        Span::styled(
            format!(
                " Timeline ({} segments: {} static, {} motion) ",
                segments.len(),
                static_count,
                motion_count
            ),
            theme.normal(),
        ),
        Span::raw(" "),
        Span::styled("[Segments ", theme.dim()),
        Span::styled("▼", theme.highlight()),
        Span::styled("]", theme.dim()),
    ]);
    frame.render_widget(Paragraph::new(title), title_area);

    // Build timeline bar
    let available_width = area.width as usize;
    let mut bar_spans: Vec<Span> = Vec::new();
    let mut cursor_spans: Vec<Span> = Vec::new();

    // Calculate widths for each segment
    let mut segment_positions: Vec<(usize, usize)> = Vec::new(); // (start_col, width)
    let mut current_col = 0;

    for (idx, segment) in segments.iter().enumerate() {
        // Calculate proportional width (minimum 1 char)
        let proportion = segment.duration_ms() as f64 / total_duration as f64;
        let width = ((proportion * available_width as f64).round() as usize).max(1);
        let width = width.min(available_width.saturating_sub(current_col));

        segment_positions.push((current_col, width));

        // Determine segment style
        let is_cursor = idx == app.selected_segment;
        let is_selected = app.selected_segments.contains(&segment.id);
        let op = app.operations.get(&segment.id);

        let (char, style) = get_segment_style(segment, is_cursor, is_selected, op, theme);

        bar_spans.push(Span::styled(char.to_string().repeat(width), style));
        current_col += width;
    }

    // Fill remaining space if any
    if current_col < available_width {
        bar_spans.push(Span::styled(
            " ".repeat(available_width - current_col),
            theme.dim(),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(bar_spans)), bar_area);

    // Cursor indicator line
    let cursor_col = if app.selected_segment < segment_positions.len() {
        let (start, width) = segment_positions[app.selected_segment];
        start + width / 2
    } else {
        0
    };

    // Build cursor line with arrow pointing to current segment
    let mut cursor_line = " ".repeat(available_width);
    if cursor_col < available_width {
        cursor_line.replace_range(cursor_col..cursor_col + 1, "▲");
    }
    cursor_spans.push(Span::styled(cursor_line, theme.highlight()));
    frame.render_widget(Paragraph::new(Line::from(cursor_spans)), cursor_area);

    // Info line
    if app.selected_segment < segments.len() {
        let segment = &segments[app.selected_segment];
        let type_str = if segment.is_static {
            "STATIC"
        } else {
            "MOTION"
        };
        let type_style = if segment.is_static {
            theme.static_label()
        } else {
            theme.motion_label()
        };

        let op = app.operations.get(&segment.id);
        let (op_text, op_style) = get_operation_text(op, theme);

        // Selection summary
        let selection_summary = if !app.selected_segments.is_empty() {
            let selected_duration: u64 = segments
                .iter()
                .filter(|s| app.selected_segments.contains(&s.id))
                .map(|s| s.duration_ms() as u64)
                .sum();
            format!(
                " | Selected: {} ({}) ",
                app.selected_segments.len(),
                format_duration(selected_duration)
            )
        } else {
            String::new()
        };

        // Pending ops summary
        let ops_summary = if !app.operations.is_empty() {
            let remove_count = app
                .operations
                .values()
                .filter(|o| matches!(o, SegmentOp::Remove))
                .count();
            if remove_count > 0 {
                format!(" | Removing: {} segments", remove_count)
            } else {
                format!(" | {} ops pending", app.operations.len())
            }
        } else {
            String::new()
        };

        let info = Line::from(vec![
            Span::styled(format!(" Seg #{} ", segment.id), theme.dim()),
            Span::styled(type_str, type_style),
            Span::styled(
                format!(
                    " {} ({}fr) ",
                    format_duration(segment.duration_ms() as u64),
                    segment.frame_count()
                ),
                theme.normal(),
            ),
            Span::styled(op_text, op_style),
            Span::styled(selection_summary, theme.dim()),
            Span::styled(ops_summary, theme.warning()),
        ]);

        frame.render_widget(Paragraph::new(info), info_area);
    }
}

/// Render frame-level timeline (zoomed into a segment).
fn render_frames_timeline(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    analysis: &figif_core::Analysis<img_hash::ImageHash>,
    segment_id: usize,
) {
    let theme = &app.theme;

    let Some(segment) = analysis.segments.iter().find(|s| s.id == segment_id) else {
        return;
    };

    let frame_count = segment.frame_count();
    if frame_count == 0 {
        return;
    }

    // Layout: title line, timeline bar, cursor line, info line
    let title_area = Rect { height: 1, ..area };
    let bar_area = Rect {
        y: area.y + 1,
        height: 1,
        ..area
    };
    let cursor_area = Rect {
        y: area.y + 2,
        height: 1,
        ..area
    };
    let info_area = Rect {
        y: area.y + 3,
        height: area.height.saturating_sub(3),
        ..area
    };

    // Title line with zoom indicator
    let type_str = if segment.is_static {
        "STATIC"
    } else {
        "MOTION"
    };
    let title = Line::from(vec![
        Span::styled(
            format!(
                " Segment #{} ({} frames, {}) ",
                segment_id, frame_count, type_str
            ),
            theme.normal(),
        ),
        Span::raw(" "),
        Span::styled("[Frames ", theme.dim()),
        Span::styled("▲", theme.highlight()),
        Span::styled("]", theme.dim()),
    ]);
    frame.render_widget(Paragraph::new(title), title_area);

    // Build frame timeline bar
    let available_width = area.width as usize;
    let mut bar_spans: Vec<Span> = Vec::new();

    // Calculate total duration for proportional widths
    let total_duration: u64 = (0..frame_count)
        .filter_map(|i| {
            analysis
                .frames
                .get(segment.frame_range.start + i)
                .map(|f| f.frame.delay_centiseconds as u64 * 10)
        })
        .sum();

    let total_duration = total_duration.max(1);

    // Track frame positions for cursor
    let mut frame_positions: Vec<(usize, usize)> = Vec::new();
    let mut current_col = 0;

    for frame_idx in 0..frame_count {
        let abs_frame_idx = segment.frame_range.start + frame_idx;
        let frame_data = analysis.frames.get(abs_frame_idx);

        // Calculate proportional width
        let delay_ms = frame_data
            .map(|f| f.frame.delay_centiseconds as u64 * 10)
            .unwrap_or(100);
        let proportion = delay_ms as f64 / total_duration as f64;
        let width = ((proportion * available_width as f64).round() as usize).max(1);
        let width = width.min(available_width.saturating_sub(current_col));

        frame_positions.push((current_col, width));

        // Determine frame style
        let is_cursor = frame_idx == app.selected_frame;
        let is_selected = app.selected_frames.contains(&frame_idx);
        let distance = frame_data.and_then(|f| f.distance_to_prev);
        let op = app.frame_operations.get(&(segment_id, frame_idx));

        let (char, style) = get_frame_style(is_cursor, is_selected, distance, op, theme);

        bar_spans.push(Span::styled(char.to_string().repeat(width), style));
        current_col += width;
    }

    // Fill remaining space
    if current_col < available_width {
        bar_spans.push(Span::styled(
            " ".repeat(available_width - current_col),
            theme.dim(),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(bar_spans)), bar_area);

    // Cursor indicator line
    let cursor_col = if app.selected_frame < frame_positions.len() {
        let (start, width) = frame_positions[app.selected_frame];
        start + width / 2
    } else {
        0
    };

    let mut cursor_line = " ".repeat(available_width);
    if cursor_col < available_width {
        cursor_line.replace_range(cursor_col..cursor_col + 1, "▲");
    }
    let cursor_spans = vec![Span::styled(cursor_line, theme.highlight())];
    frame.render_widget(Paragraph::new(Line::from(cursor_spans)), cursor_area);

    // Info line
    let abs_frame_idx = segment.frame_range.start + app.selected_frame;
    let frame_data = analysis.frames.get(abs_frame_idx);
    let delay_ms = frame_data
        .map(|f| f.frame.delay_centiseconds as u64 * 10)
        .unwrap_or(0);
    let distance = frame_data.and_then(|f| f.distance_to_prev);

    let (sim_text, sim_style) = get_similarity_text(distance, theme);

    let op = app.frame_operations.get(&(segment_id, app.selected_frame));
    let (op_text, op_style) = get_frame_operation_text(op, theme);

    // Selection summary
    let selection_summary = if !app.selected_frames.is_empty() {
        format!(" | Selected: {} frames", app.selected_frames.len())
    } else {
        String::new()
    };

    // Frame ops summary
    let frame_ops_count = app
        .frame_operations
        .iter()
        .filter(|((sid, _), _)| *sid == segment_id)
        .count();
    let ops_summary = if frame_ops_count > 0 {
        format!(" | {} frame ops", frame_ops_count)
    } else {
        String::new()
    };

    let info = Line::from(vec![
        Span::styled(format!(" Frame #{} ", app.selected_frame), theme.dim()),
        Span::styled(format_duration(delay_ms), theme.normal()),
        Span::styled(" ", theme.dim()),
        Span::styled(sim_text, sim_style),
        Span::styled(op_text, op_style),
        Span::styled(selection_summary, theme.dim()),
        Span::styled(ops_summary, theme.warning()),
    ]);

    frame.render_widget(Paragraph::new(info), info_area);
}

/// Get segment display character and style.
fn get_segment_style(
    segment: &figif_core::Segment,
    is_cursor: bool,
    is_selected: bool,
    op: Option<&SegmentOp>,
    theme: &Theme,
) -> (char, Style) {
    // Base character: filled block for segments
    let char = if matches!(op, Some(SegmentOp::Remove)) {
        '░' // Strikethrough effect for removed
    } else {
        '█'
    };

    // Determine color based on segment type and state
    let base_style = if segment.is_static {
        theme.static_label()
    } else {
        theme.motion_label()
    };

    let style = if matches!(op, Some(SegmentOp::Remove)) {
        theme.error().add_modifier(Modifier::DIM)
    } else if is_cursor && is_selected {
        theme
            .selected()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else if is_cursor {
        base_style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else if is_selected {
        theme.selected()
    } else {
        base_style
    };

    (char, style)
}

/// Get frame display character and style.
fn get_frame_style(
    is_cursor: bool,
    is_selected: bool,
    distance: Option<u32>,
    op: Option<&FrameOp>,
    theme: &Theme,
) -> (char, Style) {
    // Use vertical bar for frames
    let char = if matches!(op, Some(FrameOp::Remove)) {
        '░'
    } else if matches!(op, Some(FrameOp::SplitAfter)) {
        '┃'
    } else {
        '│'
    };

    // Color based on similarity
    let base_style = match distance {
        None | Some(0) => theme.static_label(), // Identical/first
        Some(1..=2) => theme.dim(),             // Nearly identical
        Some(3..=5) => theme.normal(),          // Similar
        Some(_) => theme.motion_label(),        // Different
    };

    let style = if matches!(op, Some(FrameOp::Remove)) {
        theme.error().add_modifier(Modifier::DIM)
    } else if is_cursor && is_selected {
        theme
            .selected()
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else if is_cursor {
        base_style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else if is_selected {
        theme.selected()
    } else {
        base_style
    };

    (char, style)
}

/// Get operation text and style for segment.
fn get_operation_text(op: Option<&SegmentOp>, theme: &Theme) -> (String, Style) {
    match op {
        Some(SegmentOp::Remove) => (
            " REMOVE ".to_string(),
            theme.error().add_modifier(Modifier::BOLD),
        ),
        Some(SegmentOp::Collapse { delay_cs }) => (
            format!(" COLLAPSE {}ms ", delay_cs * 10),
            theme.warning().add_modifier(Modifier::BOLD),
        ),
        Some(SegmentOp::Scale { factor }) if *factor < 1.0 => (
            format!(" FASTER {:.1}x ", 1.0 / factor),
            theme.motion_label().add_modifier(Modifier::BOLD),
        ),
        Some(SegmentOp::Scale { factor }) => (
            format!(" SLOWER {:.1}x ", factor),
            theme.motion_label().add_modifier(Modifier::BOLD),
        ),
        Some(SegmentOp::SetDuration { total_cs }) => (
            format!(" DURATION {}ms ", total_cs * 10),
            theme.highlight().add_modifier(Modifier::BOLD),
        ),
        Some(SegmentOp::SetFrameDelay { delay_cs }) => (
            format!(" DELAY {}ms ", delay_cs * 10),
            theme.highlight().add_modifier(Modifier::BOLD),
        ),
        Some(SegmentOp::Keep) | None => (String::new(), theme.dim()),
    }
}

/// Get operation text and style for frame.
fn get_frame_operation_text(op: Option<&FrameOp>, theme: &Theme) -> (String, Style) {
    match op {
        Some(FrameOp::Remove) => (
            " REMOVE ".to_string(),
            theme.error().add_modifier(Modifier::BOLD),
        ),
        Some(FrameOp::SplitAfter) => (
            " SPLIT ".to_string(),
            theme.warning().add_modifier(Modifier::BOLD),
        ),
        Some(FrameOp::Keep) | None => (String::new(), theme.dim()),
    }
}

/// Get similarity indicator text.
fn get_similarity_text(distance: Option<u32>, theme: &Theme) -> (String, Style) {
    match distance {
        None => ("(first)".to_string(), theme.dim()),
        Some(0) => ("(identical)".to_string(), theme.static_label()),
        Some(1..=2) => ("(~identical)".to_string(), theme.dim()),
        Some(3..=5) => ("(similar)".to_string(), theme.normal()),
        Some(d) => (format!("(diff:{})", d), theme.motion_label()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_level_default() {
        assert_eq!(ZoomLevel::default(), ZoomLevel::Segments);
    }
}
