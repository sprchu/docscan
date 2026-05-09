use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::Scanner;
use crate::app::FileType;

/// Scanner for plain UTF-8 text files (`.txt`, `.md`, `.rs`, `.py`, etc.)
///
/// This demonstrates how to add a new scanner to the system:
/// 1. Create a struct implementing the `Scanner` trait
/// 2. Register it in `scan::all_scanners()`
/// 3. Add its extensions to `FileType::Text::extensions()`
pub struct TextScanner;

impl Scanner for TextScanner {
    fn file_type(&self) -> FileType {
        FileType::Text
    }

    fn search(&self, path: &Path, keyword: &str) -> bool {
        search_text(path, keyword)
    }
}

/// Search a plain text file for `keyword` line-by-line.
/// Returns `true` on the first match (short-circuits).
pub fn search_text(path: &Path, keyword: &str) -> bool {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = BufReader::new(file);
    for line in reader.lines() {
        match line {
            Ok(line) if line.contains(keyword) => return true,
            Err(_) => return false, // invalid UTF-8, bail
            _ => {}
        }
    }

    false
}
