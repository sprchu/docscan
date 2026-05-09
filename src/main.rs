mod app;
mod command;
mod scan;
mod ui;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use app::{App, FileType, Focus, Mode, ScanResult, UiRects};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about = "word/excel/pdf 文档搜索工具")]
struct Args {
    #[arg(short, help = "待搜索字符串")]
    query: Option<String>,

    #[arg(short, default_value_t = num_cpus::get(), help = "并发线程数")]
    jobs: usize,

    #[arg(short, long, default_value_t = false, help = "是否搜索PDF文件")]
    pdf: bool,

    #[arg(help = "扫描目录列表")]
    dirs: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let mut terminal = ratatui::init();
    let mut app = App::new(
        args.query.unwrap_or_default(),
        args.jobs,
        args.dirs,
        args.pdf,
    );

    // Enable mouse capture
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);

    let result = run_app(&mut terminal, &mut app);

    // Disable mouse capture on exit
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);

    ratatui::restore();
    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()>
where
    <B as ratatui::backend::Backend>::Error: Send + Sync + 'static,
{
    let (scan_tx, scan_rx): (mpsc::Sender<ScanMessage>, mpsc::Receiver<ScanMessage>) =
        mpsc::channel();

    let mut ui_rects = UiRects::default();

    loop {
        terminal.draw(|f| {
            ui_rects = ui::render(f, app);
        })?;

        // Drain scan messages
        while let Ok(msg) = scan_rx.try_recv() {
            match msg {
                ScanMessage::Progress { scanned, total } => {
                    app.scanned_files = scanned;
                    app.total_files = total;
                }
                ScanMessage::Hit(result) => {
                    app.results.push(result);
                }
                ScanMessage::Done { dirs, total } => {
                    app.scanning = false;
                    app.total_files = total;
                    if app.results.is_empty() {
                        if total == 0 {
                            app.message = format!(
                                "No supported files found in [{}]. Check dirs and file types.",
                                dirs.join(", ")
                            );
                        } else {
                            app.message = format!(
                                "No matches for '{}' in {} files across [{}].",
                                app.query,
                                total,
                                dirs.join(", ")
                            );
                        }
                    } else {
                        app.message = format!(
                            "Done: {} hits in {} files across [{}].",
                            app.results.len(),
                            total,
                            dirs.join(", ")
                        );
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }

        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            match event {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    match app.mode {
                        Mode::Command => handle_command_input(app, key.code),
                        Mode::Browse => handle_browse_key(app, key.code),
                        Mode::Normal => handle_normal_key(app, key, &scan_tx, &ui_rects),
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(app, mouse, &scan_tx, &ui_rects);
                }
                _ => {}
            }
        }
    }
}

enum ScanMessage {
    Progress { scanned: usize, total: usize },
    Hit(ScanResult),
    Done { dirs: Vec<String>, total: usize },
}

// ==================== Key input ====================

fn handle_normal_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
    _rects: &UiRects,
) {
    // Global keys (not in filter input)
    match key.code {
        KeyCode::Char('q') => {
            if !(app.focus == Focus::ConfigLeft && app.config_left_row == 0) {
                app.should_quit = true;
                return;
            }
        }
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

    match app.focus {
        Focus::ConfigLeft => handle_config_left_key(app, key, scan_tx),
        Focus::ConfigRight => handle_config_right_key(app, key, scan_tx),
        Focus::Results => handle_results_key(app, key, scan_tx),
    }
}

fn handle_config_left_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
) {
    match key.code {
        KeyCode::Up => app.config_left_row_up(),
        KeyCode::Down => app.config_left_row_down(),
        KeyCode::Enter => start_scan(app, scan_tx),

        _ => match app.config_left_row {
            0 => match key.code {
                KeyCode::Left => app.move_query_cursor_left(),
                KeyCode::Right => app.move_query_cursor_right(),
                KeyCode::Backspace => app.delete_query_char(),
                KeyCode::Char(c) => app.insert_query_char(c),
                _ => {}
            },
            1 => match key.code {
                KeyCode::Left => app.threads_dec(),
                KeyCode::Right => app.threads_inc(),
                _ => {}
            },
            2 => match key.code {
                KeyCode::Left => app.ft_cursor_left(),
                KeyCode::Right => app.ft_cursor_right(),
                KeyCode::Char(' ') => app.toggle_file_type(app.ft_cursor),
                _ => {}
            },
            _ => {}
        },
    }
}

fn handle_config_right_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
) {
    match key.code {
        KeyCode::Up => app.dir_selected_up(),
        KeyCode::Down => app.dir_selected_down(),
        KeyCode::Enter => start_scan(app, scan_tx),
        KeyCode::Delete | KeyCode::Char('d') => app.delete_selected_path(),
        _ => {}
    }
}

fn handle_results_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    _scan_tx: &mpsc::Sender<ScanMessage>,
) {
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
            // Typing switches to filter focus
            KeyCode::Char(c) => {
                app.filter_focused = true;
                app.insert_filter_char(c);
            }
            _ => {}
        }
    }
}

fn handle_command_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.exit_command_mode(),
        KeyCode::Enter => command::execute(app),
        KeyCode::Backspace => app.delete_command_char(),
        KeyCode::Left => app.move_command_cursor_left(),
        KeyCode::Right => app.move_command_cursor_right(),
        KeyCode::Char(c) => app.insert_command_char(c),
        _ => {}
    }
}

fn handle_browse_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.exit_browse_mode(),
        KeyCode::Up | KeyCode::Char('k') => app.browse_up(),
        KeyCode::Down | KeyCode::Char('j') => app.browse_down(),
        KeyCode::Enter => app.browse_enter(),
        KeyCode::Backspace | KeyCode::Left => app.browse_parent(),
        KeyCode::Char(' ') => app.select_browse_dir(),
        KeyCode::Char('~') => {
            if let Some(home) = dirs_fallback::home_dir() {
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

// ==================== Mouse input ====================

fn handle_mouse(
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
    rects: &UiRects,
) {
    let (col, row) = (mouse.column, mouse.row);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // ── Browse popup (modal) ──
            if app.mode == Mode::Browse {
                if let Some(ref r) = rects.browse_confirm_btn {
                    if hit(r, col, row) {
                        app.select_browse_dir();
                        return;
                    }
                }
                if let Some(ref r) = rects.browse_cancel_btn {
                    if hit(r, col, row) {
                        app.exit_browse_mode();
                        return;
                    }
                }
                // Click on browse row — single=select, double=enter
                for (i, r) in rects.browse_rows.iter().enumerate() {
                    if hit(r, col, row) {
                        let idx = rects.browse_list_start + i;
                        // Double-click detection (same row, within 400ms)
                        let now = std::time::Instant::now();
                        let is_double =
                            app.last_browse_click
                                .map_or(false, |(prev_idx, prev_time)| {
                                    prev_idx == idx
                                        && now.duration_since(prev_time).as_millis() < 400
                                });
                        if is_double {
                            app.browse_selected = idx;
                            app.browse_enter();
                            app.last_browse_click = None;
                        } else {
                            app.browse_selected = idx;
                            app.last_browse_click = Some((idx, now));
                        }
                        return;
                    }
                }
                // Click on browse panel but not on rows = ignore
                if let Some(ref p) = rects.browse_panel {
                    if hit(p, col, row) {
                        return;
                    }
                }
                // Click outside browse popup = cancel
                app.exit_browse_mode();
                return;
            }

            // ── Quit button ──
            if let Some(ref r) = rects.quit_button {
                if hit(r, col, row) {
                    app.should_quit = true;
                    return;
                }
            }

            // ── Keyword input ──
            if let Some(ref r) = rects.keyword_input {
                if hit(r, col, row) {
                    app.focus = Focus::ConfigLeft;
                    app.config_left_row = 0;
                    return;
                }
            }

            // ── Scan button ──
            if let Some(ref r) = rects.scan_button {
                if hit(r, col, row) {
                    start_scan(app, scan_tx);
                    return;
                }
            }

            // ── Threads dec/inc ──
            if let Some(ref r) = rects.threads_dec_btn {
                if hit(r, col, row) {
                    app.threads_dec();
                    app.focus = Focus::ConfigLeft;
                    app.config_left_row = 1;
                    return;
                }
            }
            if let Some(ref r) = rects.threads_inc_btn {
                if hit(r, col, row) {
                    app.threads_inc();
                    app.focus = Focus::ConfigLeft;
                    app.config_left_row = 1;
                    return;
                }
            }

            // ── Type toggle buttons ──
            for (i, r) in rects.type_btns.iter().enumerate() {
                if hit(r, col, row) {
                    app.toggle_file_type(i);
                    app.focus = Focus::ConfigLeft;
                    app.config_left_row = 2;
                    app.ft_cursor = i;
                    return;
                }
            }

            // ── Path Add / Edit / Delete buttons ──
            if let Some(ref r) = rects.path_add_btn {
                if hit(r, col, row) {
                    app.add_new_path_via_browse();
                    return;
                }
            }
            if let Some(ref r) = rects.path_edit_btn {
                if hit(r, col, row) {
                    app.edit_path_via_browse();
                    return;
                }
            }
            if let Some(ref r) = rects.path_del_btn {
                if hit(r, col, row) {
                    app.delete_selected_path();
                    return;
                }
            }

            // ── Path rows ──
            for (i, r) in rects.path_rows.iter().enumerate() {
                if hit(r, col, row) {
                    app.focus = Focus::ConfigRight;
                    app.dir_selected = rects.path_list_start + i;
                    return;
                }
            }
            // Click in paths panel area (but not on a row) → focus
            if let Some(ref p) = rects.paths_panel {
                if hit(p, col, row) {
                    app.focus = Focus::ConfigRight;
                    return;
                }
            }

            // ── Filter input ──
            if let Some(ref r) = rects.filter_input {
                if hit(r, col, row) {
                    app.focus = Focus::Results;
                    app.filter_focused = true;
                    return;
                }
            }

            // ── Filter type buttons ──
            for (ft, r) in &rects.filter_type_btns {
                if hit(r, col, row) {
                    app.set_filter_type(Some(ft.clone()));
                    return;
                }
            }
            if let Some(ref r) = rects.filter_all_btn {
                if hit(r, col, row) {
                    app.set_filter_type(None);
                    return;
                }
            }
            if let Some(ref r) = rects.filter_clear_btn {
                if hit(r, col, row) {
                    app.filter_text.clear();
                    app.filter_text_cursor = 0;
                    app.filter_focused = false;
                    return;
                }
            }

            // ── Result rows ──
            for (i, r) in rects.result_rows.iter().enumerate() {
                if hit(r, col, row) {
                    app.focus = Focus::Results;
                    app.filter_focused = false;
                    app.selected = rects.result_list_start + i;
                    return;
                }
            }
            // Click in results panel area → focus
            if let Some(ref p) = rects.results_panel {
                if hit(p, col, row) {
                    app.focus = Focus::Results;
                    return;
                }
            }

            // Click elsewhere in params area → focus keyword
            app.focus = Focus::ConfigLeft;
            app.config_left_row = 0;
        }

        MouseEventKind::ScrollDown => {
            // Browse popup has priority when open (modal)
            if app.mode == Mode::Browse {
                if let Some(ref p) = rects.browse_panel {
                    if hit(p, col, row) {
                        app.browse_down();
                        return;
                    }
                }
            }
            if let Some(ref p) = rects.paths_panel {
                if hit(p, col, row) {
                    app.focus = Focus::ConfigRight;
                    app.dir_selected_down();
                    return;
                }
            }
            if let Some(ref p) = rects.results_panel {
                if hit(p, col, row) {
                    app.focus = Focus::Results;
                    app.selected_down();
                    return;
                }
            }
        }

        MouseEventKind::ScrollUp => {
            if app.mode == Mode::Browse {
                if let Some(ref p) = rects.browse_panel {
                    if hit(p, col, row) {
                        app.browse_up();
                        return;
                    }
                }
            }
            if let Some(ref p) = rects.paths_panel {
                if hit(p, col, row) {
                    app.focus = Focus::ConfigRight;
                    app.dir_selected_up();
                    return;
                }
            }
            if let Some(ref p) = rects.results_panel {
                if hit(p, col, row) {
                    app.focus = Focus::Results;
                    app.selected_up();
                    return;
                }
            }
        }

        _ => {}
    }
}

fn hit(rect: &ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

// ==================== File open ====================

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

// ==================== Scan logic ====================

fn start_scan(app: &mut App, tx: &mpsc::Sender<ScanMessage>) {
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
    let enabled_exts = enabled_exts;
    let keyword_clone = keyword.clone();

    std::thread::spawn(move || {
        let utf16_pattern = scan::to_utf16le_bytes(&keyword_clone);
        let utf8_pattern = keyword_clone.as_bytes();

        let files: Vec<_> = dirs
            .iter()
            .flat_map(|d| {
                WalkDir::new(d)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        let path = e.path();
                        if !path.is_file() {
                            return false;
                        }
                        path.extension().map_or(false, |ext| {
                            let ext = ext.to_str().unwrap_or("").to_lowercase();
                            enabled_exts.contains(&ext)
                        })
                    })
            })
            .collect();

        let total = files.len();
        let _ = tx.send(ScanMessage::Progress { scanned: 0, total });

        let pool = match ThreadPoolBuilder::new().num_threads(threads).build() {
            Ok(p) => p,
            Err(_e) => {
                let _ = tx.send(ScanMessage::Progress {
                    scanned: 0,
                    total: 0,
                });
                let _ = tx.send(ScanMessage::Done { dirs, total: 0 });
                return;
            }
        };

        let scanned = AtomicUsize::new(0);

        pool.install(|| {
            files.par_iter().for_each(|entry| {
                let path = entry.path();
                let path_str = path.to_string_lossy();
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let hit = match ext.as_str() {
                    "docx" => scan::search_docx(&path_str, &keyword_clone).unwrap_or(false),
                    "doc" | "wps" => {
                        scan::search_doc(&path_str, &utf16_pattern, utf8_pattern).unwrap_or(false)
                    }
                    "xlsx" | "xls" | "et" => {
                        scan::search_excel(&path_str, &keyword_clone).unwrap_or(false)
                    }
                    "pdf" => scan::search_pdf(&path_str, &keyword_clone).unwrap_or(false),
                    _ => false,
                };

                if hit {
                    let file_type = FileType::from_ext(ext.as_str()).unwrap_or(FileType::Word);
                    let _ = tx.send(ScanMessage::Hit(ScanResult {
                        path: path.to_path_buf(),
                        file_type,
                    }));
                }

                let n = scanned.fetch_add(1, Ordering::Relaxed) + 1;
                if n % 10 == 0 || n == total {
                    let _ = tx.send(ScanMessage::Progress { scanned: n, total });
                }
            });
        });

        let _ = tx.send(ScanMessage::Done { dirs, total });
    });
}

/// Fallback home directory for platforms without dirs crate
mod dirs_fallback {
    pub fn home_dir() -> Option<std::path::PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE")
                .ok()
                .map(std::path::PathBuf::from)
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME").ok().map(std::path::PathBuf::from)
        }
    }
}
