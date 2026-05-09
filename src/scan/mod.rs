//! Pluggable scanner architecture.
//!
//! ## Adding a new scanner
//!
//! 1. Create a module implementing the [`Scanner`] trait
//! 2. Register it in [`all_scanners()`]
//! 3. Add a `FileType` variant and its extensions in [`crate::app::types`]
//!
//! See [`text`] for a minimal example.

pub mod doc;
pub mod docx;
pub mod excel;
pub mod pdf;
pub mod text;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::app::input::ScanMessage;
use crate::app::{FileType, ScanResult};

// ═══════════════════════════════════════════════════════════════
//  Scanner trait — implement this to add a new file scanner
// ═══════════════════════════════════════════════════════════════

/// A scanner knows which file type it handles and how to search it.
///
/// To add a new scanner (e.g. for `.epub`):
/// 1. Create a new module in `scan/`
/// 2. Implement this trait
/// 3. Register it in the `scanner_for_ext` dispatch below
pub trait Scanner: Send + Sync {
    /// The FileType this scanner handles
    fn file_type(&self) -> FileType;

    /// Return true if `keyword` is found in the file at `path`
    fn search(&self, path: &Path, keyword: &str) -> bool;
}

// ═══════════════════════════════════════════════════════════════
//  Scanner registry — maps file extensions to scanners
// ═══════════════════════════════════════════════════════════════

/// Create all scanner instances.
/// Add new scanners here when implementing new `Scanner` types.
fn all_scanners() -> Vec<Box<dyn Scanner>> {
    vec![
        Box::new(docx::DocxScanner),
        Box::new(doc::DocScanner),
        Box::new(excel::ExcelScanner),
        Box::new(pdf::PdfScanner),
        Box::new(text::TextScanner),
    ]
}

/// Pick the right scanner for a given file extension.
/// Returns `None` if no scanner handles this extension.
pub fn scanner_for_ext(ext: &str) -> Option<Box<dyn Scanner>> {
    let ext_lower = ext.to_lowercase();
    all_scanners().into_iter().find(|s| {
        s.file_type()
            .extensions()
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&ext_lower))
    })
}

// ═══════════════════════════════════════════════════════════════
//  Utilities
// ═══════════════════════════════════════════════════════════════

/// Convert a string to a UTF-16LE byte sequence (for binary search in DOC files)
pub fn to_utf16le_bytes(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

// ═══════════════════════════════════════════════════════════════
//  Scan orchestrator — runs the scan pipeline in a background thread
// ═══════════════════════════════════════════════════════════════

/// Run a full scan pipeline in a background thread.
/// Called from `app::input::start_scan`.
pub fn run_scan(
    tx: mpsc::Sender<ScanMessage>,
    dirs: Vec<String>,
    keyword: String,
    enabled_exts: Vec<String>,
    threads: usize,
) {
    std::thread::spawn(move || {
        let utf16_pattern = to_utf16le_bytes(&keyword);
        let utf8_pattern = keyword.as_bytes();

        // Collect all candidate files
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
            Err(_) => {
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

                // Use the scanner registry for new-style scanners
                let hit = if let Some(scanner) = scanner_for_ext(&ext) {
                    scanner.search(path, &keyword)
                } else {
                    // Fallback for extensions not covered by Scanner trait
                    // (kept for backward compat; all current types should be covered)
                    match ext.as_str() {
                        "doc" | "wps" => {
                            // These need the binary patterns for efficient search
                            doc::search_raw(&path_str, &utf16_pattern, utf8_pattern)
                                .unwrap_or(false)
                        }
                        _ => false,
                    }
                };

                if hit {
                    let file_type = FileType::from_ext(ext.as_str()).unwrap_or(FileType::Text);
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
