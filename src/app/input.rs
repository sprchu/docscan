use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::state::App;
use super::types::{Focus, Mode};

/// Message sent from the scan worker thread to the main event loop
pub enum ScanMessage {
    Progress { scanned: usize, total: usize },
    Hit(super::types::ScanResult),
    Done { dirs: Vec<String>, total: usize },
}

// ═══════════════════════════════════════════════════════════════
//  Top-level key dispatch (called from main event loop)
// ═══════════════════════════════════════════════════════════════

/// Handle a key event, dispatching based on current mode & focus.
/// Returns `true` if the application should redraw.
pub fn handle_key(app: &mut App, key: KeyEvent, scan_tx: &mpsc::Sender<ScanMessage>) {
    if key.kind == KeyEventKind::Release {
        return;
    }

    match app.mode {
        Mode::Command => handle_command_key(app, key.code),
        Mode::Browse => handle_browse_key(app, key.code),
        Mode::Normal => handle_normal_key(app, key, scan_tx),
    }
}

// ═══════════════════════════════════════════════════════════════
//  Normal mode
// ═══════════════════════════════════════════════════════════════

fn handle_normal_key(app: &mut App, key: KeyEvent, scan_tx: &mpsc::Sender<ScanMessage>) {
    // ── Global keys (ignored when filter input is focused) ──
    if !app.filter_focused {
        match key.code {
            KeyCode::Char(':') => {
                app.enter_command_mode();
                return;
            }
            KeyCode::Tab => {
                app.focus = app.focus.next();
                match app.focus {
                    Focus::ConfigRight => {
                        if app.dir_selected >= app.dirs.len().saturating_sub(1) {
                            app.dir_selected = app.dirs.len().saturating_sub(1);
                        }
                    }
                    Focus::Results => {
                        app.filter_focused = false;
                        let max = app.filtered_results().len().saturating_sub(1);
                        if app.selected > max {
                            app.selected = max;
                        }
                    }
                    _ => {}
                }
                return;
            }
            KeyCode::Esc => {
                app.filter_text.clear();
                app.filter_text_cursor = 0;
                app.filter_focused = false;
                return;
            }
            _ => {}
        }
    }

    // ── Focus-specific keys ──
    match app.focus {
        Focus::ConfigLeft => handle_config_left_key(app, key, scan_tx),
        Focus::ConfigRight => handle_config_right_key(app, key, scan_tx),
        Focus::Results => handle_results_key(app, key),
    }
}

// ── ConfigLeft panel ──

fn handle_config_left_key(app: &mut App, key: KeyEvent, scan_tx: &mpsc::Sender<ScanMessage>) {
    match key.code {
        KeyCode::Up => app.config_left_row_up(),
        KeyCode::Down => app.config_left_row_down(),
        KeyCode::Enter => start_scan(app, scan_tx),

        _ => match app.config_left_row {
            0 /* query */ => match key.code {
                KeyCode::Left => app.move_query_cursor_left(),
                KeyCode::Right => app.move_query_cursor_right(),
                KeyCode::Backspace => app.delete_query_char(),
                KeyCode::Char(c) => app.insert_query_char(c),
                _ => {}
            },
            1 /* threads */ => match key.code {
                KeyCode::Left => app.threads_dec(),
                KeyCode::Right => app.threads_inc(),
                _ => {}
            },
            2 /* file types */ => match key.code {
                KeyCode::Left => app.ft_cursor_left(),
                KeyCode::Right => app.ft_cursor_right(),
                KeyCode::Char(' ') => app.toggle_file_type(app.ft_cursor),
                _ => {}
            },
            _ => {}
        },
    }
}

// ── ConfigRight panel (directory list) ──

fn handle_config_right_key(app: &mut App, key: KeyEvent, scan_tx: &mpsc::Sender<ScanMessage>) {
    match key.code {
        KeyCode::Up => app.dir_selected_up(),
        KeyCode::Down => app.dir_selected_down(),
        KeyCode::Enter => start_scan(app, scan_tx),
        KeyCode::Delete | KeyCode::Char('d') => app.delete_selected_path(),
        _ => {}
    }
}

// ── Results panel ──

fn handle_results_key(app: &mut App, key: KeyEvent) {
    if app.filter_focused {
        // Typing in filter input
        match key.code {
            KeyCode::Esc => {
                app.filter_text.clear();
                app.filter_text_cursor = 0;
                app.filter_focused = false;
            }
            KeyCode::Up | KeyCode::Down => app.filter_focused = false,
            KeyCode::Left => app.move_filter_cursor_left(),
            KeyCode::Right => app.move_filter_cursor_right(),
            KeyCode::Backspace => app.delete_filter_char(),
            KeyCode::Char(c) => app.insert_filter_char(c),
            _ => {}
        }
    } else {
        // Navigating results list
        match key.code {
            KeyCode::Up => app.selected_up(),
            KeyCode::Down => app.selected_down(),
            KeyCode::Enter => open_selected_file(app),
            // Typing any char switches to filter focus
            KeyCode::Char(c) => {
                app.filter_focused = true;
                app.insert_filter_char(c);
            }
            _ => {}
        }
    }
}

// ── Command mode ──

fn handle_command_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.exit_command_mode(),
        KeyCode::Enter => super::super::command::execute(app),
        KeyCode::Backspace => app.delete_command_char(),
        KeyCode::Left => app.move_command_cursor_left(),
        KeyCode::Right => app.move_command_cursor_right(),
        KeyCode::Char(c) => app.insert_command_char(c),
        _ => {}
    }
}

// ── Browse mode ──

fn handle_browse_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.exit_browse_mode(),
        KeyCode::Up | KeyCode::Char('k') => app.browse_up(),
        KeyCode::Down | KeyCode::Char('j') => app.browse_down(),
        KeyCode::Enter => app.browse_enter(),
        KeyCode::Backspace | KeyCode::Left => app.browse_parent(),
        KeyCode::Char(' ') => app.select_browse_dir(),
        KeyCode::Char('~') => {
            if let Some(home) = super::super::utils::home_dir() {
                app.browse_cwd = home;
                app.browse_entries = App::list_dir_entries(&app.browse_cwd);
                app.browse_selected = 0;
            }
        }
        KeyCode::Char('/') => {
            #[cfg(target_os = "windows")]
            {
                app.browse_cwd = std::path::PathBuf::from("C:\\");
            }
            #[cfg(not(target_os = "windows"))]
            {
                app.browse_cwd = std::path::PathBuf::from("/");
            }
            app.browse_entries = App::list_dir_entries(&app.browse_cwd);
            app.browse_selected = 0;
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════════════

/// Open the currently selected file with the OS default application
fn open_selected_file(app: &mut App) {
    let filtered = app.filtered_results();
    if let Some(result) = filtered.get(app.selected) {
        let path = result.path.to_string_lossy().to_string();
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "", &path])
                .spawn();
        }
        app.message = format!("Opened: {}", path);
    }
}

/// Kick off a scan in a background thread
pub(crate) fn start_scan(app: &mut App, tx: &mpsc::Sender<ScanMessage>) {
    if app.query.is_empty() {
        app.message = "No query set. Type a keyword first.".to_string();
        return;
    }

    let enabled_exts = app.enabled_extensions();
    if enabled_exts.is_empty() {
        app.message = "Enable at least one file type.".to_string();
        return;
    }

    let dirs: Vec<String> = app
        .dirs
        .iter()
        .filter(|d| !d.trim().is_empty())
        .map(|d| d.trim().to_string())
        .collect();
    if dirs.is_empty() {
        app.message = "No directories set. Click + Add in the Paths panel.".to_string();
        return;
    }

    let threads = app.threads;
    let keyword = app.query.clone();

    app.results.clear();
    app.scanning = true;
    app.scanned_files = 0;
    app.total_files = 0;
    app.selected = 0;
    app.filter_text.clear();
    app.filter_text_cursor = 0;
    app.filter_focused = false;
    app.filter = None;
    app.message = format!(
        "Scanning '{}' in [{}] with {} threads...",
        keyword,
        dirs.join(", "),
        threads
    );

    let tx = tx.clone();
    let keyword_clone = keyword.clone();

    // Delegate actual scanning logic to the scan module
    super::super::scan::run_scan(tx, dirs, keyword_clone, enabled_exts, threads);
}
