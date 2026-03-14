//! Application state and main event loop.

#![allow(dead_code)]

use crate::actions::{Action, ExportState, InputKind, OperationSummary};
use crate::theme::Theme;
use crate::ui;
use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use figif_core::hashers::{BlockHasher, DHasher, PHasher};
use figif_core::prelude::*;

/// Available hash algorithms for frame similarity detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::enum_variant_names)]
pub enum HashAlgorithm {
    /// Difference hash - fast, good for near-duplicates (default)
    #[default]
    DHash,
    /// Perceptual hash (DCT) - robust to transformations
    PHash,
    /// Block average hash - balanced approach
    BlockHash,
}

impl HashAlgorithm {
    /// Get the next algorithm in the cycle.
    pub fn next(self) -> Self {
        match self {
            HashAlgorithm::DHash => HashAlgorithm::PHash,
            HashAlgorithm::PHash => HashAlgorithm::BlockHash,
            HashAlgorithm::BlockHash => HashAlgorithm::DHash,
        }
    }

    /// Get display name.
    pub fn name(self) -> &'static str {
        match self {
            HashAlgorithm::DHash => "dHash",
            HashAlgorithm::PHash => "pHash",
            HashAlgorithm::BlockHash => "blockHash",
        }
    }

    /// Get short description.
    pub fn description(self) -> &'static str {
        match self {
            HashAlgorithm::DHash => "Fast, good for duplicates",
            HashAlgorithm::PHash => "Robust to transforms",
            HashAlgorithm::BlockHash => "Balanced approach",
        }
    }
}
use figif_core::FrameOps;

use ratatui::{DefaultTerminal, Frame};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Application mode / overlay state.
#[derive(Default, Clone)]
pub enum Mode {
    #[default]
    Normal,
    Help,
    Input(InputKind),
    Export(ExportState),
    OperationsMenu,
}

impl PartialEq for Mode {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Mode::Normal, Mode::Normal)
                | (Mode::Help, Mode::Help)
                | (Mode::Input(_), Mode::Input(_))
                | (Mode::Export(_), Mode::Export(_))
                | (Mode::OperationsMenu, Mode::OperationsMenu)
        )
    }
}

impl Eq for Mode {}

/// View mode for navigating between segment list and frame list (tree view).
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Top-level segment list (default).
    #[default]
    Segments,
    /// Drilling into a specific segment to see individual frames.
    Frames {
        /// The segment ID being viewed.
        segment_id: usize,
    },
}

/// App state for rendering tracking.
#[derive(Clone, Copy, PartialEq)]
pub struct RenderState {
    pub frame_index: usize,
    pub preview_scale: f32,
}

/// Main application state.
pub struct App {
    /// Current mode
    pub mode: Mode,
    /// View mode (segment list vs frame list)
    pub view_mode: ViewMode,
    /// Loaded GIF analysis (type-erased for simplicity)
    pub analysis: Option<figif_core::Analysis<img_hash::ImageHash>>,
    /// File path
    pub file_path: Option<PathBuf>,
    /// Currently selected segment index
    pub selected_segment: usize,
    /// Selected segment IDs for batch operations
    pub selected_segments: HashSet<usize>,
    /// Pending operations
    pub operations: SegmentOps,
    /// Frame-level operations (segment_id, frame_idx) -> FrameOp
    pub frame_operations: FrameOps,
    /// Currently selected frame index within segment (for frame view)
    pub selected_frame: usize,
    /// Selected frame indices for batch operations in frame view
    pub selected_frames: HashSet<usize>,
    /// Undo history
    pub history: Vec<SegmentOps>,
    /// Frame operation history for undo
    pub frame_history: Vec<FrameOps>,
    /// Error message
    pub error: Option<String>,
    /// Success message (for export confirmation)
    pub success: Option<String>,
    /// Should exit
    pub should_quit: bool,
    /// Theme
    pub theme: Theme,
    /// Similarity threshold
    pub threshold: u32,
    /// Current hash algorithm
    pub hash_algorithm: HashAlgorithm,
    /// Input buffer for text input
    pub input_buffer: String,
    /// Scroll offset for segments list
    pub scroll_offset: usize,
    /// Current frame index within segment (for preview)
    pub preview_frame_index: usize,
    /// Whether preview is playing (auto-advancing)
    pub preview_playing: bool,
    /// Image protocol picker for terminal graphics
    pub picker: Option<Picker>,
    /// Current image protocol state for rendering
    pub image_state: Option<StatefulProtocol>,
    /// Last rendered state (to avoid re-encoding)
    pub last_rendered_state: Option<RenderState>,
    /// Preview zoom scale (1.0 = fit to area, >1.0 = zoomed in)
    pub preview_scale: f32,
    /// Whether terminal capability detection is in progress
    pub picker_loading: bool,
    /// Whether a file is currently being loaded/analyzed
    pub loading_file: bool,
    /// Current loading progress (current, total)
    pub loading_progress: (usize, usize),
    /// Channel for background tasks to send actions
    pub action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    /// Channel for background tasks to receive actions (internal)
    pub action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
}

impl App {
    pub fn new(threshold: u32) -> Self {
        let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            mode: Mode::Normal,
            view_mode: ViewMode::Segments,
            analysis: None,
            file_path: None,
            selected_segment: 0,
            selected_segments: HashSet::new(),
            operations: SegmentOps::new(),
            frame_operations: FrameOps::new(),
            selected_frame: 0,
            selected_frames: HashSet::new(),
            history: Vec::new(),
            frame_history: Vec::new(),
            error: None,
            success: None,
            should_quit: false,
            theme: Theme::default(),
            threshold,
            hash_algorithm: HashAlgorithm::default(),
            input_buffer: String::new(),
            scroll_offset: 0,
            preview_frame_index: 0,
            preview_playing: false,
            picker: None,
            image_state: None,
            last_rendered_state: None,
            preview_scale: 1.0,
            picker_loading: false,
            loading_file: false,

            loading_progress: (0, 0),
            action_tx,
            action_rx,
        }
    }

    /// Start loading a GIF file for analysis asynchronously.
    pub fn load_file(&mut self, path: PathBuf) {
        self.loading_file = true;
        self.loading_progress = (0, 0);
        self.file_path = Some(path.clone());
        self.error = None;

        let tx = self.action_tx.clone();
        let threshold = self.threshold;
        let algorithm = self.hash_algorithm;

        tokio::task::spawn_blocking(move || {
            let progress_tx = tx.clone();
            let callback = Arc::new(move |current, total| {
                let _ = progress_tx.send(Action::LoadingProgress(current, total));
            });

            let result = match algorithm {
                HashAlgorithm::DHash => Figif::new()
                    .with_hasher(DHasher::new())
                    .similarity_threshold(threshold)
                    .with_progress_callback(callback)
                    .analyze_file(&path),
                HashAlgorithm::PHash => Figif::new()
                    .with_hasher(PHasher::new())
                    .similarity_threshold(threshold)
                    .with_progress_callback(callback)
                    .analyze_file(&path),
                HashAlgorithm::BlockHash => Figif::new()
                    .with_hasher(BlockHasher::new())
                    .similarity_threshold(threshold)
                    .with_progress_callback(callback)
                    .analyze_file(&path),
            };

            let mapped_result = result.map_err(|e: figif_core::FigifError| e.to_string());
            let _ = tx.send(Action::AnalysisResult(mapped_result));
        });
    }

    /// Complete the loading process with the analysis result.
    fn finish_loading(
        &mut self,
        result: Result<figif_core::Analysis<img_hash::ImageHash>, String>,
    ) {
        self.loading_file = false;

        match result {
            Ok(analysis) => {
                self.analysis = Some(analysis);
                self.view_mode = ViewMode::Segments;
                self.selected_segment = 0;
                self.selected_segments.clear();
                self.operations = SegmentOps::new();
                self.frame_operations = FrameOps::new();
                self.selected_frame = 0;
                self.selected_frames.clear();
                self.history.clear();
                self.frame_history.clear();
                self.error = None;
                self.success = None;
                self.scroll_offset = 0;
                self.preview_frame_index = 0;
                self.preview_playing = false;
                self.image_state = None;
                self.last_rendered_state = None;
            }
            Err(err) => {
                self.error = Some(format!("Failed to load GIF: {}", err));
            }
        }
    }

    /// Get the current frame index to display in preview.
    pub fn get_preview_frame_index(&self) -> Option<usize> {
        let analysis = self.analysis.as_ref()?;
        if self.selected_segment >= analysis.segments.len() {
            return None;
        }

        let segment = &analysis.segments[self.selected_segment];
        let frame_idx = segment.frame_range.start + self.preview_frame_index;

        if frame_idx < segment.frame_range.end {
            Some(frame_idx)
        } else {
            Some(segment.frame_range.start)
        }
    }

    /// Update image state for current preview frame.
    pub fn update_preview_image(&mut self) {
        let Some(picker) = &self.picker else { return };
        let Some(frame_idx) = self.get_preview_frame_index() else {
            return;
        };

        // Check both frame and scale to decide if re-render is needed
        if self.last_rendered_state
            == Some(RenderState {
                frame_index: frame_idx,
                preview_scale: self.preview_scale,
            })
        {
            return;
        }

        let Some(analysis) = &self.analysis else {
            return;
        };
        if frame_idx >= analysis.frames.len() {
            return;
        }

        // Get the frame image and convert to DynamicImage
        let rgba_image = &analysis.frames[frame_idx].frame.image;
        let mut dyn_image = image::DynamicImage::ImageRgba8(rgba_image.clone());

        // Apply magnification zoom by cropping the source image
        // This is safer for terminal protocols than scaling the render area
        if self.preview_scale > 1.0 {
            use image::GenericImageView;
            let (w, h) = dyn_image.dimensions();
            let crop_w = (w as f32 / self.preview_scale) as u32;
            let crop_h = (h as f32 / self.preview_scale) as u32;
            let x = (w.saturating_sub(crop_w)) / 2;
            let y = (h.saturating_sub(crop_h)) / 2;
            dyn_image = dyn_image.crop_imm(x, y, crop_w, crop_h);
        }

        // Create protocol state for this image
        self.image_state = Some(picker.new_resize_protocol(dyn_image));
        self.last_rendered_state = Some(RenderState {
            frame_index: frame_idx,
            preview_scale: self.preview_scale,
        });
    }

    /// Advance preview frame for motion segments.
    pub fn advance_preview_frame(&mut self) {
        let Some(analysis) = &self.analysis else {
            return;
        };
        if self.selected_segment >= analysis.segments.len() {
            return;
        }

        let segment = &analysis.segments[self.selected_segment];
        let frame_count = segment.frame_count();

        if frame_count > 1 {
            self.preview_frame_index = (self.preview_frame_index + 1) % frame_count;
            self.update_preview_image();
        }
    }

    /// Go to previous preview frame for motion segments.
    pub fn prev_preview_frame(&mut self) {
        let Some(analysis) = &self.analysis else {
            return;
        };
        if self.selected_segment >= analysis.segments.len() {
            return;
        }

        let segment = &analysis.segments[self.selected_segment];
        let frame_count = segment.frame_count();

        if frame_count > 1 {
            self.preview_frame_index = if self.preview_frame_index == 0 {
                frame_count - 1
            } else {
                self.preview_frame_index - 1
            };
            self.update_preview_image();
        }
    }

    /// Main application loop.
    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        use crossterm::event::EventStream;
        use futures::StreamExt;

        // Perform dynamic capability detection after alternate screen/raw mode is active
        // Do this in background to avoid blocking the first few renders
        if self.picker.is_none() && !self.picker_loading {
            self.picker_loading = true;
            let tx = self.action_tx.clone();
            tokio::task::spawn_blocking(move || {
                let picker = Picker::from_query_stdio().ok().or_else(|| {
                    // Fallback to halfblocks if protocol query fails
                    Some(Picker::halfblocks())
                });
                let _ = tx.send(Action::PickerInitialized(picker));
            });
        }

        let mut events = EventStream::new();
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        loop {
            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

            if self.should_quit {
                break;
            }

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            tokio::select! {
                // Handle system events (keys, etc)
                maybe_event = events.next() => {
                    if let Some(Ok(event)) = maybe_event {
                        match event {
                            Event::Key(key) if key.kind == KeyEventKind::Press => {
                                if let Some(action) = self.handle_key(key) {
                                    self.update(action);
                                }
                            }
                            Event::Resize(_, _) => {
                                // Refresh picker on resize to ensure cell size is accurate
                                self.picker = Picker::from_query_stdio().ok().or_else(|| {
                                    Some(Picker::halfblocks())
                                });
                                // Force re-render of image
                                self.last_rendered_state = None;
                            }
                            _ => {}
                        }
                    }
                }
                // Handle background task actions
                maybe_action = self.action_rx.recv() => {
                    if let Some(action) = maybe_action {
                        self.update(action);
                    }
                }
                // Graceful shutdown on Ctrl+C (as a signal)
                _ = tokio::signal::ctrl_c() => {
                    self.should_quit = true;
                }
                // Tick
                _ = tokio::time::sleep(timeout) => {
                    if last_tick.elapsed() >= tick_rate {
                        self.update(Action::Tick);
                        last_tick = Instant::now();
                    }
                }
            }
        }

        Ok(())
    }

    /// Render the UI.
    fn render(&mut self, frame: &mut Frame) {
        ui::render(self, frame);
    }

    /// Handle key events and return an action.
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Clear messages on any key
        self.error = None;
        self.success = None;

        // Handle input mode specially
        if let Mode::Input(kind) = &self.mode {
            return self.handle_input_key(key, *kind);
        }

        // Handle export mode
        if matches!(self.mode, Mode::Export(_)) {
            return self.handle_export_key(key);
        }

        // Handle help mode
        if self.mode == Mode::Help {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Some(Action::HideOverlay),
                _ => None,
            };
        }

        // Handle operations menu mode
        if self.mode == Mode::OperationsMenu {
            return self.handle_operations_menu_key(key);
        }

        // Handle frame view mode keybindings
        if matches!(self.view_mode, ViewMode::Frames { .. }) {
            return self.handle_frame_view_key(key);
        }

        // Segment view mode keybindings (timeline navigation)
        match key.code {
            // Quit
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Esc => Some(Action::HideOverlay),

            // Timeline navigation (horizontal: h/l and ←/→)
            KeyCode::Left | KeyCode::Char('h') => Some(Action::PrevSegment),
            KeyCode::Right | KeyCode::Char('l') => Some(Action::NextSegment),
            KeyCode::Home | KeyCode::Char('g') => Some(Action::FirstSegment),
            KeyCode::End | KeyCode::Char('G') => Some(Action::LastSegment),

            // Timeline zoom (drill into frames: ↑/k/Enter, exit: ↓/j)
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Enter => Some(Action::EnterFrameView),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::ExitFrameView),

            // Preview zoom (+/= zoom in, - zoom out, 0 reset)
            KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::ZoomPreviewIn),
            KeyCode::Char('-') => Some(Action::ZoomPreviewOut),
            KeyCode::Char('0') => Some(Action::ResetPreviewZoom),

            // Selection
            KeyCode::Char(' ') => Some(Action::ToggleSelected),
            KeyCode::Char('a') => Some(Action::SelectAll),
            KeyCode::Char('A') => Some(Action::DeselectAll),
            KeyCode::Char('s') => Some(Action::SelectStatic),
            KeyCode::Char('m') => Some(Action::SelectMotion),

            // Operations
            KeyCode::Char('c') => Some(Action::ShowInput(InputKind::CapDuration)),
            KeyCode::Char('C') => Some(Action::ShowInput(InputKind::CollapseDuration)),
            KeyCode::Char('r') => Some(Action::ToggleRemove),
            KeyCode::Char('x') => Some(Action::ClearOperation),
            KeyCode::Char('u') => Some(Action::Undo),
            KeyCode::Char('U') => Some(Action::ResetAll),

            // Operations menu
            KeyCode::Char('o') => Some(Action::ShowOperationsMenu),

            // Preview playback
            KeyCode::Char('p') => Some(Action::TogglePlayback),

            // Settings
            KeyCode::Char('H') => Some(Action::CycleHashAlgorithm),

            // File
            KeyCode::Char('e') => Some(Action::ShowExport),

            // Help
            KeyCode::Char('?') => Some(Action::ShowHelp),

            _ => None,
        }
    }

    /// Handle key events in operations menu mode.
    fn handle_operations_menu_key(&mut self, key: KeyEvent) -> Option<Action> {
        use crate::ui::operations_menu::OptimizeOption;

        match key.code {
            KeyCode::Esc => Some(Action::CloseOperationsMenu),
            KeyCode::Char(c) if ('1'..='6').contains(&c) => {
                OptimizeOption::from_key(c).map(Action::ApplyOptimization)
            }
            _ => None,
        }
    }

    /// Handle key events in export mode.
    fn handle_export_key(&mut self, key: KeyEvent) -> Option<Action> {
        let Mode::Export(state) = &mut self.mode else {
            return None;
        };

        match key.code {
            KeyCode::Enter => Some(Action::ConfirmExport),
            KeyCode::Esc => Some(Action::CancelExport),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::DiscardChanges)
            }
            KeyCode::Backspace => {
                if state.cursor > 0 {
                    state.path.remove(state.cursor - 1);
                    state.cursor -= 1;
                    state.check_file_exists();
                    state.confirmed_overwrite = false;
                }
                None
            }
            KeyCode::Delete => {
                if state.cursor < state.path.len() {
                    state.path.remove(state.cursor);
                    state.check_file_exists();
                    state.confirmed_overwrite = false;
                }
                None
            }
            KeyCode::Left => {
                if state.cursor > 0 {
                    state.cursor -= 1;
                }
                None
            }
            KeyCode::Right => {
                if state.cursor < state.path.len() {
                    state.cursor += 1;
                }
                None
            }
            KeyCode::Home => {
                state.cursor = 0;
                None
            }
            KeyCode::End => {
                state.cursor = state.path.len();
                None
            }
            KeyCode::Char(c) => {
                state.path.insert(state.cursor, c);
                state.cursor += 1;
                state.check_file_exists();
                state.confirmed_overwrite = false;
                None
            }
            _ => None,
        }
    }

    /// Handle key events in input mode.
    fn handle_input_key(&mut self, key: KeyEvent, kind: InputKind) -> Option<Action> {
        match key.code {
            KeyCode::Enter => {
                let input = self.input_buffer.clone();
                self.input_buffer.clear();
                Some(Action::SubmitInput(input))
            }
            KeyCode::Esc => {
                self.input_buffer.clear();
                Some(Action::CancelInput)
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                None
            }
            KeyCode::Char(c) => {
                // Validate input based on kind
                let valid = match kind {
                    InputKind::CapDuration | InputKind::CollapseDuration => c.is_ascii_digit(),
                    InputKind::SpeedFactor => c.is_ascii_digit() || c == '.',
                    InputKind::DuplicateSelection => {
                        c.is_ascii_digit() || matches!(c, 's' | 'e' | 'b' | ':')
                    }
                };
                if valid {
                    self.input_buffer.push(c);
                }
                None
            }
            _ => None,
        }
    }

    /// Handle key events in frame view mode (zoomed into a segment).
    fn handle_frame_view_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            // Zoom out (↓, j, Esc, or Backspace)
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Esc | KeyCode::Backspace => {
                Some(Action::ExitFrameView)
            }
            KeyCode::Char('q') => Some(Action::Quit),

            // Timeline navigation (horizontal: h/l and ←/→)
            KeyCode::Left | KeyCode::Char('h') => Some(Action::PrevFrameInList),
            KeyCode::Right | KeyCode::Char('l') => Some(Action::NextFrameInList),
            KeyCode::Home | KeyCode::Char('g') => Some(Action::FirstFrameInList),
            KeyCode::End | KeyCode::Char('G') => Some(Action::LastFrameInList),

            // Preview zoom (+/= zoom in, - zoom out, 0 reset)
            KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::ZoomPreviewIn),
            KeyCode::Char('-') => Some(Action::ZoomPreviewOut),
            KeyCode::Char('0') => Some(Action::ResetPreviewZoom),

            // Selection
            KeyCode::Char(' ') => Some(Action::ToggleFrameSelected),
            KeyCode::Char('a') => Some(Action::SelectAllFrames),
            KeyCode::Char('A') => Some(Action::DeselectAllFrames),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::ShowInput(InputKind::DuplicateSelection))
            }
            KeyCode::Char('d') => Some(Action::SelectDuplicateFrames),

            // Frame operations
            KeyCode::Char('r') => Some(Action::ToggleRemoveFrame),
            KeyCode::Char('s') => Some(Action::ToggleSplitAfterFrame),
            KeyCode::Char('x') => Some(Action::ClearFrameOperation),

            // Undo (uses frame history)
            KeyCode::Char('u') => {
                if let Some(prev) = self.frame_history.pop() {
                    self.frame_operations = prev;
                }
                None
            }

            // Clear all operations
            KeyCode::Char('U') => Some(Action::ResetAll),

            // Preview playback
            KeyCode::Char('p') => Some(Action::TogglePlayback),

            // Help
            KeyCode::Char('?') => Some(Action::ShowHelp),

            _ => None,
        }
    }

    /// Update state based on action.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,

            Action::NextSegment => {
                if let Some(analysis) = &self.analysis
                    && self.selected_segment < analysis.segments.len().saturating_sub(1)
                {
                    self.selected_segment += 1;
                    self.preview_frame_index = 0;
                    self.image_state = None;
                    self.last_rendered_state = None;
                }
            }

            Action::PrevSegment => {
                if self.selected_segment > 0 {
                    self.selected_segment -= 1;
                    self.preview_frame_index = 0;
                    self.image_state = None;
                    self.last_rendered_state = None;
                }
            }

            Action::FirstSegment => {
                self.selected_segment = 0;
                self.preview_frame_index = 0;
                self.image_state = None;
                self.last_rendered_state = None;
            }

            Action::LastSegment => {
                if let Some(analysis) = &self.analysis {
                    self.selected_segment = analysis.segments.len().saturating_sub(1);
                    self.preview_frame_index = 0;
                    self.image_state = None;
                    self.last_rendered_state = None;
                }
            }

            Action::NextFrame => {
                self.advance_preview_frame();
            }

            Action::PrevFrame => {
                self.prev_preview_frame();
            }

            Action::FirstFrame => {
                self.preview_frame_index = 0;
                self.image_state = None;
                self.last_rendered_state = None;
            }

            Action::LastFrame => {
                if let Some(analysis) = &self.analysis
                    && self.selected_segment < analysis.segments.len()
                {
                    let segment = &analysis.segments[self.selected_segment];
                    let frame_count = segment.frame_count();
                    if frame_count > 0 {
                        self.preview_frame_index = frame_count - 1;
                        self.image_state = None;
                        self.last_rendered_state = None;
                    }
                }
            }

            Action::TogglePlayback => {
                self.preview_playing = !self.preview_playing;
            }

            // Frame view navigation (zoom into segment)
            Action::EnterFrameView => {
                if let Some(analysis) = &self.analysis
                    && self.selected_segment < analysis.segments.len()
                {
                    let segment_id = analysis.segments[self.selected_segment].id;
                    self.view_mode = ViewMode::Frames { segment_id };
                    self.selected_frame = 0;
                    self.selected_frames.clear();
                }
            }

            Action::ExitFrameView => {
                self.view_mode = ViewMode::Segments;
                self.selected_frame = 0;
                self.selected_frames.clear();
            }

            Action::NextFrameInList => {
                let frame_count = self.get_frame_view_frame_count();
                if frame_count > 0 && self.selected_frame < frame_count - 1 {
                    self.selected_frame += 1;
                    // Also update preview to show this frame
                    self.preview_frame_index = self.selected_frame;
                    self.image_state = None;
                    self.last_rendered_state = None;
                }
            }

            Action::PrevFrameInList => {
                if self.selected_frame > 0 {
                    self.selected_frame -= 1;
                    self.preview_frame_index = self.selected_frame;
                    self.image_state = None;
                    self.last_rendered_state = None;
                }
            }

            Action::FirstFrameInList => {
                self.selected_frame = 0;
                self.preview_frame_index = 0;
                self.image_state = None;
                self.last_rendered_state = None;
            }

            Action::LastFrameInList => {
                let frame_count = self.get_frame_view_frame_count();
                if frame_count > 0 {
                    self.selected_frame = frame_count - 1;
                    self.preview_frame_index = self.selected_frame;
                    self.image_state = None;
                    self.last_rendered_state = None;
                }
            }

            Action::ToggleFrameSelected => {
                if self.selected_frames.contains(&self.selected_frame) {
                    self.selected_frames.remove(&self.selected_frame);
                } else {
                    self.selected_frames.insert(self.selected_frame);
                }
            }

            Action::SelectAllFrames => {
                let frame_count = self.get_frame_view_frame_count();
                self.selected_frames = (0..frame_count).collect();
            }

            Action::DeselectAllFrames => {
                self.selected_frames.clear();
            }

            Action::SelectDuplicateFrames => {
                // Add frames identical to the highlighted frame to selection (doesn't clear)
                if let Some(segment_id) = self.get_frame_view_segment_id()
                    && let Some(analysis) = &self.analysis
                    && let Some(segment) = analysis.segments.iter().find(|s| s.id == segment_id)
                {
                    // Get the highlighted frame's hash
                    let highlighted_abs_idx = segment.frame_range.start + self.selected_frame;
                    if let Some(highlighted_frame) = analysis.frames.get(highlighted_abs_idx) {
                        let target_hash = &highlighted_frame.hash;

                        // Add all frames with matching hash to selection
                        for frame_idx in 0..segment.frame_count() {
                            let abs_frame_idx = segment.frame_range.start + frame_idx;
                            if let Some(frame) = analysis.frames.get(abs_frame_idx)
                                && target_hash.dist(&frame.hash) == 0
                            {
                                self.selected_frames.insert(frame_idx);
                            }
                        }
                    }
                }
            }

            Action::ToggleRemoveFrame => {
                if let Some(segment_id) = self.get_frame_view_segment_id() {
                    self.save_frame_history();
                    let target_frames = self.get_target_frame_indices();
                    for frame_idx in target_frames {
                        let key = (segment_id, frame_idx);
                        if self.frame_operations.get(&key) == Some(&FrameOp::Remove) {
                            self.frame_operations.remove(&key);
                        } else {
                            self.frame_operations.insert(key, FrameOp::Remove);
                        }
                    }
                }
            }

            Action::ToggleSplitAfterFrame => {
                if let Some(segment_id) = self.get_frame_view_segment_id() {
                    self.save_frame_history();
                    let target_frames = self.get_target_frame_indices();
                    for frame_idx in target_frames {
                        let key = (segment_id, frame_idx);
                        if self.frame_operations.get(&key) == Some(&FrameOp::SplitAfter) {
                            self.frame_operations.remove(&key);
                        } else {
                            self.frame_operations.insert(key, FrameOp::SplitAfter);
                        }
                    }
                }
            }

            Action::ClearFrameOperation => {
                if let Some(segment_id) = self.get_frame_view_segment_id() {
                    self.save_frame_history();
                    let target_frames = self.get_target_frame_indices();
                    for frame_idx in target_frames {
                        self.frame_operations.remove(&(segment_id, frame_idx));
                    }
                }
            }

            Action::ToggleSelected => {
                if let Some(analysis) = &self.analysis
                    && self.selected_segment < analysis.segments.len()
                {
                    let id = analysis.segments[self.selected_segment].id;
                    if self.selected_segments.contains(&id) {
                        self.selected_segments.remove(&id);
                    } else {
                        self.selected_segments.insert(id);
                    }
                }
            }

            Action::SelectAll => {
                if let Some(analysis) = &self.analysis {
                    for seg in &analysis.segments {
                        self.selected_segments.insert(seg.id);
                    }
                }
            }

            Action::DeselectAll => {
                self.selected_segments.clear();
            }

            Action::SelectStatic => {
                if let Some(analysis) = &self.analysis {
                    for seg in &analysis.segments {
                        if seg.is_static {
                            self.selected_segments.insert(seg.id);
                        }
                    }
                }
            }

            Action::SelectMotion => {
                if let Some(analysis) = &self.analysis {
                    for seg in &analysis.segments {
                        if !seg.is_static {
                            self.selected_segments.insert(seg.id);
                        }
                    }
                }
            }

            Action::CapDuration(ms) => {
                self.save_history();
                if let Some(analysis) = &self.analysis {
                    let new_ops = self.build_ops_for_selected(analysis, |_| SegmentOp::Collapse {
                        delay_cs: (ms / 10).min(u16::MAX as u32) as u16,
                    });
                    // Only cap segments longer than ms
                    for id in new_ops.keys() {
                        if let Some(seg) = analysis.segments.iter().find(|s| s.id == *id)
                            && seg.duration_ms() > ms
                        {
                            self.operations.insert(
                                *id,
                                SegmentOp::Collapse {
                                    delay_cs: (ms / 10) as u16,
                                },
                            );
                        }
                    }
                }
            }

            Action::CollapseDuration(ms) => {
                self.save_history();
                if let Some(analysis) = &self.analysis {
                    let new_ops = self.build_ops_for_selected(analysis, |_| SegmentOp::Collapse {
                        delay_cs: (ms / 10) as u16,
                    });
                    self.operations.extend(new_ops);
                }
            }

            Action::ToggleRemove => {
                self.save_history();
                let target_ids = self.get_target_segment_ids();
                for id in target_ids {
                    if self.operations.get(&id) == Some(&SegmentOp::Remove) {
                        // Toggle off - remove the operation
                        self.operations.remove(&id);
                    } else {
                        // Toggle on - add remove operation
                        self.operations.insert(id, SegmentOp::Remove);
                    }
                }
            }

            Action::ClearOperation => {
                self.save_history();
                let target_ids = self.get_target_segment_ids();
                for id in target_ids {
                    self.operations.remove(&id);
                }
            }

            Action::SpeedUp(factor) => {
                self.save_history();
                if let Some(analysis) = &self.analysis {
                    let new_ops = self.build_ops_for_selected(analysis, |_| SegmentOp::Scale {
                        factor: 1.0 / factor,
                    });
                    self.operations.extend(new_ops);
                }
            }

            Action::SlowDown(factor) => {
                self.save_history();
                if let Some(analysis) = &self.analysis {
                    let new_ops =
                        self.build_ops_for_selected(analysis, |_| SegmentOp::Scale { factor });
                    self.operations.extend(new_ops);
                }
            }

            Action::SetDuration(ms) => {
                self.save_history();
                if let Some(analysis) = &self.analysis {
                    let new_ops =
                        self.build_ops_for_selected(analysis, |_| SegmentOp::SetDuration {
                            total_cs: (ms / 10) as u16,
                        });
                    self.operations.extend(new_ops);
                }
            }

            Action::Undo => {
                if let Some(prev) = self.history.pop() {
                    self.operations = prev;
                }
            }

            Action::ResetAll => {
                self.save_history();
                self.save_frame_history();
                self.operations = SegmentOps::new();
                self.frame_operations = FrameOps::new();
                self.success = Some("All operations cleared".to_string());
            }

            // Preview zoom actions
            Action::ZoomPreviewIn => {
                self.preview_scale = (self.preview_scale * 1.25).min(4.0);
                // Force re-render at new scale
                self.image_state = None;
                self.last_rendered_state = None;
            }

            Action::ZoomPreviewOut => {
                self.preview_scale = (self.preview_scale / 1.25).max(0.25);
                // Force re-render at new scale
                self.image_state = None;
                self.last_rendered_state = None;
            }

            Action::ResetPreviewZoom => {
                self.preview_scale = 1.0;
                // Force re-render at new scale
                self.image_state = None;
                self.last_rendered_state = None;
            }

            Action::CycleHashAlgorithm => {
                self.hash_algorithm = self.hash_algorithm.next();
                // Re-analyze with new algorithm if we have a file loaded
                if let Some(path) = self.file_path.clone() {
                    self.load_file(path);
                    self.success = Some(format!(
                        "Switching to {} ({})",
                        self.hash_algorithm.name(),
                        self.hash_algorithm.description()
                    ));
                }
            }

            Action::ShowHelp => {
                self.mode = Mode::Help;
            }

            Action::HideOverlay => {
                self.mode = Mode::Normal;
            }

            Action::ShowInput(kind) => {
                self.input_buffer = kind.default_value().to_string();
                self.mode = Mode::Input(kind);
            }

            Action::SubmitInput(input) => {
                if let Mode::Input(kind) = &self.mode {
                    match kind {
                        InputKind::CapDuration => {
                            if let Ok(ms) = input.parse::<u32>() {
                                self.update(Action::CapDuration(ms));
                            }
                        }
                        InputKind::CollapseDuration => {
                            if let Ok(ms) = input.parse::<u32>() {
                                self.update(Action::CollapseDuration(ms));
                            }
                        }
                        InputKind::SpeedFactor => {
                            if let Ok(factor) = input.parse::<f64>()
                                && factor > 0.0
                            {
                                self.update(Action::SpeedUp(factor));
                            }
                        }
                        InputKind::DuplicateSelection => {
                            self.select_duplicates_with_config(&input);
                        }
                    }
                }
                self.mode = Mode::Normal;
            }

            Action::CancelInput => {
                self.input_buffer.clear();
                self.mode = Mode::Normal;
            }

            // Export dialog
            Action::ShowExport => {
                let has_ops = !self.operations.is_empty() || !self.frame_operations.is_empty();
                if self.analysis.is_some() && has_ops {
                    let default_path = self.get_default_export_path();
                    let summary = self.build_operation_summary();
                    self.mode = Mode::Export(ExportState::new(default_path, summary));
                }
            }

            Action::ConfirmExport => {
                if let Mode::Export(ref state) = self.mode.clone() {
                    if state.file_exists && !state.confirmed_overwrite {
                        // Need to confirm overwrite - update state
                        if let Mode::Export(ref mut s) = self.mode {
                            s.confirmed_overwrite = true;
                        }
                    } else {
                        // Proceed with export
                        let path = PathBuf::from(&state.path);
                        self.export_to_file(&path);
                        self.mode = Mode::Normal;
                    }
                }
            }

            Action::CancelExport => {
                self.mode = Mode::Normal;
            }

            Action::DiscardChanges => {
                self.save_history();
                self.operations = SegmentOps::new();
                self.mode = Mode::Normal;
            }

            Action::LoadFile(path) => {
                self.load_file(path);
            }

            Action::LoadingProgress(current, total) => {
                self.loading_progress = (current, total);
            }

            Action::AnalysisResult(result) => {
                self.finish_loading(result);
            }

            Action::PickerInitialized(picker) => {
                self.picker = picker;
                self.picker_loading = false;
                self.last_rendered_state = None; // Force re-render with new picker
            }

            Action::Export(path, _config) => {
                self.export_to_file(&path);
            }

            Action::Error(msg) => {
                self.error = Some(msg);
            }

            // Operations menu
            Action::ShowOperationsMenu => {
                if self.analysis.is_some() {
                    self.mode = Mode::OperationsMenu;
                }
            }

            Action::CloseOperationsMenu => {
                self.mode = Mode::Normal;
                // Debug: confirm the action was processed
                self.success = Some("Menu closed".to_string());
            }

            Action::ApplyOptimization(option) => {
                self.apply_optimization(option);
                self.mode = Mode::Normal;
            }

            Action::Tick => {
                // Auto-advance preview frame if playing
                if self.preview_playing {
                    self.update(Action::NextFrame);
                }
            }
        }
    }

    /// Apply an optimization option from the menu.
    fn apply_optimization(&mut self, option: crate::ui::operations_menu::OptimizeOption) {
        use crate::ui::operations_menu::OptimizeOption;

        let Some(analysis) = &self.analysis else {
            return;
        };

        // Collect segment info first to avoid borrow issues
        let segment_info: Vec<(usize, bool, u32)> = analysis
            .segments
            .iter()
            .map(|s| (s.id, s.is_static, s.duration_ms()))
            .collect();

        self.save_history();

        match option {
            OptimizeOption::RemoveAllStatic => {
                for (id, is_static, _) in &segment_info {
                    if *is_static {
                        self.operations.insert(*id, SegmentOp::Remove);
                    }
                }
            }
            OptimizeOption::RemoveStaticLong => {
                for (id, is_static, duration) in &segment_info {
                    if *is_static && *duration > 300 {
                        self.operations.insert(*id, SegmentOp::Remove);
                    }
                }
            }
            OptimizeOption::CapPauses => {
                for (id, is_static, duration) in &segment_info {
                    if *is_static && *duration > 500 {
                        self.operations
                            .insert(*id, SegmentOp::Collapse { delay_cs: 50 });
                    }
                }
            }
            OptimizeOption::CollapseStatic => {
                for (id, is_static, _) in &segment_info {
                    if *is_static {
                        self.operations
                            .insert(*id, SegmentOp::Collapse { delay_cs: 10 });
                    }
                }
            }
            OptimizeOption::SpeedUpAll => {
                for (id, _, _) in &segment_info {
                    self.operations
                        .insert(*id, SegmentOp::Scale { factor: 1.0 / 1.5 });
                }
            }
            OptimizeOption::SpeedUpPauses => {
                for (id, is_static, _) in &segment_info {
                    if *is_static {
                        self.operations
                            .insert(*id, SegmentOp::Scale { factor: 0.5 });
                    }
                }
            }
        }
    }

    /// Save current operations to history for undo.
    fn save_history(&mut self) {
        self.history.push(self.operations.clone());
        // Keep history limited
        if self.history.len() > 50 {
            self.history.remove(0);
        }
    }

    /// Save current frame operations to history for undo.
    fn save_frame_history(&mut self) {
        self.frame_history.push(self.frame_operations.clone());
        // Keep history limited
        if self.frame_history.len() > 50 {
            self.frame_history.remove(0);
        }
    }

    /// Get the current segment ID being viewed in frame mode.
    fn get_frame_view_segment_id(&self) -> Option<usize> {
        match self.view_mode {
            ViewMode::Frames { segment_id } => Some(segment_id),
            ViewMode::Segments => None,
        }
    }

    /// Get frame count in current segment being viewed.
    fn get_frame_view_frame_count(&self) -> usize {
        let Some(segment_id) = self.get_frame_view_segment_id() else {
            return 0;
        };
        let Some(analysis) = &self.analysis else {
            return 0;
        };
        analysis
            .segments
            .iter()
            .find(|s| s.id == segment_id)
            .map_or(0, |s| s.frame_count())
    }

    /// Select duplicates of the highlighted frame with configuration.
    /// Format: "s:N" (keep first N), "e:N" (keep last N), "b:N" (keep N on both sides), or just "N" (same as s:N)
    fn select_duplicates_with_config(&mut self, config: &str) {
        // Parse config: s:N, e:N, b:N, or just N
        let (keep_start, keep_end) = if let Some(rest) = config.strip_prefix("s:") {
            (rest.parse::<usize>().unwrap_or(1), 0)
        } else if let Some(rest) = config.strip_prefix("e:") {
            (0, rest.parse::<usize>().unwrap_or(1))
        } else if let Some(rest) = config.strip_prefix("b:") {
            let n = rest.parse::<usize>().unwrap_or(1);
            (n, n)
        } else {
            // Default: treat as keep first N
            (config.parse::<usize>().unwrap_or(1), 0)
        };

        let Some(segment_id) = self.get_frame_view_segment_id() else {
            return;
        };
        let Some(analysis) = &self.analysis else {
            return;
        };
        let Some(segment) = analysis.segments.iter().find(|s| s.id == segment_id) else {
            return;
        };

        // Get the highlighted frame's hash
        let highlighted_abs_idx = segment.frame_range.start + self.selected_frame;
        let Some(highlighted_frame) = analysis.frames.get(highlighted_abs_idx) else {
            return;
        };
        let target_hash = &highlighted_frame.hash;

        // Find all matching frame indices
        let mut matching_indices: Vec<usize> = Vec::new();
        for frame_idx in 0..segment.frame_count() {
            let abs_frame_idx = segment.frame_range.start + frame_idx;
            if let Some(frame) = analysis.frames.get(abs_frame_idx)
                && target_hash.dist(&frame.hash) == 0
            {
                matching_indices.push(frame_idx);
            }
        }

        // Skip first keep_start and last keep_end
        let total = matching_indices.len();
        if total <= keep_start + keep_end {
            return; // Nothing to select
        }

        for (i, &frame_idx) in matching_indices.iter().enumerate() {
            let skip_start = i < keep_start;
            let skip_end = i >= total - keep_end;
            if !skip_start && !skip_end {
                self.selected_frames.insert(frame_idx);
            }
        }
    }

    /// Get target frame indices for frame operations.
    fn get_target_frame_indices(&self) -> Vec<usize> {
        if !self.selected_frames.is_empty() {
            self.selected_frames.iter().copied().collect()
        } else {
            vec![self.selected_frame]
        }
    }

    /// Build operations for selected segments (or current if none selected).
    fn build_ops_for_selected<F>(
        &self,
        analysis: &figif_core::Analysis<img_hash::ImageHash>,
        op_fn: F,
    ) -> SegmentOps
    where
        F: Fn(&figif_core::Segment) -> SegmentOp,
    {
        let mut ops = SegmentOps::new();

        let target_ids: Vec<usize> = if self.selected_segments.is_empty() {
            // Apply to current segment only
            if self.selected_segment < analysis.segments.len() {
                vec![analysis.segments[self.selected_segment].id]
            } else {
                vec![]
            }
        } else {
            self.selected_segments.iter().copied().collect()
        };

        for id in target_ids {
            if let Some(seg) = analysis.segments.iter().find(|s| s.id == id) {
                ops.insert(id, op_fn(seg));
            }
        }

        ops
    }

    /// Export the modified GIF to a file.
    fn export_to_file(&mut self, path: &PathBuf) {
        if let Some(analysis) = &self.analysis {
            let encoder = StandardEncoder::new();
            let config = EncodeConfig::default();

            // Use the enhanced export that handles both segment and frame operations
            match analysis.export_to_file_with_frame_ops(
                &encoder,
                &self.operations,
                &self.frame_operations,
                path,
                &config,
            ) {
                Ok(()) => {
                    self.error = None;
                    self.success = Some(format!("Exported to {}", path.display()));
                }
                Err(e) => {
                    self.error = Some(format!("Export failed: {}", e));
                }
            }
        }
    }

    /// Get target segment IDs (selected segments or current segment).
    fn get_target_segment_ids(&self) -> Vec<usize> {
        if !self.selected_segments.is_empty() {
            self.selected_segments.iter().copied().collect()
        } else if let Some(analysis) = &self.analysis {
            if self.selected_segment < analysis.segments.len() {
                vec![analysis.segments[self.selected_segment].id]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    /// Get default export path based on input file.
    fn get_default_export_path(&self) -> String {
        if let Some(path) = &self.file_path {
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let parent = path.parent().unwrap_or(std::path::Path::new("."));
            parent
                .join(format!("{}-optimized.gif", stem))
                .to_string_lossy()
                .to_string()
        } else {
            "optimized.gif".to_string()
        }
    }

    /// Build operation summary for export dialog.
    fn build_operation_summary(&self) -> OperationSummary {
        let mut summary = OperationSummary::default();

        for op in self.operations.values() {
            match op {
                SegmentOp::Remove => summary.segments_removed += 1,
                SegmentOp::Collapse { .. } => summary.segments_collapsed += 1,
                SegmentOp::Scale { .. } => summary.segments_scaled += 1,
                SegmentOp::SetDuration { .. } => summary.segments_scaled += 1,
                SegmentOp::SetFrameDelay { .. } => summary.segments_scaled += 1,
                SegmentOp::Keep => {}
            }
        }

        if let Some(stats) = self.get_preview_stats() {
            summary.original_frames = stats.original_frames;
            summary.result_frames = stats.result_frames;
            summary.original_duration_ms = stats.original_duration;
            summary.result_duration_ms = stats.result_duration;
        }

        summary
    }

    /// Get current segment ID.
    pub fn current_segment_id(&self) -> Option<usize> {
        self.analysis.as_ref().and_then(|a| {
            if self.selected_segment < a.segments.len() {
                Some(a.segments[self.selected_segment].id)
            } else {
                None
            }
        })
    }

    /// Check if current segment has an operation.
    pub fn current_has_operation(&self) -> bool {
        self.current_segment_id()
            .is_some_and(|id| self.operations.contains_key(&id))
    }

    /// Get segment count.
    pub fn segment_count(&self) -> usize {
        self.analysis.as_ref().map_or(0, |a| a.segments.len())
    }

    /// Get context-aware keybindings for footer display.
    pub fn get_context_keybinds(&self) -> Vec<(&'static str, &'static str)> {
        let mut binds = vec![("q", "quit"), ("?", "help")];

        if self.analysis.is_some() {
            binds.push(("j/k", "nav"));
            binds.push(("Space", "select"));

            let has_selection = !self.selected_segments.is_empty();
            let has_segments = self.segment_count() > 0;

            if has_selection || has_segments {
                binds.push(("r", "remove"));
                binds.push(("c", "cap"));
            }

            if self.current_has_operation()
                || self
                    .selected_segments
                    .iter()
                    .any(|id| self.operations.contains_key(id))
            {
                binds.push(("x", "clear"));
            }

            if !self.operations.is_empty() {
                binds.push(("u", "undo"));
                binds.push(("e", "export"));
            }
        }

        binds
    }

    /// Get preview statistics.
    pub fn get_preview_stats(&self) -> Option<PreviewStats> {
        let analysis = self.analysis.as_ref()?;
        // Calculate impact without cloning images
        let (result_frames, result_duration) =
            analysis.calculate_impact(&self.operations, &self.frame_operations);

        let original_duration = analysis.total_duration_ms();

        Some(PreviewStats {
            original_frames: analysis.frame_count(),
            original_duration,
            result_frames,
            result_duration,
        })
    }
}

/// Preview statistics for displaying changes.
pub struct PreviewStats {
    pub original_frames: usize,
    pub original_duration: u64,
    pub result_frames: usize,
    pub result_duration: u64,
}

impl PreviewStats {
    pub fn saved_duration(&self) -> i64 {
        self.original_duration as i64 - self.result_duration as i64
    }

    pub fn saved_percent(&self) -> f64 {
        if self.original_duration > 0 {
            self.saved_duration() as f64 / self.original_duration as f64
        } else {
            0.0
        }
    }
}
