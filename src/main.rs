use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use calamine::{Reader, open_workbook_auto};
use cfb::CompoundFile;
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use memchr::memmem;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Parser, Debug)]
#[command(version, about = "word/excel/pdf 文档搜索工具")]
struct Args {
    #[arg(short, help = "待搜索字符串")]
    query: String,

    #[arg(short, default_value_t = num_cpus::get(), help = "并发线程数")]
    jobs: usize,

    #[arg(short, long, default_value_t = false, help = "是否搜素PDF文件")]
    pdf: bool,

    #[arg(required = true, help = "扫描目录列表")]
    dirs: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let keyword = args.query;
    let utf16_pattern = to_utf16le_bytes(&keyword);
    let utf8_pattern = keyword.as_bytes();

    // 初始化线程池
    ThreadPoolBuilder::new()
        .num_threads(args.jobs)
        .build_global()
        .expect("无法创建全局线程池");

    println!(
        "{} {}\n{} {}",
        "🔍 搜索字符串:".blue(),
        keyword.yellow().bold(),
        "📂 扫描目录:".blue(),
        args.dirs.join(" ").green()
    );
    println!("⚙️ 使用线程数: {}", args.jobs);

    let start_time = Instant::now();

    // 收集文件
    let files: Vec<_> = args
        .dirs
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
                        matches!(
                            ext.to_str().unwrap_or("").to_lowercase().as_str(),
                            "docx" | "wps" | "doc" | "xlsx" | "xls" | "et" | "pdf"
                        )
                    })
                })
        })
        .collect();

    let total = files.len() as u64;
    if total == 0 {
        println!("{}", "⚠️ 未在指定目录中找到支持的文档文件。".yellow());
        return;
    }

    // 初始化进度条
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({per_sec})",
        )
        .unwrap()
        .progress_chars("=>-"),
    );

    let hit_count = AtomicUsize::new(0);

    // 并行处理文件
    files.par_iter().for_each(|entry| {
        let path = entry.path();
        let path_str = path.to_string_lossy(); // 避免非 UTF-8 路径导致的 panic
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let hit = match ext.as_str() {
            "docx" => search_docx(&path_str, &keyword).unwrap_or(false),
            "doc" | "wps" => search_doc(&path_str, &utf16_pattern, utf8_pattern).unwrap_or(false),
            "xlsx" | "xls" | "et" => search_excel(&path_str, &keyword).unwrap_or(false),
            "pdf" => args
                .pdf
                .then(|| search_pdf(&path_str, &keyword).unwrap_or(false))
                .unwrap_or(false),
            _ => false,
        };

        if hit {
            hit_count.fetch_add(1, Ordering::Relaxed);

            pb.println(format!(
                "{} {}",
                "📄 命中:".green().bold(),
                path.display().to_string().bold()
            ));
        }

        pb.inc(1);
    });

    pb.finish_and_clear();

    let duration = start_time.elapsed();

    println!(
        "{} {} 个文件",
        "✅ 命中数量:".green().bold(),
        hit_count.load(Ordering::Relaxed)
    );

    println!("{} {:.2?}", "⏱ 总耗时:".blue().bold(), duration);
    println!("{}", "🎉 搜索完成!".blue());
}

/// 将字符串转换为 UTF-16LE 字节序列
fn to_utf16le_bytes(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

/// 搜索 DOCX (过滤 XML 标签，防止因字体/样式改变导致搜索词被打断)
fn search_docx(path: &str, keyword: &str) -> Result<bool> {
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
fn search_doc(path: &str, keyword_utf16: &[u8], keyword_utf8: &[u8]) -> Result<bool> {
    let file = File::open(path)?;
    let mut comp = CompoundFile::open(file)?;

    let mut stream = comp.open_stream("WordDocument")?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;

    // 同时搜索 UTF-16LE 和 UTF-8/ANSI 格式的字节
    // 老式 DOC 文件中有时英文/简单文本以单字节形式存在
    Ok(memmem::find(&buf, keyword_utf16).is_some() || memmem::find(&buf, keyword_utf8).is_some())
}

/// 搜索 Excel 文件
fn search_excel(path: &str, keyword: &str) -> Result<bool> {
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
fn search_pdf(path: &str, keyword: &str) -> Result<bool> {
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
