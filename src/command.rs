use crate::app::App;

pub fn execute(app: &mut App) {
    let cmd = app.command.trim().to_string();

    if cmd.is_empty() {
        app.exit_command_mode();
        return;
    }

    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let action = parts[0];
    let arg = parts.get(1).unwrap_or(&"").trim();

    match action {
        "q" | "quit" => {
            app.should_quit = true;
        }

        // ── 配置命令 ──
        "query" | "k" => {
            if arg.is_empty() {
                app.message = format!("Current query: '{}'", app.query);
            } else {
                app.query = arg.to_string();
                app.query_cursor = app.query.len();
                app.message = format!("Query set to '{}'", arg);
            }
        }

        "threads" | "jobs" | "j" => {
            if arg.is_empty() {
                app.message = format!("Current threads: {}", app.threads);
            } else {
                match arg.parse::<usize>() {
                    Ok(n) if n > 0 => {
                        app.threads = n;
                        app.message = format!("Threads set to {}", n);
                    }
                    _ => {
                        app.message = "Invalid thread count. Use a positive integer.".to_string();
                    }
                }
            }
        }

        "dir" => {
            let sub_parts: Vec<&str> = arg.splitn(2, ' ').collect();
            let sub = sub_parts[0];
            let sub_arg = sub_parts.get(1).unwrap_or(&"").trim();

            match sub {
                "add" | "a" => {
                    if sub_arg.is_empty() {
                        app.message = "Usage: :dir add <path>".to_string();
                    } else {
                        app.dirs.push(sub_arg.to_string());
                        app.message = format!("Added directory: {}", sub_arg);
                    }
                }
                "rm" | "remove" | "r" => {
                    if app.dirs.is_empty() {
                        app.message = "No directories to remove.".to_string();
                    } else {
                        let index = sub_arg.parse::<usize>().unwrap_or(1);
                        let removed = app.dirs.remove(index - 1);
                        app.message = format!("Removed: {}", removed);
                    }
                }
                "clear" | "c" => {
                    let count = app.dirs.len();
                    app.dirs.clear();
                    app.message = format!(
                        "Cleared {} director{}.",
                        count,
                        if count == 1 { "y" } else { "ies" }
                    );
                }
                _ => {
                    app.message = "Usage: :dir add <path> | :dir rm | :dir clear".to_string();
                }
            }
        }

        // ── 过滤 ──
        "filter" | "f" => {
            if arg.is_empty() || arg == "clear" {
                app.filter = None;
                app.selected = 0;
                app.message = "Filter cleared.".to_string();
            } else {
                let ft = match arg.to_lowercase().as_str() {
                    "word" | "docx" | "doc" | "wps" => Some(crate::app::FileType::Word),
                    "excel" | "xlsx" | "xls" | "et" => Some(crate::app::FileType::Excel),
                    "pdf" => Some(crate::app::FileType::Pdf),
                    _ => None,
                };
                match ft {
                    Some(ft) => {
                        let label = ft.short_label().to_string();
                        app.filter = Some(ft);
                        app.selected = 0;
                        app.message = format!("Filter: {}", label);
                    }
                    None => {
                        app.message = "Unknown type. Use: word, excel, pdf, or clear.".to_string();
                    }
                }
            }
        }

        "help" | "?" => {
            app.mode = crate::app::Mode::Help;
            return;
        }

        _ => {
            app.message = format!("Unknown: {}. Try :help", action);
        }
    }

    app.exit_command_mode();
}
