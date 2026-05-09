use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use cfb::CompoundFile;
use memchr::memmem;

use super::Scanner;
use crate::app::FileType;

/// Scanner for legacy `.doc` and `.wps` binary files
pub struct DocScanner;

impl Scanner for DocScanner {
    fn file_type(&self) -> FileType {
        FileType::Word
    }

    fn search(&self, path: &Path, keyword: &str) -> bool {
        let utf16_pattern = super::to_utf16le_bytes(keyword);
        let utf8_pattern = keyword.as_bytes();
        search_raw(&path.to_string_lossy(), &utf16_pattern, utf8_pattern).unwrap_or(false)
    }
}

/// Raw binary search in DOC/WPS compound files.
/// Searches for both UTF-16LE and UTF-8 representations of the keyword.
pub fn search_raw(path_str: &str, utf16_pattern: &[u8], utf8_pattern: &[u8]) -> Result<bool> {
    let file = File::open(path_str)?;
    let mut comp = CompoundFile::open(file)?;

    let mut stream = comp.open_stream("WordDocument")?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;

    Ok(memmem::find(&buf, utf16_pattern).is_some() || memmem::find(&buf, utf8_pattern).is_some())
}
