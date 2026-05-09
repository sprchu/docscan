use std::path::Path;

use calamine::{Reader, open_workbook_auto};

use super::Scanner;
use crate::app::FileType;

/// Scanner for Excel files (`.xlsx`, `.xls`, `.et`)
pub struct ExcelScanner;

impl Scanner for ExcelScanner {
    fn file_type(&self) -> FileType {
        FileType::Excel
    }

    fn search(&self, path: &Path, keyword: &str) -> bool {
        search_excel(path, keyword)
    }
}

/// Search Excel file for `keyword` across all sheets
pub fn search_excel(path: &Path, keyword: &str) -> bool {
    let mut workbook = match open_workbook_auto(path) {
        Ok(wb) => wb,
        Err(_) => return false,
    };

    for sheet_name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            for row in range.rows() {
                for cell in row {
                    if cell.to_string().contains(keyword) {
                        return true;
                    }
                }
            }
        }
    }

    false
}
