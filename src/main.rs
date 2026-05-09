mod app;
mod command;
mod scan;
mod ui;
mod utils;

use std::sync::mpsc;
use std::time::Duration;

use app::input::ScanMessage;
use app::{App, UiRects};
use clap::Parser;
use crossterm::event::{self, Event};

#[derive(Parser, Debug)]
#[command(version, about = "word/excel/pdf/text 文档搜索工具")]
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

    let result = run_event_loop(&mut terminal, &mut app);

    // Disable mouse capture on exit
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);

    ratatui::restore();
    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
}

/// Main event loop: render → drain messages → poll input → dispatch
fn run_event_loop<B: ratatui::backend::Backend>(
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
        // ── Render ──
        terminal.draw(|f| {
            ui_rects = ui::render(f, app);
        })?;

        // ── Drain scan progress messages ──
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

        // ── Poll for input events ──
        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            match event {
                Event::Key(key) => {
                    app::input::handle_key(app, key, &scan_tx);
                }
                Event::Mouse(mouse) => {
                    app::mouse::handle_mouse(app, mouse, &scan_tx, &ui_rects);
                }
                _ => {}
            }
        }
    }
}
