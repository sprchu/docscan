use std::path::Path;

use super::Scanner;
use crate::app::FileType;

/// Scanner for PDF files
pub struct PdfScanner;

impl Scanner for PdfScanner {
    fn file_type(&self) -> FileType {
        FileType::Pdf
    }

    fn search(&self, path: &Path, keyword: &str) -> bool {
        search_pdf(path, keyword)
    }
}

/// Search a PDF file for `keyword`, stripping whitespace to handle
/// PDF's tendency to insert spaces/newlines between characters.
pub fn search_pdf(path: &Path, keyword: &str) -> bool {
    let doc = match lopdf::Document::load(path) {
        Ok(d) => d,
        Err(_) => return false,
    };

    for page in doc.get_pages() {
        let text = match doc.extract_text(&[page.0]) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let compact: String = text
            .chars()
            .filter(|c| !matches!(c, ' ' | '\n' | '\r'))
            .collect();
        if compact.contains(keyword) {
            return true;
        }
    }

    false
}
