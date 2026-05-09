use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use ratatui::layout::Rect;

#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    Word,
    Excel,
    Pdf,
}

impl FileType {
    pub fn extensions(&self) -> &[&str] {
        match self {
            FileType::Word => &["docx", "doc", "wps"],
            FileType::Excel => &["xlsx", "xls", "et"],
            FileType::Pdf => &["pdf"],
        }
    }

    pub fn short_label(&self) -> &str {
        match self {
            FileType::Word => "Word",
            FileType::Excel => "Excel",
            FileType::Pdf => "PDF",
        }
    }

    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "docx" | "doc" | "wps" => Some(FileType::Word),
            "xlsx" | "xls" | "et" => Some(FileType::Excel),
            "pdf" => Some(FileType::Pdf),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScanResult {
    pub path: PathBuf,
    pub file_type: FileType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Focus {
    ConfigLeft,
    ConfigRight,
    Results,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Focus::ConfigLeft => Focus::ConfigRight,
            Focus::ConfigRight => Focus::Results,
            Focus::Results => Focus::ConfigLeft,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Command,
    Browse,
}

// ── Clickable regions populated during render ──

#[derive(Default, Clone)]
pub struct UiRects {
    // Params panel
    pub keyword_input: Option<Rect>,
    pub scan_button: Option<Rect>,
    pub threads_dec_btn: Option<Rect>,
    pub threads_inc_btn: Option<Rect>,
    pub type_btns: Vec<Rect>,
    // Paths panel
    pub paths_panel: Option<Rect>,
    pub path_add_btn: Option<Rect>,
    pub path_edit_btn: Option<Rect>,
    pub path_del_btn: Option<Rect>,
    pub path_rows: Vec<Rect>,
    pub path_list_start: usize,
    // Results panel
    pub results_panel: Option<Rect>,
    pub result_rows: Vec<Rect>,
    pub result_list_start: usize,
    pub filter_input: Option<Rect>,
    pub filter_type_btns: Vec<(FileType, Rect)>,
    pub filter_all_btn: Option<Rect>,
    pub filter_clear_btn: Option<Rect>,
    pub quit_button: Option<Rect>,
    // Browse popup
    pub browse_panel: Option<Rect>,
    pub browse_confirm_btn: Option<Rect>,
    pub browse_cancel_btn: Option<Rect>,
    pub browse_rows: Vec<Rect>,
    pub browse_list_start: usize,
}

pub struct App {
    // Config
    pub query: String,
    pub query_cursor: usize,
    pub dirs: Vec<String>,
    pub threads: usize,
    pub file_types: Vec<(FileType, bool)>,
    pub ft_cursor: usize,

    // Results
    pub results: Vec<ScanResult>,
    pub scanning: bool,
    pub total_files: usize,
    pub scanned_files: usize,

    // Interaction state
    pub focus: Focus,
    pub mode: Mode,
    pub config_left_row: usize,
    pub selected: usize,
    pub dir_selected: usize,
    pub command: String,
    pub command_cursor: usize,
    pub message: String,

    // Filter
    pub filter: Option<FileType>,
    pub filter_text: String,
    pub filter_text_cursor: usize,
    /// When Focus::Results, true = typing in filter, false = navigating list
    pub filter_focused: bool,

    // Browse state
    pub browse_cwd: PathBuf,
    pub browse_entries: Vec<BrowseEntry>,
    pub browse_selected: usize,
    pub browse_scroll: usize,
    pub browse_target_index: Option<usize>,
    /// (entry_index, click_time) for double-click detection in browse popup
    pub last_browse_click: Option<(usize, Instant)>,

    pub should_quit: bool,
}

#[derive(Clone, Debug)]
pub struct BrowseEntry {
    pub name: String,
    pub is_dir: bool,
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

    /// Delete the selected path
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

    // ── Browse mode ──

    pub fn enter_browse_mode(&mut self) {
        self.browse_target_index = None;
        self.browse_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self._enter_browse();
    }

    pub fn enter_browse_mode_for_index(&mut self, idx: usize) {
        self.browse_target_index = Some(idx);
        let seed = PathBuf::from(&self.dirs[idx]);
        if seed.is_dir() {
            self.browse_cwd = seed;
        } else {
            self.browse_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
        self._enter_browse();
    }

    fn _enter_browse(&mut self) {
        self.mode = Mode::Browse;
        self.command.clear();
        self.command_cursor = 0;
        // Canonicalize to resolve "." and symlinks so parent navigation works
        if let Ok(canon) = self.browse_cwd.canonicalize() {
            self.browse_cwd = canon;
        }
        self.browse_entries = Self::list_dir_entries(&self.browse_cwd);
        self.browse_selected = 0;
        self.browse_scroll = 0;
        self.last_browse_click = None;
    }

    pub fn exit_browse_mode(&mut self) {
        self.mode = Mode::Normal;
        self.browse_target_index = None;
    }

    pub fn browse_up(&mut self) {
        let total = self.browse_entries.len() + 1; // +1 for current-dir row
        if total == 0 {
            return;
        }
        if self.browse_selected > 0 {
            self.browse_selected -= 1;
        } else {
            self.browse_selected = total - 1;
        }
        self.browse_scroll = self.browse_selected;
    }

    pub fn browse_down(&mut self) {
        let total = self.browse_entries.len() + 1;
        if total == 0 {
            return;
        }
        if self.browse_selected + 1 < total {
            self.browse_selected += 1;
        } else {
            self.browse_selected = 0;
        }
        self.browse_scroll = self.browse_selected;
    }

    pub fn browse_enter(&mut self) {
        if self.browse_selected == 0 {
            // Current directory row
            self.select_browse_dir();
            return;
        }
        let entry_idx = self.browse_selected - 1;
        if let Some(entry) = self.browse_entries.get(entry_idx) {
            if entry.name == ".." {
                self.browse_parent();
            } else if entry.is_dir {
                let new_path = self.browse_cwd.join(&entry.name);
                self.browse_cwd = new_path;
                self._enter_browse();
            }
        }
    }

    pub fn browse_parent(&mut self) {
        if let Some(parent) = self.browse_cwd.parent() {
            self.browse_cwd = parent.to_path_buf();
            self.browse_entries = Self::list_dir_entries(&self.browse_cwd);
            self.browse_selected = 0;
        }
    }

    pub fn select_browse_dir(&mut self) {
        // Resolve path from selected row, not browse_cwd
        let path = if self.browse_selected == 0 {
            self.browse_cwd.to_string_lossy().to_string()
        } else if let Some(entry) = self.browse_entries.get(self.browse_selected - 1) {
            self.browse_cwd
                .join(&entry.name)
                .to_string_lossy()
                .to_string()
        } else {
            self.browse_cwd.to_string_lossy().to_string()
        };

        if let Some(idx) = self.browse_target_index {
            if idx < self.dirs.len() {
                self.dirs[idx] = path.clone();
            }
            self.message = format!("Path updated: {}", path);
        } else {
            if !self.dirs.iter().any(|d| d == &path) {
                self.dirs.push(path.clone());
                self.message = format!("Added directory: {}", path);
            } else {
                self.message = format!("Directory already in list: {}", path);
            }
        }

        self.exit_browse_mode();
    }

    pub fn add_new_path_via_browse(&mut self) {
        self.enter_browse_mode();
    }

    pub fn edit_path_via_browse(&mut self) {
        if self.dir_selected < self.dirs.len() && !self.dirs[self.dir_selected].trim().is_empty() {
            self.enter_browse_mode_for_index(self.dir_selected);
        }
    }

    pub fn list_dir_entries(cwd: &PathBuf) -> Vec<BrowseEntry> {
        let mut entries: Vec<BrowseEntry> = Vec::new();
        if cwd.parent().is_some() {
            entries.push(BrowseEntry {
                name: "..".to_string(),
                is_dir: true,
            });
        }
        if let Ok(read_dir) = fs::read_dir(cwd) {
            let mut dirs: Vec<BrowseEntry> = Vec::new();
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                if is_dir {
                    dirs.push(BrowseEntry { name, is_dir });
                }
            }
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            entries.extend(dirs);
        }
        entries
    }

    // ── Query editing ──

    pub fn move_query_cursor_left(&mut self) {
        if self.query_cursor > 0 {
            let before = &self.query[..self.query_cursor];
            if let Some(c) = before.chars().last() {
                self.query_cursor -= c.len_utf8();
            }
        }
    }

    pub fn move_query_cursor_right(&mut self) {
        if self.query_cursor < self.query.len() {
            let after = &self.query[self.query_cursor..];
            if let Some(c) = after.chars().next() {
                self.query_cursor += c.len_utf8();
            }
        }
    }

    pub fn insert_query_char(&mut self, c: char) {
        let pos = self.query_cursor;
        self.query.insert(pos, c);
        self.query_cursor += c.len_utf8();
    }

    pub fn delete_query_char(&mut self) {
        if self.query_cursor > 0 {
            let before = &self.query[..self.query_cursor];
            if let Some(c) = before.chars().last() {
                self.query_cursor -= c.len_utf8();
                self.query.remove(self.query_cursor);
            }
        }
    }

    // ── Filter text editing ──

    pub fn insert_filter_char(&mut self, c: char) {
        let pos = self.filter_text_cursor;
        self.filter_text.insert(pos, c);
        self.filter_text_cursor += c.len_utf8();
    }

    pub fn delete_filter_char(&mut self) {
        if self.filter_text_cursor > 0 {
            let before = &self.filter_text[..self.filter_text_cursor];
            if let Some(c) = before.chars().last() {
                self.filter_text_cursor -= c.len_utf8();
                self.filter_text.remove(self.filter_text_cursor);
            }
        }
    }

    pub fn move_filter_cursor_left(&mut self) {
        if self.filter_text_cursor > 0 {
            let before = &self.filter_text[..self.filter_text_cursor];
            if let Some(c) = before.chars().last() {
                self.filter_text_cursor -= c.len_utf8();
            }
        }
    }

    pub fn move_filter_cursor_right(&mut self) {
        if self.filter_text_cursor < self.filter_text.len() {
            let after = &self.filter_text[self.filter_text_cursor..];
            if let Some(c) = after.chars().next() {
                self.filter_text_cursor += c.len_utf8();
            }
        }
    }

    // ── Other helpers ──

    pub fn filtered_results(&self) -> Vec<&ScanResult> {
        let mut out: Vec<&ScanResult> = self.results.iter().collect();
        if let Some(ref ft) = self.filter {
            out.retain(|r| r.file_type == *ft);
        }
        if !self.filter_text.is_empty() {
            let q = self.filter_text.to_lowercase();
            out.retain(|r| r.path.to_string_lossy().to_lowercase().contains(&q));
        }
        out
    }

    pub fn config_left_row_up(&mut self) {
        if self.config_left_row > 0 {
            self.config_left_row -= 1;
        }
    }

    pub fn config_left_row_down(&mut self) {
        if self.config_left_row < 2 {
            self.config_left_row += 1;
        }
    }

    pub fn dir_selected_up(&mut self) {
        if self.dir_selected > 0 {
            self.dir_selected -= 1;
        }
    }

    pub fn dir_selected_down(&mut self) {
        if self.dir_selected + 1 < self.dirs.len() {
            self.dir_selected += 1;
        }
    }

    pub fn selected_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn selected_down(&mut self) {
        let max = self.filtered_results().len().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
        }
    }

    pub fn threads_inc(&mut self) {
        if self.threads < 64 {
            self.threads += 1;
        }
    }

    pub fn threads_dec(&mut self) {
        if self.threads > 1 {
            self.threads -= 1;
        }
    }

    pub fn toggle_file_type(&mut self, idx: usize) {
        if idx < self.file_types.len() {
            self.file_types[idx].1 = !self.file_types[idx].1;
        }
    }

    pub fn ft_cursor_left(&mut self) {
        if self.ft_cursor > 0 {
            self.ft_cursor -= 1;
        }
    }

    pub fn ft_cursor_right(&mut self) {
        if self.ft_cursor + 1 < self.file_types.len() {
            self.ft_cursor += 1;
        }
    }

    pub fn enabled_extensions(&self) -> Vec<String> {
        self.file_types
            .iter()
            .filter(|(_, enabled)| *enabled)
            .flat_map(|(ft, _)| ft.extensions().iter().map(|s| s.to_string()))
            .collect()
    }

    pub fn enter_command_mode(&mut self) {
        self.mode = Mode::Command;
        self.command.clear();
        self.command_cursor = 0;
    }

    pub fn exit_command_mode(&mut self) {
        self.mode = Mode::Normal;
        self.command.clear();
        self.command_cursor = 0;
    }

    pub fn insert_command_char(&mut self, c: char) {
        let pos = self.command_cursor;
        self.command.insert(pos, c);
        self.command_cursor += c.len_utf8();
    }

    pub fn delete_command_char(&mut self) {
        if self.command_cursor > 0 {
            let before = &self.command[..self.command_cursor];
            if let Some(c) = before.chars().last() {
                self.command_cursor -= c.len_utf8();
                let new_pos = self.command_cursor;
                self.command.remove(new_pos);
            }
        }
    }

    pub fn move_command_cursor_left(&mut self) {
        if self.command_cursor > 0 {
            let before = &self.command[..self.command_cursor];
            if let Some(c) = before.chars().last() {
                self.command_cursor -= c.len_utf8();
            }
        }
    }

    pub fn move_command_cursor_right(&mut self) {
        if self.command_cursor < self.command.len() {
            let after = &self.command[self.command_cursor..];
            if let Some(c) = after.chars().next() {
                self.command_cursor += c.len_utf8();
            }
        }
    }

    pub fn set_filter_type(&mut self, ft: Option<FileType>) {
        self.filter = ft;
        self.selected = 0;
    }
}
