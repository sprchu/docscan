use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, Mode};

const CONFIG_HEIGHT: u16 = 7;
const LABEL_W: u16 = 10;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let vchunks = Layout::vertical([
        Constraint::Length(CONFIG_HEIGHT),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    render_config_panel(f, app, vchunks[0]);
    render_results_table(f, app, vchunks[1]);
    render_command_bar(f, app, vchunks[2]);

    if app.mode == Mode::Help {
        render_help_popup(f, area);
    }
    if app.mode == Mode::Browse {
        render_browse_popup(f, app, area);
    }
}

// ==================== Config panel ====================

fn render_config_panel(f: &mut Frame, app: &App, area: Rect) {
    // Split vertically: panels on top, hint at bottom
    let vchunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    // Split panels area horizontally: left (Params) / right (Paths)
    let hchunks = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(vchunks[0]);

    render_params_panel(f, app, hchunks[0]);
    render_paths_panel(f, app, hchunks[1]);

    // Hint line
    let hint = match app.focus {
        Focus::ConfigLeft => match app.config_left_row {
            0 => "Type keyword | ←→ move cursor | Backspace delete",
            1 => "←→ adjust threads",
            2 => "←→ select type | Space toggle",
            _ => "",
        },
        Focus::ConfigRight => "↑↓ browse | e picker edit | d delete | a picker add | Enter scan",
        Focus::Results => "",
    };
    f.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
        vchunks[1],
    );
}

// ==================== Left: Params ====================

fn param_styles(selected: bool) -> (Style, Style) {
    let label = if selected {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::Gray)
    };
    let row = if selected {
        Style::default().bg(Color::Rgb(30, 40, 50))
    } else {
        Style::default()
    };
    (label, row)
}

fn render_label(f: &mut Frame, text: &str, label_style: Style, row_style: Style, x: u16, y: u16) {
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("{:w$}", text, w = LABEL_W as usize),
            label_style,
        )]))
        .style(row_style),
        Rect::new(x, y, LABEL_W, 1),
    );
}

fn render_params_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::ConfigLeft;
    let border_color = if is_focused { Color::Cyan } else { Color::Gray };

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(" Params ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let x = inner.x + 1;
    let y = inner.y;
    let w = inner.width.saturating_sub(2);

    render_keyword_row(f, app, is_focused, x, y, w);
    render_threads_row(f, app, is_focused, x, y + 1, w);
    render_types_row(f, app, is_focused, x, y + 2, w);
}

// ── Keyword row ──

fn render_keyword_row(f: &mut Frame, app: &App, is_focused: bool, x: u16, y: u16, w: u16) {
    let sel = is_focused && app.config_left_row == 0;
    let (label_style, row_style) = param_styles(sel);

    render_label(f, "Keyword", label_style, row_style, x, y);

    let value = &app.query;
    let display = if value.is_empty() {
        Span::styled("—", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(value, Style::default().fg(Color::White))
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![display])).style(row_style),
        Rect::new(x + LABEL_W, y, w.saturating_sub(LABEL_W), 1),
    );

    if sel {
        let prefix = UnicodeWidthStr::width(&value[..app.query_cursor.min(value.len())]) as u16;
        f.set_cursor_position((x + LABEL_W + prefix, y));
    }
}

// ── Threads row ──

fn render_threads_row(f: &mut Frame, app: &App, is_focused: bool, x: u16, y: u16, w: u16) {
    let sel = is_focused && app.config_left_row == 1;
    let (label_style, row_style) = param_styles(sel);
    let arrow = if sel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    render_label(f, "Threads", label_style, row_style, x, y);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("◀ ", arrow),
            Span::styled(app.threads.to_string(), Style::default().fg(Color::White)),
            Span::styled(" ▶", arrow),
        ]))
        .style(row_style),
        Rect::new(x + LABEL_W, y, w.saturating_sub(LABEL_W), 1),
    );
}

// ── File types row ──

fn render_types_row(f: &mut Frame, app: &App, is_focused: bool, x: u16, y: u16, w: u16) {
    let sel = is_focused && app.config_left_row == 2;
    let (label_style, row_style) = param_styles(sel);

    render_label(f, "Types", label_style, row_style, x, y);

    let parts: Vec<Span> = app
        .file_types
        .iter()
        .enumerate()
        .flat_map(|(i, (ft, enabled))| {
            let cursor = sel && app.ft_cursor == i;
            let marker = if *enabled { "●" } else { "○" };
            let style = if cursor {
                Style::default().fg(Color::Cyan).bold()
            } else if *enabled {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let arrow = if cursor {
                Span::styled(" ⬅", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("  ")
            };
            vec![
                Span::styled(format!("{} ", marker), style),
                Span::styled(ft.short_label(), style),
                arrow,
                Span::raw("  "),
            ]
        })
        .collect();

    f.render_widget(
        Paragraph::new(Line::from(parts)).style(row_style),
        Rect::new(x + LABEL_W, y, w.saturating_sub(LABEL_W), 1),
    );
}

// ==================== Right: Paths ====================

fn render_paths_panel(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::ConfigRight;
    let border_color = if is_focused { Color::Cyan } else { Color::Gray };

    let active_count = app.active_dirs().len();
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Paths ({}) ", active_count))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows: Vec<Row> = app
        .dirs
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let is_sel = is_focused && i == app.dir_selected;
            let is_empty = d.trim().is_empty();
            let style = if is_sel {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if i % 2 == 0 {
                Style::default().bg(Color::Rgb(30, 30, 40))
            } else {
                Style::default()
            };
            let num = if is_empty {
                "+".to_string()
            } else {
                format!("{}", i + 1)
            };
            let path_text = if is_empty {
                Span::styled("<add new path>", Style::default().fg(Color::DarkGray))
            } else {
                Span::raw(d.to_owned())
            };
            Row::new(vec![Cell::from(num), Cell::from(path_text)]).style(style)
        })
        .collect();

    let widths = [Constraint::Length(4), Constraint::Fill(1)];

    let mut table_state = TableState::default().with_selected(if is_focused {
        Some(app.dir_selected)
    } else {
        None
    });

    f.render_stateful_widget(
        Table::new(rows, widths)
            .header(Row::new(vec!["#", "Path"]))
            .column_spacing(1),
        inner,
        &mut table_state,
    );
}

// ==================== Results ====================

fn render_results_table(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Results;
    let border_color = if is_focused { Color::Cyan } else { Color::Gray };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Results ({}) ", app.filtered_results().len()))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let filtered = app.filtered_results();

    if filtered.is_empty() {
        let msg = if app.scanning {
            "Scanning..."
        } else if !app.results.is_empty() {
            "No results match the current filter."
        } else {
            "No results yet. Set keyword, dirs, file types, then Enter to scan."
        };
        f.render_widget(
            Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("#"),
        Cell::from("Path"),
        Cell::from("Type"),
    ])
    .style(Style::default().fg(Color::Black).bg(Color::Gray).bold());

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if is_focused && i == app.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if i % 2 == 0 {
                Style::default().bg(Color::Rgb(30, 30, 40))
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(format!("{}", i + 1)),
                Cell::from(r.path.to_string_lossy().to_string()),
                Cell::from(r.file_type.short_label()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Fill(1),
        Constraint::Length(10),
    ];

    let mut table_state = TableState::default().with_selected(if is_focused {
        Some(app.selected.min(filtered.len().saturating_sub(1)))
    } else {
        None
    });

    f.render_stateful_widget(
        Table::new(rows, widths).header(header).column_spacing(1),
        inner,
        &mut table_state,
    );
}

// ==================== Command bar ====================

fn render_command_bar(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match app.mode {
        Mode::Command => {
            let text = format!(":{}", app.command);
            let cursor_pos = 1 + UnicodeWidthStr::width(&app.command[..app.command_cursor]) as u16;
            f.render_widget(Paragraph::new(text), inner);
            f.set_cursor_position((inner.x + cursor_pos, inner.y));
        }
        Mode::Help => {
            f.render_widget(
                Paragraph::new("Press any key to close help.")
                    .style(Style::default().fg(Color::Gray)),
                inner,
            );
        }
        Mode::Normal => {
            f.render_widget(
                Paragraph::new(app.message.as_str()).style(Style::default().fg(Color::Gray)),
                inner,
            );
        }
        Mode::Browse => {}
    }
}

// ==================== Help popup ====================

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 75, area);

    f.render_widget(Clear, popup_area);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let keys = vec![
        Line::from(Span::styled(
            "Key bindings",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab     ", Style::default().fg(Color::Cyan)),
            Span::raw("Cycle focus: Params → Paths → Results"),
        ]),
        Line::from(vec![
            Span::styled("  ↑↓      ", Style::default().fg(Color::Cyan)),
            Span::raw("Navigate items in the focused panel"),
        ]),
        Line::from(vec![
            Span::styled("  ←→      ", Style::default().fg(Color::Cyan)),
            Span::raw("Adjust threads / select file type"),
        ]),
        Line::from(vec![
            Span::styled("  Space   ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle file type (on Types row)"),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", Style::default().fg(Color::Cyan)),
            Span::raw("Start scan / open selected file"),
        ]),
        Line::from(vec![
            Span::styled("  e       ", Style::default().fg(Color::Cyan)),
            Span::raw("Pick new path for selected entry"),
        ]),
        Line::from(vec![
            Span::styled("  d       ", Style::default().fg(Color::Cyan)),
            Span::raw("Delete selected path"),
        ]),
        Line::from(vec![
            Span::styled("  a       ", Style::default().fg(Color::Cyan)),
            Span::raw("Pick new path to add"),
        ]),
        Line::from(vec![
            Span::styled("  :       ", Style::default().fg(Color::Cyan)),
            Span::raw("Enter command mode"),
        ]),
        Line::from(vec![
            Span::styled("  q       ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Path List",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑↓      ", Style::default().fg(Color::Cyan)),
            Span::raw("Select path entry"),
        ]),
        Line::from(vec![
            Span::styled("  e       ", Style::default().fg(Color::Cyan)),
            Span::raw("Open picker to replace selected entry"),
        ]),
        Line::from(vec![
            Span::styled("  d       ", Style::default().fg(Color::Cyan)),
            Span::raw("Delete selected entry"),
        ]),
        Line::from(vec![
            Span::styled("  a       ", Style::default().fg(Color::Cyan)),
            Span::raw("Open picker to add new directory"),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", Style::default().fg(Color::Cyan)),
            Span::raw("Start scan"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Directory Browser",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑↓/jk   ", Style::default().fg(Color::Cyan)),
            Span::raw("Navigate entries"),
        ]),
        Line::from(vec![
            Span::styled("  Enter   ", Style::default().fg(Color::Cyan)),
            Span::raw("Open subdirectory / select current"),
        ]),
        Line::from(vec![
            Span::styled("  Space   ", Style::default().fg(Color::Cyan)),
            Span::raw("Select current directory"),
        ]),
        Line::from(vec![
            Span::styled("  Bksp    ", Style::default().fg(Color::Cyan)),
            Span::raw("Go to parent directory"),
        ]),
        Line::from(vec![
            Span::styled("  ~       ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to home directory"),
        ]),
        Line::from(vec![
            Span::styled("  /       ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to root (/ or C:\\)"),
        ]),
        Line::from(vec![
            Span::styled("  Esc     ", Style::default().fg(Color::Cyan)),
            Span::raw("Cancel / close browser"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Commands",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  :q, :quit          ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit the program"),
        ]),
        Line::from(vec![
            Span::styled("  :query <text>      ", Style::default().fg(Color::Cyan)),
            Span::raw("Set search keyword (:k)"),
        ]),
        Line::from(vec![
            Span::styled("  :threads <n>       ", Style::default().fg(Color::Cyan)),
            Span::raw("Set thread count (:j)"),
        ]),
        Line::from(vec![
            Span::styled("  :dir add <path>    ", Style::default().fg(Color::Cyan)),
            Span::raw("Add scan directory (:a)"),
        ]),
        Line::from(vec![
            Span::styled("  :dir rm <n>        ", Style::default().fg(Color::Cyan)),
            Span::raw("Remove directory (:r)"),
        ]),
        Line::from(vec![
            Span::styled("  :dir clear         ", Style::default().fg(Color::Cyan)),
            Span::raw("Clear all directories (:c)"),
        ]),
        Line::from(vec![
            Span::styled("  :dir browse        ", Style::default().fg(Color::Cyan)),
            Span::raw("Open directory browser (:b)"),
        ]),
        Line::from(vec![
            Span::styled("  :filter <type>     ", Style::default().fg(Color::Cyan)),
            Span::raw("Filter: word/excel/pdf/clear (:f)"),
        ]),
        Line::from(vec![
            Span::styled("  :help              ", Style::default().fg(Color::Cyan)),
            Span::raw("Show this help (:?)"),
        ]),
    ];

    f.render_widget(Paragraph::new(Text::from(keys)), inner);
}

// ==================== Browse popup ====================

fn render_browse_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(65, 75, area);

    f.render_widget(Clear, popup_area);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Browse Directory ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split: header line + list area + footer hint
    let vchunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(inner);

    // Header: current path
    let cwd_display = app.browse_cwd.to_string_lossy().to_string();
    let header_text = vec![
        Line::from(Span::styled(
            "Current:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            format!("📁 {}", cwd_display),
            Style::default().fg(Color::Yellow),
        )),
    ];
    f.render_widget(Paragraph::new(Text::from(header_text)), vchunks[0]);

    // Directory listing
    if app.browse_entries.is_empty() {
        f.render_widget(
            Paragraph::new("(empty directory)")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            vchunks[1],
        );
    } else {
        let rows: Vec<Row> = app
            .browse_entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let is_selected = i == app.browse_selected;
                let icon = "📁";
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else if i % 2 == 0 {
                    Style::default().bg(Color::Rgb(25, 25, 35))
                } else {
                    Style::default()
                };
                Row::new(vec![Cell::from(format!("  {} {}", icon, entry.name))]).style(style)
            })
            .collect();

        let widths = [Constraint::Fill(1)];
        let mut table_state = TableState::default().with_selected(Some(app.browse_selected));

        f.render_stateful_widget(
            Table::new(rows, widths).column_spacing(1),
            vchunks[1],
            &mut table_state,
        );
    }

    // Footer hints
    let hints = vec![
        Span::styled("Enter", Style::default().fg(Color::Cyan).bold()),
        Span::raw(":open dir  "),
        Span::styled("Space", Style::default().fg(Color::Cyan).bold()),
        Span::raw(":select  "),
        Span::styled("Backspace", Style::default().fg(Color::Cyan).bold()),
        Span::raw(":parent  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan).bold()),
        Span::raw(":cancel  "),
        Span::styled("~", Style::default().fg(Color::Cyan).bold()),
        Span::raw(":home"),
    ];
    f.render_widget(
        Paragraph::new(Line::from(hints)).alignment(Alignment::Center),
        vchunks[2],
    );
}
