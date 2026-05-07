mod app;
mod command;
mod scan;
mod ui;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use app::{App, FileType, Focus, Mode, ScanResult};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
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

    let result = run_app(&mut terminal, &mut app);

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

    let mut first_frame = true;

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if first_frame {
            first_frame = false;
            if app.auto_scan {
                app.auto_scan = false;
                start_scan(app, &scan_tx);
            }
        }

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
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                match app.mode {
                    Mode::Command => handle_command_input(app, key.code),
                    Mode::Help => {
                        app.mode = Mode::Normal;
                    }
                    Mode::Browse => handle_browse_input(app, key.code),
                    Mode::Normal => handle_normal_input(app, key, &scan_tx),
                }
            }
        }
    }
}

enum ScanMessage {
    Progress { scanned: usize, total: usize },
    Hit(ScanResult),
    Done { dirs: Vec<String>, total: usize },
}

// ==================== 输入处理 ====================
//
// Tab         = 循环焦点: ConfigLeft → ConfigRight → Results → ConfigLeft
// ↑/↓         = ConfigLeft: 选择参数行 / ConfigRight: 浏览路径 / Results: 浏览结果
// ←/→         = ConfigLeft: 调整参数 / ConfigRight: —
// Space       = ConfigLeft+Types: 切换文件类型
// e           = ConfigRight: 路径选择器编辑
// d           = ConfigRight: 删除所选路径
// a           = ConfigRight: 路径选择器新增
// Enter       = ConfigLeft/ConfigRight: 开始扫描 / Results: 打开文件
// :           = 命令模式
// q           = 退出

fn handle_normal_input(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
) {
    // Global keys
    match key.code {
        KeyCode::Char('q') => {
            // Don't quit when typing 'q' in the keyword field
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
            // Clamp selections when switching to panels
            match app.focus {
                Focus::ConfigRight => {
                    if app.dir_selected >= app.dirs.len().saturating_sub(1) {
                        app.dir_selected = app.dirs.len().saturating_sub(1);
                    }
                }
                Focus::Results => {
                    let max = app.filtered_results().len().saturating_sub(1);
                    if app.selected > max {
                        app.selected = max;
                    }
                }
                _ => {}
            }
            return;
        }
        _ => {}
    }

    match app.focus {
        Focus::ConfigLeft => handle_config_left(app, key, scan_tx),
        Focus::ConfigRight => handle_config_right(app, key, scan_tx),
        Focus::Results => handle_results(app, key, scan_tx),
    }
}

fn handle_config_left(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
) {
    match key.code {
        KeyCode::Up => app.config_left_row_up(),
        KeyCode::Down => app.config_left_row_down(),

        KeyCode::Enter => start_scan(app, scan_tx),

        _ => match app.config_left_row {
            // ── Keyword row ──
            0 => match key.code {
                KeyCode::Left => app.move_query_cursor_left(),
                KeyCode::Right => app.move_query_cursor_right(),
                KeyCode::Backspace => app.delete_query_char(),
                KeyCode::Char(c) => app.insert_query_char(c),
                _ => {}
            },

            // ── Threads row ──
            1 => match key.code {
                KeyCode::Left => app.threads_dec(),
                KeyCode::Right => app.threads_inc(),
                _ => {}
            },

            // ── File types row ──
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

fn handle_config_right(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
) {
    match key.code {
        KeyCode::Up => app.dir_selected_up(),
        KeyCode::Down => app.dir_selected_down(),
        KeyCode::Char('e') => {
            let idx = app.dir_selected;
            app.enter_browse_mode_for_index(idx);
        }
        KeyCode::Char('d') => app.delete_selected_path(),
        KeyCode::Char('a') => {
            app.enter_browse_mode();
        }
        KeyCode::Enter => start_scan(app, scan_tx),
        _ => {}
    }
}

fn handle_results(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    _scan_tx: &mpsc::Sender<ScanMessage>,
) {
    match key.code {
        KeyCode::Up => app.selected_up(),
        KeyCode::Down => app.selected_down(),
        KeyCode::Enter => open_selected_file(app),
        _ => {}
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

// ==================== 扫描逻辑 ====================

fn start_scan(app: &mut App, tx: &mpsc::Sender<ScanMessage>) {
    if app.query.is_empty() {
        app.message = "No query set. Type keyword or use :query <text>.".to_string();
        return;
    }

    let enabled_exts = app.enabled_extensions();
    if enabled_exts.is_empty() {
        app.message = "Enable at least one file type (←→ Space in Types).".to_string();
        return;
    }

    let dirs: Vec<String> = app
        .dirs
        .iter()
        .filter(|d| !d.trim().is_empty())
        .map(|d| d.trim().to_string())
        .collect();
    if dirs.is_empty() {
        app.message = "No directories set. Use :dir add <path>.".to_string();
        return;
    }

    let threads = app.threads;
    let keyword = app.query.clone();

    app.results.clear();
    app.scanning = true;
    app.scanned_files = 0;
    app.total_files = 0;
    app.selected = 0;
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

// ==================== Browse input ====================

fn handle_browse_input(app: &mut App, key: KeyCode) {
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
            // Go to root (or C:\ on Windows)
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
