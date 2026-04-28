use std::fs::File;
use std::io::Read;

use anyhow::{Context, Result};
use calamine::{Reader, open_workbook_auto};
use cfb::CompoundFile;
use memchr::memmem;
use zip::ZipArchive;

/// 将字符串转换为 UTF-16LE 字节序列
pub fn to_utf16le_bytes(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

/// 搜索 DOCX (过滤 XML 标签，防止因字体/样式改变导致搜索词被打断)
pub fn search_docx(path: &str, keyword: &str) -> Result<bool> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut xml = String::new();
    if let Ok(mut f) = archive.by_name("word/document.xml") {
        f.read_to_string(&mut xml)
            .context("读取 document.xml 失败")?;
    } else {
        return Ok(false);
    }

    // 剥离 XML 标签，提取纯文本
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

/// 搜索老版本 DOC/WPS 文件
pub fn search_doc(path: &str, keyword_utf16: &[u8], keyword_utf8: &[u8]) -> Result<bool> {
    let file = File::open(path)?;
    let mut comp = CompoundFile::open(file)?;

    let mut stream = comp.open_stream("WordDocument")?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;

    Ok(memmem::find(&buf, keyword_utf16).is_some() || memmem::find(&buf, keyword_utf8).is_some())
}

/// 搜索 Excel 文件
pub fn search_excel(path: &str, keyword: &str) -> Result<bool> {
    let mut workbook = open_workbook_auto(path)?;

    for sheet_name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            for row in range.rows() {
                for cell in row {
                    if cell.to_string().contains(keyword) {
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}

/// 搜索 PDF 文件
pub fn search_pdf(path: &str, keyword: &str) -> Result<bool> {
    let doc = lopdf::Document::load(path)?;
    for page in doc.get_pages() {
        let text = doc.extract_text(&[page.0])?;
        let res: String = text
            .chars()
            .filter(|c| !matches!(c, ' ' | '\n' | '\r'))
            .collect();
        if res.contains(keyword) {
            return Ok(true);
        }
    }
    Ok(false)
}
