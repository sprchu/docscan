use super::state::App;

// ── Command mode (":" prompt) text editing ──

impl App {
    pub fn enter_command_mode(&mut self) {
        self.mode = super::types::Mode::Command;
        self.command.clear();
        self.command_cursor = 0;
    }

    pub fn exit_command_mode(&mut self) {
        self.mode = super::types::Mode::Normal;
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
                self.command.remove(self.command_cursor);
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
