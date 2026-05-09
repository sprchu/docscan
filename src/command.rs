use crate::app::App;

pub fn execute(app: &mut App) {
    let cmd = app.command.trim().to_string();

    if cmd.is_empty() {
        app.exit_command_mode();
        return;
    }

    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let action = parts[0];
    let arg = parts.get(1).copied().unwrap_or("").trim();

    match action {
        "q" | "quit" => {
            app.should_quit = true;
        }

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
            let sub_arg = sub_parts.get(1).copied().unwrap_or("").trim();

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
                    let active: Vec<usize> = app
                        .dirs
                        .iter()
                        .enumerate()
                        .filter(|(_, d)| !d.trim().is_empty())
                        .map(|(i, _)| i)
                        .collect();
                    if active.is_empty() {
                        app.message = "No directories to remove.".to_string();
                    } else {
                        let index: usize = sub_arg.parse().unwrap_or(1);
                        let pos = index.saturating_sub(1).min(active.len().saturating_sub(1));
                        let real_idx = active[pos];
                        let removed = app.dirs.remove(real_idx);
                        app.message = format!("Removed: {}", removed);
                    }
                }
                "clear" | "c" => {
                    let count = app.active_dirs().len();
                    app.dirs.clear();
                    app.dir_selected = 0;
                    app.message = format!(
                        "Cleared {} director{}.",
                        count,
                        if count == 1 { "y" } else { "ies" }
                    );
                }
                "browse" | "b" => {
                    app.enter_browse_mode();
                    return;
                }
                _ => {
                    app.message = "Usage: :dir add <path> | :dir rm [n] | :dir clear | :dir browse"
                        .to_string();
                }
            }
        }

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

        _ => {
            app.message = format!("Unknown: {}. Try :q :query :threads :dir :filter", action);
        }
    }

    app.exit_command_mode();
}
