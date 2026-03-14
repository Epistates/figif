//! UI rendering components.

mod export;
mod help;
pub mod operations_menu;
mod preview;
pub mod timeline;

use crate::app::{App, Mode, ViewMode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph};

/// Main render function - NEW timeline-centric layout.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // NEW LAYOUT: impact header (2), preview (fills), timeline (6), footer (1)
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(2), // Impact header with size/duration
        Constraint::Fill(1),   // Preview + content area
        Constraint::Length(1), // Footer keybinds
    ])
    .areas(area);

    render_impact_header(app, frame, header_area);

    // Only render main body if not loading
    if !app.loading_file {
        render_main_body(app, frame, body_area);
    }

    render_footer(app, frame, footer_area);

    // Render loading overlay if active
    if app.loading_file {
        render_loading_overlay(app, frame, area);
    }

    // Render other overlays
    match &app.mode {
        Mode::Help => {
            help::render(app, frame, centered_rect(60, 80, area));
        }
        Mode::Input(kind) => {
            render_input_dialog(app, frame, kind, centered_rect(60, 20, area));
        }
        Mode::Export(state) => {
            export::render(app, frame, state, centered_rect(60, 50, area));
        }
        Mode::OperationsMenu => {
            operations_menu::render(app, frame, centered_rect(50, 40, area));
        }
        Mode::Normal => {}
    }
}

/// Render the impact header showing file info and live size/duration changes.
fn render_impact_header(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;

    let (file_info, impact_info) = if let Some(path) = &app.file_path {
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        if app.loading_file {
            (format!(" {} ", name), "Analyzing...".to_string())
        } else if let Some(analysis) = &app.analysis {
            // Calculate live impact from operations
            let stats = app.get_preview_stats();

            let file_part = format!(" {} ", name);

            let impact_part = if let Some(stats) = stats {
                let has_changes = stats.result_frames != stats.original_frames
                    || stats.result_duration != stats.original_duration;

                if has_changes {
                    let frame_delta = stats.original_frames as i64 - stats.result_frames as i64;
                    let duration_delta =
                        stats.original_duration as i64 - stats.result_duration as i64;
                    let frame_pct = if stats.original_frames > 0 {
                        (frame_delta as f64 / stats.original_frames as f64 * 100.0) as i64
                    } else {
                        0
                    };

                    format!(
                        "{}fr → {}fr (−{}%) │ {} → {} (−{}ms)",
                        stats.original_frames,
                        stats.result_frames,
                        frame_pct,
                        format_duration(stats.original_duration),
                        format_duration(stats.result_duration),
                        duration_delta
                    )
                } else {
                    format!(
                        "{}x{} │ {} frames │ {}",
                        analysis.metadata.width,
                        analysis.metadata.height,
                        analysis.frame_count(),
                        format_duration(analysis.total_duration_ms())
                    )
                }
            } else {
                format!(
                    "{}x{} │ {} frames │ {}",
                    analysis.metadata.width,
                    analysis.metadata.height,
                    analysis.frame_count(),
                    format_duration(analysis.total_duration_ms())
                )
            };

            (file_part, impact_part)
        } else {
            (format!(" {} ", name), "Error".to_string())
        }
    } else {
        (" No file loaded ".to_string(), String::new())
    };

    // Status indicator
    let status = if app.loading_file {
        ("● Loading", theme.warning())
    } else if !app.operations.is_empty() || !app.frame_operations.is_empty() {
        ("● Modified", theme.warning())
    } else {
        ("● Ready", theme.success())
    };

    // Hash algorithm indicator
    let hash_info = format!("[{}]", app.hash_algorithm.name());

    // Build two-line header
    let line1 = Line::from(vec![
        Span::styled(" figif ", theme.highlight()),
        Span::styled(file_info, theme.normal()),
    ]);

    let line2 = Line::from(vec![
        Span::styled(" ", theme.dim()),
        Span::styled(impact_info, theme.normal()),
        Span::raw("  "),
        Span::styled(hash_info, theme.dim()),
        Span::raw("  "),
        Span::styled(status.0, status.1),
    ]);

    let widget = Paragraph::new(vec![line1, line2]).style(theme.header());
    frame.render_widget(widget, area);
}

/// Render main body with preview and timeline.
fn render_main_body(app: &mut App, frame: &mut Frame, area: Rect) {
    // Split into preview area and timeline area
    let [preview_area, timeline_area] = Layout::vertical([
        Constraint::Fill(1),   // Preview fills available space
        Constraint::Length(6), // Timeline (title + bar + cursor + info)
    ])
    .areas(area);

    // Render centered preview
    frame.render_widget(Clear, preview_area);
    preview::render(app, frame, preview_area, true);

    // Render timeline
    timeline::render(app, frame, timeline_area);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;

    // Show success message if any
    let content = if let Some(success) = &app.success {
        Line::from(vec![Span::styled(
            format!(" OK: {}", success),
            theme.success(),
        )])
    // Show error if any
    } else if let Some(error) = &app.error {
        Line::from(vec![Span::styled(
            format!(" ERROR: {}", error),
            theme.error(),
        )])
    // Frame view keybindings (zoomed in)
    } else if matches!(app.view_mode, ViewMode::Frames { .. }) {
        let mut spans = vec![Span::raw(" ")];
        spans.push(Span::styled("←→", theme.keybind()));
        spans.push(Span::styled(":move ", theme.keybind_desc()));
        spans.push(Span::styled("Space", theme.keybind()));
        spans.push(Span::styled(":sel ", theme.keybind_desc()));
        spans.push(Span::styled("j/↓", theme.keybind()));
        spans.push(Span::styled(":back ", theme.keybind_desc()));
        spans.push(Span::styled("r", theme.keybind()));
        spans.push(Span::styled(":rm ", theme.keybind_desc()));
        spans.push(Span::styled("s", theme.keybind()));
        spans.push(Span::styled(":split ", theme.keybind_desc()));
        spans.push(Span::styled("d", theme.keybind()));
        spans.push(Span::styled(":dupes ", theme.keybind_desc()));
        spans.push(Span::styled("+/-", theme.keybind()));
        spans.push(Span::styled(":zoom ", theme.keybind_desc()));

        let has_ops = !app.frame_operations.is_empty() || !app.operations.is_empty();
        if has_ops {
            spans.push(Span::styled("u", theme.keybind()));
            spans.push(Span::styled(":undo ", theme.keybind_desc()));
            spans.push(Span::styled("U", theme.keybind()));
            spans.push(Span::styled(":clear ", theme.keybind_desc()));
        }

        spans.push(Span::styled("?", theme.keybind()));
        spans.push(Span::styled(":help", theme.keybind_desc()));
        Line::from(spans)
    } else if app.analysis.is_some() {
        // Segment view keybindings (default timeline view)
        let mut spans = vec![Span::raw(" ")];

        // Navigation
        spans.push(Span::styled("←→", theme.keybind()));
        spans.push(Span::styled(":move ", theme.keybind_desc()));
        spans.push(Span::styled("Space", theme.keybind()));
        spans.push(Span::styled(":select ", theme.keybind_desc()));
        spans.push(Span::styled("k/↑", theme.keybind()));
        spans.push(Span::styled(":zoom ", theme.keybind_desc()));

        // Operations
        spans.push(Span::styled("r", theme.keybind()));
        spans.push(Span::styled(":remove ", theme.keybind_desc()));
        spans.push(Span::styled("o", theme.keybind()));
        spans.push(Span::styled(":ops ", theme.keybind_desc()));

        // Preview zoom
        spans.push(Span::styled("+/-", theme.keybind()));
        spans.push(Span::styled(":zoom ", theme.keybind_desc()));

        // Show undo/export if ops exist
        let has_ops = !app.operations.is_empty() || !app.frame_operations.is_empty();
        if has_ops {
            spans.push(Span::styled("u", theme.keybind()));
            spans.push(Span::styled(":undo ", theme.keybind_desc()));
            spans.push(Span::styled("U", theme.keybind()));
            spans.push(Span::styled(":clear ", theme.keybind_desc()));
            spans.push(Span::styled("e", theme.keybind()));
            spans.push(Span::styled(":export ", theme.keybind_desc()));
        }

        spans.push(Span::styled("?", theme.keybind()));
        spans.push(Span::styled(":help", theme.keybind_desc()));

        Line::from(spans)
    } else {
        // No file loaded
        let mut spans = vec![Span::raw(" ")];
        spans.push(Span::styled("?", theme.keybind()));
        spans.push(Span::styled(":help ", theme.keybind_desc()));
        spans.push(Span::styled("q", theme.keybind()));
        spans.push(Span::styled(":quit", theme.keybind_desc()));
        Line::from(spans)
    };

    let widget = Paragraph::new(content).style(theme.footer());
    frame.render_widget(widget, area);
}

fn render_input_dialog(app: &App, frame: &mut Frame, kind: &crate::actions::InputKind, area: Rect) {
    let theme = &app.theme;

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(kind.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        // Description
        Line::from(vec![Span::styled(kind.description(), theme.dim())]),
        Line::from(""),
        // Input field
        Line::from(vec![
            Span::styled(kind.prompt(), theme.normal()),
            Span::styled(&app.input_buffer, theme.highlight()),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
        ]),
        Line::from(""),
        // Instructions
        Line::from(vec![
            Span::styled("Enter", theme.keybind()),
            Span::styled(": confirm  ", theme.keybind_desc()),
            Span::styled("Esc", theme.keybind()),
            Span::styled(": cancel", theme.keybind_desc()),
        ]),
    ];

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, inner);
}

/// Create a centered rect of given percentage size.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

/// Format duration in milliseconds to human readable.
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) as f64 / 1000.0;
        format!("{}m {:.1}s", mins, secs)
    }
}

/// Render a loading overlay when a file is being analyzed.
fn render_loading_overlay(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;
    let loading_area = centered_rect(40, 30, area);

    frame.render_widget(Clear, loading_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.highlight())
        .title(" Analyzing GIF ");

    // Split loading area into text and progress bar
    let [text_area, progress_area] =
        Layout::vertical([Constraint::Length(6), Constraint::Length(3)])
            .margin(1)
            .areas(loading_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled("Please wait while we perform", theme.normal())),
        Line::from(Span::styled("perceptual analysis...", theme.normal())),
        Line::from(""),
        Line::from(Span::styled("This can take a few seconds", theme.dim())),
        Line::from(Span::styled("for large GIFs.", theme.dim())),
    ];

    let paragraph = Paragraph::new(text).alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(block, loading_area);
    frame.render_widget(paragraph, text_area);

    // Render progress bar
    let (current, total) = app.loading_progress;
    let percent = if total > 0 {
        (current as f64 / total as f64 * 100.0) as u16
    } else {
        0
    };

    let label = if total > 0 {
        format!("{} / {} frames ({}%)", current, total, percent)
    } else {
        "Preparing...".to_string()
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(theme.highlight())
        .percent(percent)
        .label(label);

    frame.render_widget(gauge, progress_area);
}
