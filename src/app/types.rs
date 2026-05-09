use std::path::PathBuf;

use ratatui::layout::Rect;

/// Supported document file types
#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    Word,
    Excel,
    Pdf,
    /// Plain UTF-8 text files (extensible: add new variants here)
    Text,
}

impl FileType {
    /// File extensions associated with this type.
    /// When adding a new variant, add its extensions here.
    pub fn extensions(&self) -> &[&str] {
        match self {
            FileType::Word => &["docx", "doc", "wps"],
            FileType::Excel => &["xlsx", "xls", "et"],
            FileType::Pdf => &["pdf"],
            FileType::Text => &[
                "txt", "md", "rs", "py", "js", "ts", "java", "c", "cpp", "h", "hpp", "go", "rb",
                "php", "html", "css", "json", "xml", "yaml", "yml", "toml", "csv", "log", "sh",
                "bat", "ps1", "conf", "cfg", "ini",
            ],
        }
    }

    /// Human-readable short label for UI
    pub fn short_label(&self) -> &str {
        match self {
            FileType::Word => "Word",
            FileType::Excel => "Excel",
            FileType::Pdf => "PDF",
            FileType::Text => "Text",
        }
    }

    /// Determine FileType from a file extension string
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "docx" | "doc" | "wps" => Some(FileType::Word),
            "xlsx" | "xls" | "et" => Some(FileType::Excel),
            "pdf" => Some(FileType::Pdf),
            "txt" | "md" | "rs" | "py" | "js" | "ts" | "java" | "c" | "cpp" | "h" | "hpp"
            | "go" | "rb" | "php" | "html" | "css" | "json" | "xml" | "yaml" | "yml" | "toml"
            | "csv" | "log" | "sh" | "bat" | "ps1" | "conf" | "cfg" | "ini" => Some(FileType::Text),
            _ => None,
        }
    }
}

/// A single search result
#[derive(Clone, Debug)]
pub struct ScanResult {
    pub path: PathBuf,
    pub file_type: FileType,
}

/// Which panel has keyboard focus
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Focus {
    ConfigLeft,
    ConfigRight,
    Results,
}

impl Focus {
    /// Cycle focus to the next panel
    pub fn next(self) -> Self {
        match self {
            Focus::ConfigLeft => Focus::ConfigRight,
            Focus::ConfigRight => Focus::Results,
            Focus::Results => Focus::ConfigLeft,
        }
    }
}

/// Global application mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Command,
    Browse,
}

/// A directory entry shown in the browse popup
#[derive(Clone, Debug)]
pub struct BrowseEntry {
    pub name: String,
    pub is_dir: bool,
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
