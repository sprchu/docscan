use super::state::App;
use super::types::FileType;

impl App {
    // ── Result selection ──

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

    // ── Filter type ──

    pub fn set_filter_type(&mut self, ft: Option<FileType>) {
        self.filter = ft;
        self.selected = 0;
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

    // ── Filtered results view ──

    /// Returns results after applying type filter and text search
    pub fn filtered_results(&self) -> Vec<&super::types::ScanResult> {
        let mut out: Vec<&super::types::ScanResult> = self.results.iter().collect();
        if let Some(ref ft) = self.filter {
            out.retain(|r| r.file_type == *ft);
        }
        if !self.filter_text.is_empty() {
            let q = self.filter_text.to_lowercase();
            out.retain(|r| r.path.to_string_lossy().to_lowercase().contains(&q));
        }
        out
    }
}
