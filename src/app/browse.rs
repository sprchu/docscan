use std::fs;
use std::path::PathBuf;

use super::state::App;
use super::types::BrowseEntry;

impl App {
    // ── Enter / exit browse mode ──

    /// Enter browse mode to add a new directory
    pub fn enter_browse_mode(&mut self) {
        self.browse_target_index = None;
        self.browse_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self._enter_browse();
    }

    /// Enter browse mode to edit the directory at the given index
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
        self.mode = super::types::Mode::Browse;
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
        self.mode = super::types::Mode::Normal;
        self.browse_target_index = None;
    }

    // ── Navigation ──

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
            // Current directory row → confirm selection
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

    // ── Confirm / set path ──

    pub fn select_browse_dir(&mut self) {
        // Resolve path from the selected row, not browse_cwd directly
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

    // ── Convenience entry points ──

    pub fn add_new_path_via_browse(&mut self) {
        self.enter_browse_mode();
    }

    pub fn edit_path_via_browse(&mut self) {
        if self.dir_selected < self.dirs.len() && !self.dirs[self.dir_selected].trim().is_empty() {
            self.enter_browse_mode_for_index(self.dir_selected);
        }
    }

    // ── Directory listing ──

    /// List directory entries (directories only, plus ".." if has parent)
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
}
