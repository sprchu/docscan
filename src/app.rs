use std::path::PathBuf;

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
    Help,
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
    pub config_left_row: usize, // 0=Keyword, 1=Threads, 2=FileTypes
    pub selected: usize,        // selected index in filtered results
    pub dir_selected: usize,    // selected index in dirs list
    pub command: String,
    pub command_cursor: usize,
    pub message: String,

    // Filter
    pub filter: Option<FileType>,

    pub should_quit: bool,
    pub auto_scan: bool,
}

impl App {
    pub fn new(query: String, threads: usize, dirs: Vec<String>, pdf: bool) -> Self {
        let query_cursor = query.len();
        let auto_scan = !query.is_empty() && !dirs.is_empty();
        let dirs = if dirs.is_empty() {
            vec![".".to_string()]
        } else {
            dirs
        };
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
            message: String::from(
                "Tab: switch panel | ↑↓: navigate | ←→: adjust | Enter: scan | :help | q: quit",
            ),
            filter: None,
            should_quit: false,
            auto_scan,
        }
    }

    pub fn filtered_results(&self) -> Vec<&ScanResult> {
        if let Some(ref ft) = self.filter {
            self.results.iter().filter(|r| r.file_type == *ft).collect()
        } else {
            self.results.iter().collect()
        }
    }

    // ── Config left row navigation ──

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

    // ── Dir list navigation ──

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

    // ── Results navigation ──

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

    // ── Threads adjustment ──

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

    // ── File type interactive toggle ──

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

    // ── Command mode ──

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
}
