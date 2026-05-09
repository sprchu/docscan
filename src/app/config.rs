use super::state::App;

// ── Query text editing ──

impl App {
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

    // ── Config-left row navigation ──

    pub fn config_left_row_up(&mut self) {
        if self.config_left_row > 0 {
            self.config_left_row -= 1;
        }
    }

    pub fn config_left_row_down(&mut self) {
        // Now 3 rows: query, threads, types (since Text was added)
        if self.config_left_row < 2 {
            self.config_left_row += 1;
        }
    }

    // ── Threads ──

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

    // ── File type toggles ──

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

    // ── Directory selection ──

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
}
