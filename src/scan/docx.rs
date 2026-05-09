use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use zip::ZipArchive;

use super::Scanner;
use crate::app::FileType;

/// Scanner for `.docx` (Office Open XML) files
pub struct DocxScanner;

impl Scanner for DocxScanner {
    fn file_type(&self) -> FileType {
        FileType::Word
    }

    fn search(&self, path: &Path, keyword: &str) -> bool {
        search_docx(path, keyword).unwrap_or(false)
    }
}

/// Search a DOCX file for `keyword`, stripping XML tags to avoid
/// false negatives caused by formatting tags splitting the search text.
pub fn search_docx(path: &Path, keyword: &str) -> Result<bool> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut xml = String::new();
    if let Ok(mut f) = archive.by_name("word/document.xml") {
        f.read_to_string(&mut xml).context("reading document.xml")?;
    } else {
        return Ok(false);
    }

    // Strip XML tags, extract pure text
    let mut pure_text = String::with_capacity(xml.len() / 2);
    let mut in_tag = false;
    for c in xml.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            pure_text.push(c);
        }
    }

    Ok(pure_text.contains(keyword))
}
