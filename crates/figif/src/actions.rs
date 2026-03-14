//! User actions (messages in Elm terminology).

#![allow(dead_code)]

use figif_core::{Analysis, EncodeConfig};
use img_hash::ImageHash;
use std::path::PathBuf;

use crate::ui::operations_menu::OptimizeOption;

/// User actions that modify application state.
#[derive(Debug, Clone)]
pub enum Action {
    // Navigation (segment view)
    NextSegment,
    PrevSegment,
    FirstSegment,
    LastSegment,

    // Selection (segment view)
    ToggleSelected,
    SelectAll,
    DeselectAll,
    SelectStatic,
    SelectMotion,

    // Operations (toggle behavior - press again to clear)
    ToggleRemove,
    CapDuration(u32),
    CollapseDuration(u32),
    SpeedUp(f64),
    SlowDown(f64),
    SetDuration(u32),
    ClearOperation,
    Undo,
    ResetAll,

    // Preview frame navigation
    NextFrame,
    PrevFrame,
    FirstFrame,
    LastFrame,
    TogglePlayback,

    // Preview zoom
    ZoomPreviewIn,
    ZoomPreviewOut,
    ResetPreviewZoom,

    // Frame view navigation (zoom into a segment)
    EnterFrameView,
    ExitFrameView,
    NextFrameInList,
    PrevFrameInList,
    FirstFrameInList,
    LastFrameInList,

    // Frame selection (within frame view)
    ToggleFrameSelected,
    SelectAllFrames,
    DeselectAllFrames,
    SelectDuplicateFrames,

    // Frame operations
    ToggleRemoveFrame,
    ToggleSplitAfterFrame,
    ClearFrameOperation,

    // File
    LoadFile(PathBuf),
    LoadingProgress(usize, usize),
    AnalysisResult(Result<Analysis<ImageHash>, String>),
    Export(PathBuf, EncodeConfig),

    // Export dialog
    ShowExport,
    ConfirmExport,
    CancelExport,
    DiscardChanges,

    // Operations menu
    ShowOperationsMenu,
    ApplyOptimization(OptimizeOption),
    CloseOperationsMenu,

    // UI
    ShowHelp,
    HideOverlay,
    ShowInput(InputKind),
    SubmitInput(String),
    CancelInput,

    // Settings
    CycleHashAlgorithm,

    // System
    Tick,
    Quit,
    Error(String),
}

/// Types of input prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    CapDuration,
    CollapseDuration,
    SpeedFactor,
    DuplicateSelection,
}

impl InputKind {
    /// Dialog title explaining the operation.
    pub fn title(&self) -> &'static str {
        match self {
            InputKind::CapDuration => " Cap Duration (c) - Limit frame delays ",
            InputKind::CollapseDuration => " Collapse (Shift+C) - Reduce to single frame ",
            InputKind::SpeedFactor => " Speed Factor ",
            InputKind::DuplicateSelection => " Select Duplicates (Ctrl+D) ",
        }
    }

    /// Description of what this operation does.
    pub fn description(&self) -> &'static str {
        match self {
            InputKind::CapDuration => {
                "Set MAX delay per frame. Frames slower than this will be sped up."
            }
            InputKind::CollapseDuration => "Replace ALL frames with ONE frame at this duration.",
            InputKind::SpeedFactor => "Multiply all delays by this factor.",
            InputKind::DuplicateSelection => {
                "Select frames matching current, keeping some unselected."
            }
        }
    }

    /// Input field label.
    pub fn prompt(&self) -> &'static str {
        match self {
            InputKind::CapDuration => "Max frame delay (ms): ",
            InputKind::CollapseDuration => "Single frame duration (ms): ",
            InputKind::SpeedFactor => "Speed factor: ",
            InputKind::DuplicateSelection => "Keep (s:1, e:1, b:1): ",
        }
    }

    pub fn default_value(&self) -> &'static str {
        match self {
            InputKind::CapDuration => "300",
            InputKind::CollapseDuration => "200",
            InputKind::SpeedFactor => "1.5",
            InputKind::DuplicateSelection => "s:1",
        }
    }
}

/// Export dialog state.
#[derive(Debug, Clone)]
pub struct ExportState {
    /// Output file path
    pub path: String,
    /// Cursor position in path
    pub cursor: usize,
    /// Whether the file already exists
    pub file_exists: bool,
    /// Whether user has confirmed overwrite
    pub confirmed_overwrite: bool,
    /// Operation summary for display
    pub operation_summary: OperationSummary,
}

impl ExportState {
    pub fn new(default_path: String, summary: OperationSummary) -> Self {
        let cursor = default_path.len();
        let file_exists = std::path::Path::new(&default_path).exists();
        Self {
            path: default_path,
            cursor,
            file_exists,
            confirmed_overwrite: false,
            operation_summary: summary,
        }
    }

    pub fn check_file_exists(&mut self) {
        self.file_exists = std::path::Path::new(&self.path).exists();
        if !self.file_exists {
            self.confirmed_overwrite = false;
        }
    }
}

/// Summary of operations for export dialog.
#[derive(Debug, Clone, Default)]
pub struct OperationSummary {
    pub segments_removed: usize,
    pub segments_collapsed: usize,
    pub segments_scaled: usize,
    pub original_frames: usize,
    pub result_frames: usize,
    pub original_duration_ms: u64,
    pub result_duration_ms: u64,
}
