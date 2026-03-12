//! Operations menu overlay for quick optimization presets.

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Menu option for optimization presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizeOption {
    RemoveAllStatic,
    RemoveStaticLong,
    CapPauses,
    CollapseStatic,
    SpeedUpAll,
    SpeedUpPauses,
}

impl OptimizeOption {
    /// Get all options in order.
    pub fn all() -> &'static [OptimizeOption] {
        &[
            OptimizeOption::RemoveAllStatic,
            OptimizeOption::RemoveStaticLong,
            OptimizeOption::CapPauses,
            OptimizeOption::CollapseStatic,
            OptimizeOption::SpeedUpAll,
            OptimizeOption::SpeedUpPauses,
        ]
    }

    /// Get the display label for this option.
    pub fn label(&self) -> &'static str {
        match self {
            OptimizeOption::RemoveAllStatic => "Remove all static segments",
            OptimizeOption::RemoveStaticLong => "Remove static > 300ms",
            OptimizeOption::CapPauses => "Cap all pauses to 500ms",
            OptimizeOption::CollapseStatic => "Collapse static to 100ms",
            OptimizeOption::SpeedUpAll => "Speed up entire GIF 1.5x",
            OptimizeOption::SpeedUpPauses => "Speed up pauses only 2x",
        }
    }

    /// Get the key for this option (1-6).
    pub fn key(&self) -> char {
        match self {
            OptimizeOption::RemoveAllStatic => '1',
            OptimizeOption::RemoveStaticLong => '2',
            OptimizeOption::CapPauses => '3',
            OptimizeOption::CollapseStatic => '4',
            OptimizeOption::SpeedUpAll => '5',
            OptimizeOption::SpeedUpPauses => '6',
        }
    }

    /// Get option from key press.
    pub fn from_key(c: char) -> Option<OptimizeOption> {
        match c {
            '1' => Some(OptimizeOption::RemoveAllStatic),
            '2' => Some(OptimizeOption::RemoveStaticLong),
            '3' => Some(OptimizeOption::CapPauses),
            '4' => Some(OptimizeOption::CollapseStatic),
            '5' => Some(OptimizeOption::SpeedUpAll),
            '6' => Some(OptimizeOption::SpeedUpPauses),
            _ => None,
        }
    }
}

/// Render the operations menu overlay.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let theme = &app.theme;

    // Clear background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Optimize ")
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build menu lines
    let mut lines = Vec::new();

    // Calculate impact preview for each option
    for option in OptimizeOption::all() {
        let impact = calculate_impact_preview(app, *option);
        let line = Line::from(vec![
            Span::styled(format!(" {}. ", option.key()), theme.keybind()),
            Span::styled(option.label(), theme.normal()),
            Span::styled(format!("  {}", impact), theme.dim()),
        ]);
        lines.push(line);
    }

    // Separator
    lines.push(Line::from(""));

    // Instructions
    lines.push(Line::from(vec![
        Span::styled(" 1-6", theme.keybind()),
        Span::styled(": apply  ", theme.keybind_desc()),
        Span::styled("Esc", theme.keybind()),
        Span::styled(": cancel", theme.keybind_desc()),
    ]));

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, inner);
}

/// Calculate a preview of the impact for an optimization option.
fn calculate_impact_preview(app: &App, option: OptimizeOption) -> String {
    let Some(analysis) = &app.analysis else {
        return String::new();
    };

    match option {
        OptimizeOption::RemoveAllStatic => {
            let static_count = analysis.segments.iter().filter(|s| s.is_static).count();
            let static_duration: u64 = analysis
                .segments
                .iter()
                .filter(|s| s.is_static)
                .map(|s| s.duration_ms() as u64)
                .sum();
            if static_count > 0 {
                format!("(−{} segs, −{}ms)", static_count, static_duration)
            } else {
                "(no static segments)".to_string()
            }
        }
        OptimizeOption::RemoveStaticLong => {
            let long_static_count = analysis
                .segments
                .iter()
                .filter(|s| s.is_static && s.duration_ms() > 300)
                .count();
            let long_static_duration: u64 = analysis
                .segments
                .iter()
                .filter(|s| s.is_static && s.duration_ms() > 300)
                .map(|s| s.duration_ms() as u64)
                .sum();
            if long_static_count > 0 {
                format!("(−{} segs, −{}ms)", long_static_count, long_static_duration)
            } else {
                "(none > 300ms)".to_string()
            }
        }
        OptimizeOption::CapPauses => {
            let affected_count = analysis
                .segments
                .iter()
                .filter(|s| s.is_static && s.duration_ms() > 500)
                .count();
            if affected_count > 0 {
                format!("({} segs affected)", affected_count)
            } else {
                "(none > 500ms)".to_string()
            }
        }
        OptimizeOption::CollapseStatic => {
            let static_count = analysis.segments.iter().filter(|s| s.is_static).count();
            let static_frames: usize = analysis
                .segments
                .iter()
                .filter(|s| s.is_static)
                .map(|s| s.frame_count())
                .sum();
            if static_count > 0 {
                format!("({}fr → {}fr)", static_frames, static_count)
            } else {
                "(no static segments)".to_string()
            }
        }
        OptimizeOption::SpeedUpAll => {
            let total = analysis.total_duration_ms();
            let new_total = (total as f64 / 1.5) as u64;
            format!("({}ms → {}ms)", total, new_total)
        }
        OptimizeOption::SpeedUpPauses => {
            let static_duration: u64 = analysis
                .segments
                .iter()
                .filter(|s| s.is_static)
                .map(|s| s.duration_ms() as u64)
                .sum();
            let new_duration = static_duration / 2;
            let saved = static_duration - new_duration;
            if saved > 0 {
                format!("(−{}ms from pauses)", saved)
            } else {
                "(no pauses)".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_from_key() {
        assert_eq!(
            OptimizeOption::from_key('1'),
            Some(OptimizeOption::RemoveAllStatic)
        );
        assert_eq!(
            OptimizeOption::from_key('6'),
            Some(OptimizeOption::SpeedUpPauses)
        );
        assert_eq!(OptimizeOption::from_key('7'), None);
    }
}
