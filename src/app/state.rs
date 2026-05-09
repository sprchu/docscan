use std::path::PathBuf;
use std::time::Instant;

use super::types::{BrowseEntry, FileType, Focus, Mode, ScanResult};

/// Central application state.
///
/// Each focus area (config, results, browse, command) has its logic
/// separated into dedicated modules. This struct holds all shared state.
pub struct App {
    // ── Config ──
    pub query: String,
    pub query_cursor: usize,
    pub dirs: Vec<String>,
    pub threads: usize,
    pub file_types: Vec<(FileType, bool)>,
    pub ft_cursor: usize,

    // ── Results ──
    pub results: Vec<ScanResult>,
    pub scanning: bool,
    pub total_files: usize,
    pub scanned_files: usize,

    // ── Interaction state ──
    pub focus: Focus,
    pub mode: Mode,
    pub config_left_row: usize,
    pub selected: usize,
    pub dir_selected: usize,
    pub command: String,
    pub command_cursor: usize,
    pub message: String,

    // ── Filter ──
    pub filter: Option<FileType>,
    pub filter_text: String,
    pub filter_text_cursor: usize,
    /// When Focus::Results, true = typing in filter input, false = navigating list
    pub filter_focused: bool,

    // ── Browse state ──
    pub browse_cwd: PathBuf,
    pub browse_entries: Vec<BrowseEntry>,
    pub browse_selected: usize,
    pub browse_scroll: usize,
    pub browse_target_index: Option<usize>,
    /// (entry_index, click_time) for double-click detection in browse popup
    pub last_browse_click: Option<(usize, Instant)>,

    pub should_quit: bool,
}

impl App {
    pub fn new(query: String, threads: usize, dirs: Vec<String>, pdf: bool) -> Self {
        let query_cursor = query.len();
        let dirs = if dirs.is_empty() {
            vec![".".to_string()]
        } else {
            dirs.into_iter().filter(|d| !d.trim().is_empty()).collect()
        };
        let browse_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let browse_entries = Self::list_dir_entries(&browse_cwd);
        App {
            query,
            query_cursor,
            dirs,
            threads,
            file_types: vec![
                (FileType::Word, true),
                (FileType::Excel, true),
                (FileType::Pdf, pdf),
                (FileType::Text, false), // off by default
            ],
            ft_cursor: 0,
            results: Vec::new(),
            scanning: false,
            total_files: 0,
            scanned_files: 0,
            focus: Focus::ConfigLeft,
            mode: Mode::Normal,
            config_left_row: 0,
            selected: 0,
            dir_selected: 0,
            command: String::new(),
            command_cursor: 0,
            message: String::from("Ready. Type a keyword, set directories, then click [Scan]."),
            filter: None,
            filter_text: String::new(),
            filter_text_cursor: 0,
            filter_focused: false,
            should_quit: false,
            browse_cwd,
            browse_entries,
            browse_selected: 0,
            browse_scroll: 0,
            browse_target_index: None,
            last_browse_click: None,
        }
    }

    // ── Active dirs ──

    pub fn active_dirs(&self) -> Vec<&str> {
        self.dirs
            .iter()
            .filter(|d| !d.trim().is_empty())
            .map(|d| d.as_str())
            .collect()
    }

    /// Delete the selected path entry
    pub fn delete_selected_path(&mut self) {
        let idx = self.dir_selected;
        if idx >= self.dirs.len() || self.dirs[idx].trim().is_empty() {
            return;
        }
        let removed = self.dirs.remove(idx);
        if self.dir_selected >= self.dirs.len() {
            self.dir_selected = self.dirs.len().saturating_sub(1);
        }
        self.message = format!("Removed: {}", removed);
    }

    /// Collect all enabled file extensions for scanning
    pub fn enabled_extensions(&self) -> Vec<String> {
        self.file_types
            .iter()
            .filter(|(_, enabled)| *enabled)
            .flat_map(|(ft, _)| ft.extensions().iter().map(|s| s.to_string()))
            .collect()
    }
}
