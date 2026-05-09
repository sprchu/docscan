//! DocScan — a blazingly fast TUI document search tool.
//!
//! Search keywords across Word, Excel, PDF, and plain text files
//! in parallel, with an interactive terminal interface.
//!
//! This crate entry point parses CLI args, initialises the terminal,
//! and runs the main event loop.

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
#[command(version, about = "Document search — Word, Excel, PDF, plain text")]
struct Args {
    #[arg(short, help = "keywords to search for")]
    query: Option<String>,

    #[arg(short, default_value_t = num_cpus::get(), help = "number of concurrent threads")]
    jobs: usize,

    #[arg(short, long, default_value_t = false, help = "search PDF files")]
    pdf: bool,

    #[arg(help = "directories to scan")]
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
