use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, FileType, Focus, Mode, UiRects};

// ── Theme ──

mod theme {
    use ratatui::style::Color;
    pub const ACCENT: Color = Color::Cyan;
    pub const FG: Color = Color::White;
    pub const FG_DIM: Color = Color::Gray;
    pub const FG_MUTED: Color = Color::DarkGray;
    pub const BG_ROW_EVEN: Color = Color::Rgb(28, 28, 38);
    pub const SELECTED_BG: Color = Color::Cyan;
    pub const SELECTED_FG: Color = Color::Black;
    pub const BORDER_FOCUSED: Color = Color::Cyan;
    pub const BORDER_UNFOCUSED: Color = Color::Gray;
    pub const BUTTON_BG: Color = Color::Rgb(40, 55, 70);
    pub const DANGER: Color = Color::Rgb(220, 80, 80);
    pub const SUCCESS: Color = Color::Rgb(80, 200, 120);
    pub const WARNING: Color = Color::Rgb(220, 180, 60);
}

const CONFIG_HEIGHT: u16 = 8;
const LABEL_W: u16 = 9;

pub fn render(f: &mut Frame, app: &App) -> UiRects {
    let area = f.area();
    let mut rects = UiRects::default();

    let vchunks = Layout::vertical([
        Constraint::Length(CONFIG_HEIGHT),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    render_config_panel(f, app, vchunks[0], &mut rects);
    render_results_panel(f, app, vchunks[1], &mut rects);
    render_status_bar(f, app, vchunks[2]);

    if app.mode == Mode::Browse {
        render_browse_popup(f, app, area, &mut rects);
    }

    // Cursor for filter input (only when filter_focused)
    if app.mode == Mode::Normal
        && app.focus == Focus::Results
        && app.filter_focused
        && !app.filter_text.is_empty()
    {
        if let Some(ref r) = rects.filter_input {
            let vis_w = r.width.saturating_sub(1) as usize;
            let (_, cursor_offset) =
                scroll_text_to_cursor(&app.filter_text, app.filter_text_cursor, vis_w);
            f.set_cursor_position((r.x + cursor_offset, r.y));
        }
    }

    rects
}

// ==================== Config panel ====================

fn render_config_panel(f: &mut Frame, app: &App, area: Rect, rects: &mut UiRects) {
    let hchunks =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).split(area);

    render_params_panel(f, app, hchunks[0], rects);
    render_paths_panel(f, app, hchunks[1], rects);
}

// ==================== Left: Params ====================

fn param_label_style(selected: bool) -> Style {
    if selected {
        Style::default().fg(theme::ACCENT).bold()
    } else {
        Style::default().fg(theme::FG_DIM)
    }
}

fn param_row_style(selected: bool) -> Style {
    if selected {
        Style::default().bg(theme::BUTTON_BG)
    } else {
        Style::default()
    }
}

fn render_params_panel(f: &mut Frame, app: &App, area: Rect, rects: &mut UiRects) {
    let is_focused = app.focus == Focus::ConfigLeft;
    let border_color = if is_focused {
        theme::BORDER_FOCUSED
    } else {
        theme::BORDER_UNFOCUSED
    };

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

    render_keyword_row(f, app, is_focused, x, y, w, rects);
    render_threads_row(f, app, is_focused, x, y + 1, w, rects);
    render_types_row(f, app, is_focused, x, y + 2, w, rects);
}

// ── Keyword row + Scan button ──

fn render_keyword_row(
    f: &mut Frame,
    app: &App,
    is_focused: bool,
    x: u16,
    y: u16,
    w: u16,
    rects: &mut UiRects,
) {
    let sel = is_focused && app.config_left_row == 0;
    let label_style = param_label_style(sel);
    let row_style = param_row_style(sel);

    // Label
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("{:w$}", "Keyword", w = LABEL_W as usize),
            label_style,
        )]))
        .style(row_style),
        Rect::new(x, y, LABEL_W, 1),
    );

    let btn_w = 8u16;
    let gap = 1u16;
    let input_w = w.saturating_sub(LABEL_W + btn_w + gap);

    // Guard: minimum usable input width
    if input_w < 4 {
        return; // too narrow to render anything useful
    }

    let value = &app.query;

    // ── Scroll input text so cursor stays visible ──
    let (visible_text, cursor_offset) = if value.is_empty() {
        (String::new(), 0u16)
    } else {
        let vis_w = input_w.saturating_sub(1) as usize; // reserve 1 cell for cursor
        scroll_text_to_cursor(value, app.query_cursor, vis_w)
    };

    let display = if visible_text.is_empty() {
        Span::styled("(type keyword)", Style::default().fg(theme::FG_MUTED))
    } else {
        Span::styled(&visible_text, Style::default().fg(theme::FG))
    };
    let input_rect = Rect::new(x + LABEL_W, y, input_w, 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![display])).style(row_style),
        input_rect,
    );
    rects.keyword_input = Some(input_rect);

    if sel && !value.is_empty() {
        f.set_cursor_position((x + LABEL_W + cursor_offset, y));
    }

    // Scan button
    let btn_x = x + LABEL_W + input_w + gap;
    let btn_rect = Rect::new(btn_x, y, btn_w, 1);
    let btn_style = if sel {
        Style::default()
            .fg(theme::SELECTED_FG)
            .bg(theme::ACCENT)
            .bold()
    } else {
        Style::default().fg(theme::FG).bg(theme::BUTTON_BG)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled("  Scan  ", btn_style)])),
        btn_rect,
    );
    rects.scan_button = Some(btn_rect);
}

// ── Threads row with clickable arrows ──

fn render_threads_row(
    f: &mut Frame,
    app: &App,
    is_focused: bool,
    x: u16,
    y: u16,
    _w: u16,
    rects: &mut UiRects,
) {
    let sel = is_focused && app.config_left_row == 1;
    let label_style = param_label_style(sel);
    let row_style = param_row_style(sel);

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("{:w$}", "Threads", w = LABEL_W as usize),
            label_style,
        )]))
        .style(row_style),
        Rect::new(x, y, LABEL_W, 1),
    );

    let arrow_style = if sel {
        Style::default().fg(theme::ACCENT).bold()
    } else {
        Style::default().fg(theme::FG_DIM)
    };

    // ◀ button
    let dec_x = x + LABEL_W;
    let dec_rect = Rect::new(dec_x, y, 2, 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled("◀ ", arrow_style)])).style(row_style),
        dec_rect,
    );
    rects.threads_dec_btn = Some(dec_rect);

    // Value
    let val_text = app.threads.to_string();
    let val_x = dec_x + 2;
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            &val_text,
            Style::default().fg(theme::FG),
        )]))
        .style(row_style),
        Rect::new(val_x, y, val_text.len() as u16, 1),
    );

    // ▶ button
    let inc_x = val_x + val_text.len() as u16;
    let inc_rect = Rect::new(inc_x, y, 2, 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(" ▶", arrow_style)])).style(row_style),
        inc_rect,
    );
    rects.threads_inc_btn = Some(inc_rect);
}

// ── File types row with clickable toggles ──

fn render_types_row(
    f: &mut Frame,
    app: &App,
    is_focused: bool,
    x: u16,
    y: u16,
    w: u16,
    rects: &mut UiRects,
) {
    let sel = is_focused && app.config_left_row == 2;
    let label_style = if sel {
        Style::default().fg(theme::ACCENT).bold()
    } else {
        Style::default().fg(theme::FG_DIM)
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("{:w$}", "Types", w = LABEL_W as usize),
            label_style,
        )])),
        Rect::new(x, y, LABEL_W, 1),
    );

    rects.type_btns.clear();
    let right_edge = x + w;
    let mut cx = x + LABEL_W;

    for (i, (ft, enabled)) in app.file_types.iter().enumerate() {
        let label = ft.short_label();
        let marker = if *enabled { "●" } else { "○" };
        let text = format!("{} {}", marker, label);
        let text_w = text.len() as u16;

        // Clip: stop drawing if the button would overflow the panel
        if cx + text_w > right_edge && cx > x + LABEL_W {
            // Show overflow indicator
            let indicator = "…";
            f.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    indicator,
                    Style::default().fg(theme::FG_MUTED),
                )])),
                Rect::new(cx, y, 1, 1),
            );
            break;
        }

        let is_cursor = is_focused && i == app.ft_cursor;
        let btn_style = if is_cursor && sel {
            Style::default().fg(theme::ACCENT).bold()
        } else if *enabled {
            Style::default().fg(theme::SUCCESS)
        } else {
            Style::default().fg(theme::FG_MUTED)
        };

        let btn_rect = Rect::new(cx, y, text_w, 1);
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(&text, btn_style)])),
            btn_rect,
        );
        rects.type_btns.push(btn_rect);
        cx += text_w;
    }
}

// ==================== Right: Paths panel ====================

fn render_paths_panel(f: &mut Frame, app: &App, area: Rect, rects: &mut UiRects) {
    let is_focused = app.focus == Focus::ConfigRight;
    let border_color = if is_focused {
        theme::BORDER_FOCUSED
    } else {
        theme::BORDER_UNFOCUSED
    };

    let active_count = app.active_dirs().len();
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Paths ({}) ", active_count))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    f.render_widget(block, area);
    rects.paths_panel = Some(inner);

    // Header with action buttons
    let header_h = 1u16;
    let list_y = inner.y + header_h;
    let list_h = inner.height.saturating_sub(header_h);

    let btn_w = 8u16;
    let gap = 1u16;
    let total_btn_w = btn_w * 3 + gap * 2;
    let header_x = inner.x + inner.width.saturating_sub(total_btn_w);

    let add_rect = Rect::new(header_x, inner.y, btn_w, 1);
    let edit_rect = Rect::new(header_x + btn_w + gap, inner.y, btn_w, 1);
    let del_rect = Rect::new(header_x + (btn_w + gap) * 2, inner.y, btn_w, 1);

    let btn_base = Style::default().fg(theme::FG);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(" + Add  ", btn_base)]))
            .style(Style::default().bg(theme::BUTTON_BG)),
        add_rect,
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(" ✎ Edit ", btn_base)]))
            .style(Style::default().bg(theme::BUTTON_BG)),
        edit_rect,
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " ✕ Del  ",
            Style::default().fg(theme::DANGER),
        )]))
        .style(Style::default().bg(theme::BUTTON_BG)),
        del_rect,
    );

    rects.path_add_btn = Some(add_rect);
    rects.path_edit_btn = Some(edit_rect);
    rects.path_del_btn = Some(del_rect);

    // Path list
    rects.path_rows.clear();

    if app.dirs.is_empty() {
        f.render_widget(
            Paragraph::new("(no directories — click + Add)")
                .style(Style::default().fg(theme::FG_MUTED))
                .alignment(Alignment::Center),
            Rect::new(inner.x, list_y, inner.width, list_h),
        );
        return;
    }

    let visible = list_h as usize;
    let start = app
        .dir_selected
        .saturating_sub(visible.saturating_sub(1))
        .min(app.dirs.len().saturating_sub(visible));
    let start = start.min(app.dir_selected);
    rects.path_list_start = start;

    let rows: Vec<Row> = app
        .dirs
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(i, d)| {
            let is_sel = is_focused && i == app.dir_selected;
            let style = if is_sel {
                Style::default()
                    .fg(theme::SELECTED_FG)
                    .bg(theme::SELECTED_BG)
            } else if i % 2 == 0 {
                Style::default().bg(theme::BG_ROW_EVEN)
            } else {
                Style::default()
            };
            let num = format!("{}", i + 1);
            Row::new(vec![
                Cell::from(num),
                Cell::from(Span::styled(d.to_owned(), Style::default().fg(theme::FG))),
            ])
            .style(style)
        })
        .collect();

    let row_count = rows.len();
    let widths = [Constraint::Length(4), Constraint::Fill(1)];

    let mut table_state = TableState::default().with_selected(if is_focused {
        Some((app.dir_selected.saturating_sub(start)).min(row_count.saturating_sub(1)))
    } else {
        None
    });

    let table_w = inner.width.saturating_sub(1);
    let list_area = Rect::new(inner.x, list_y, table_w, list_h);
    f.render_stateful_widget(
        Table::new(rows, widths).column_spacing(1),
        list_area,
        &mut table_state,
    );

    let sb_area = Rect::new(inner.x + table_w, list_y, 1, list_h);
    render_scrollbar(f, sb_area, app.dirs.len(), visible, start);

    // Record row rects for click handling
    for i in 0..row_count {
        rects
            .path_rows
            .push(Rect::new(inner.x, list_y + i as u16, inner.width, 1));
    }
}

// ==================== Results panel ====================

fn render_results_panel(f: &mut Frame, app: &App, area: Rect, rects: &mut UiRects) {
    let is_focused = app.focus == Focus::Results;
    let border_color = if is_focused {
        theme::BORDER_FOCUSED
    } else {
        theme::BORDER_UNFOCUSED
    };

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Results ({}) ", app.filtered_results().len()))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    f.render_widget(block, area);
    rects.results_panel = Some(inner);

    // ── Filter bar ──
    let bar_h = 1u16;
    let bar_y = inner.y;
    let bar_w = inner.width;
    let bar_right = inner.x + bar_w;

    // Fixed right-side buttons (laid out right-to-left)
    let quit_btn_w = 8u16;
    let clear_btn_w = 7u16;
    let all_btn_w = 5u16;
    let btn_gap = 1u16;

    // Quit button — far right
    let quit_x = bar_right.saturating_sub(quit_btn_w);
    let quit_rect = Rect::new(quit_x, bar_y, quit_btn_w, 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " ✕ Quit ",
            Style::default().fg(theme::FG).bg(theme::DANGER),
        )])),
        quit_rect,
    );
    rects.quit_button = Some(quit_rect);

    // Clear button — left of quit
    let clear_x = quit_x.saturating_sub(clear_btn_w + btn_gap);
    let clear_visible = clear_x > inner.x + 20; // only show if enough room
    if clear_visible {
        let r = Rect::new(clear_x, bar_y, clear_btn_w, 1);
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                " Clear ",
                Style::default().fg(theme::FG).bg(theme::BUTTON_BG),
            )])),
            r,
        );
        rects.filter_clear_btn = Some(r);
    } else {
        rects.filter_clear_btn = None;
    }

    // All button — left of clear
    let all_anchor = if clear_visible { clear_x } else { quit_x };
    let all_x = all_anchor.saturating_sub(all_btn_w + btn_gap);
    let all_visible = all_x > inner.x + 20;
    if all_visible {
        let r = Rect::new(all_x, bar_y, all_btn_w, 1);
        let all_active = app.filter.is_none();
        let all_style = if all_active {
            Style::default()
                .fg(theme::SELECTED_FG)
                .bg(theme::ACCENT)
                .bold()
        } else {
            Style::default().fg(theme::FG).bg(theme::BUTTON_BG)
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(" All ", all_style)])),
            r,
        );
        rects.filter_all_btn = Some(r);
    } else {
        rects.filter_all_btn = None;
    }

    // Remaining space available for filter input + type buttons
    let reserved_right = if all_visible {
        all_x
    } else if clear_visible {
        clear_x
    } else {
        quit_x
    };
    let available = reserved_right.saturating_sub(inner.x);

    // Filter input: take ~40% of available space, but at least 10 cols
    let input_w = ((available as f32 * 0.40) as u16).clamp(10, available.saturating_sub(4));
    let input_rect = Rect::new(inner.x, bar_y, input_w, 1);
    let input_bg = if app.filter_focused {
        Style::default().bg(theme::BUTTON_BG)
    } else {
        Style::default()
    };

    // Scroll filter text to keep cursor visible
    let filter_display = if app.filter_text.is_empty() {
        Span::styled("Filter path...", Style::default().fg(theme::FG_MUTED))
    } else {
        let vis_w = input_w.saturating_sub(1) as usize;
        let (visible, _) = scroll_text_to_cursor(&app.filter_text, app.filter_text_cursor, vis_w);
        Span::styled(visible, Style::default().fg(theme::FG))
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![filter_display])).style(input_bg),
        input_rect,
    );
    rects.filter_input = Some(input_rect);

    // File type filter buttons (left of reserved area, right of input)
    rects.filter_type_btns.clear();
    let mut bx = inner.x + input_w + 2;

    for ft in &[
        FileType::Word,
        FileType::Excel,
        FileType::Pdf,
        FileType::Text,
    ] {
        let label = ft.short_label();
        let is_active = app.filter.as_ref() == Some(ft);
        let bstyle = if is_active {
            Style::default()
                .fg(theme::SELECTED_FG)
                .bg(theme::ACCENT)
                .bold()
        } else {
            Style::default().fg(theme::FG).bg(theme::BUTTON_BG)
        };
        let btext = format!(" {} ", label);
        let bw = btext.len() as u16;

        // Stop if this button would overflow into reserved right area
        if bx + bw > reserved_right.saturating_sub(1) {
            break;
        }

        let btn_rect = Rect::new(bx, bar_y, bw, 1);
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(&btext, bstyle)])),
            btn_rect,
        );
        rects.filter_type_btns.push((ft.clone(), btn_rect));
        bx += bw + btn_gap;
    }

    // ── Results list ──
    let list_y = bar_y + bar_h + 1;
    let list_h = inner.height.saturating_sub(bar_h + 1);

    let filtered = app.filtered_results();
    rects.result_rows.clear();

    if filtered.is_empty() {
        let msg = if app.scanning {
            "Scanning..."
        } else if !app.results.is_empty() {
            "No results match the current filter."
        } else {
            "No results yet. Set keyword and directories, then click [Scan]."
        };
        f.render_widget(
            Paragraph::new(msg)
                .style(Style::default().fg(theme::FG_MUTED))
                .alignment(Alignment::Center),
            Rect::new(inner.x, list_y, inner.width, list_h),
        );
        return;
    }

    let visible = list_h as usize;
    let total = filtered.len();
    let sel = app.selected.min(total.saturating_sub(1));
    let start = sel
        .saturating_sub(visible.saturating_sub(1))
        .min(total.saturating_sub(visible));
    let start = start.min(sel);
    rects.result_list_start = start;

    let header = Row::new(vec![
        Cell::from("#"),
        Cell::from("Path"),
        Cell::from("Type"),
    ])
    .style(
        Style::default()
            .fg(theme::SELECTED_FG)
            .bg(theme::FG_DIM)
            .bold(),
    );

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(i, r)| {
            let style = if is_focused && i == sel {
                Style::default()
                    .fg(theme::SELECTED_FG)
                    .bg(theme::SELECTED_BG)
            } else if i % 2 == 0 {
                Style::default().bg(theme::BG_ROW_EVEN)
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

    let row_count = rows.len();
    let widths = [
        Constraint::Length(4),
        Constraint::Fill(1),
        Constraint::Length(10),
    ];

    let relative_sel = if is_focused {
        Some(sel.saturating_sub(start).min(row_count.saturating_sub(1)))
    } else {
        None
    };
    let mut table_state = TableState::default().with_selected(relative_sel);

    let table_w = inner.width.saturating_sub(1);
    let list_area = Rect::new(inner.x, list_y, table_w, list_h);
    f.render_stateful_widget(
        Table::new(rows, widths).header(header).column_spacing(1),
        list_area,
        &mut table_state,
    );

    let sb_area = Rect::new(inner.x + table_w, list_y, 1, list_h);
    render_scrollbar(f, sb_area, filtered.len(), visible, start);

    for i in 0..row_count {
        rects
            .result_rows
            .push(Rect::new(inner.x, list_y + i as u16 + 1, inner.width, 1));
    }
}

// ==================== Status bar ====================

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let raw = match app.mode {
        Mode::Command => format!(":{}", app.command),
        Mode::Normal => app.message.clone(),
        Mode::Browse => "Browsing directory...".to_string(),
    };
    // Clip to fit within the status bar area (using char count for safety)
    let max_w = area.width as usize;
    let text: String = if raw.chars().count() > max_w && max_w > 3 {
        raw.chars()
            .take(max_w.saturating_sub(1))
            .chain(['…'])
            .collect()
    } else {
        raw
    };
    f.render_widget(
        Paragraph::new(Span::styled(text, Style::default().fg(theme::FG_DIM))),
        area,
    );

    // Cursor for command mode
    if app.mode == Mode::Command {
        let cursor_pos = 1 + UnicodeWidthStr::width(&app.command[..app.command_cursor]) as u16;
        f.set_cursor_position((area.x + cursor_pos, area.y));
    }
}

// ==================== Browse popup ====================

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

fn render_browse_popup(f: &mut Frame, app: &App, area: Rect, rects: &mut UiRects) {
    let popup_area = centered_rect(65, 72, area);
    rects.browse_panel = Some(popup_area);

    f.render_widget(Clear, popup_area);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(" Select Directory ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let vchunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(inner);

    // Current path
    let cwd_display = app.browse_cwd.to_string_lossy().to_string();
    f.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(Span::styled(
                "Location:",
                Style::default().fg(theme::FG_MUTED),
            )),
            Line::from(Span::styled(
                format!("📁 {}", cwd_display),
                Style::default().fg(theme::WARNING),
            )),
        ])),
        vchunks[0],
    );

    // Directory listing
    rects.browse_rows.clear();

    let list_h = vchunks[1].height as usize;
    // Build full row list: current-dir entry + subdirs
    let mut all_rows: Vec<(String, bool, bool)> = Vec::new();
    // Index 0: current directory itself
    all_rows.push((format!("  📂 ."), true, false));
    for entry in &app.browse_entries {
        all_rows.push((
            format!("  📁 {}", entry.name),
            entry.is_dir,
            entry.name == "..",
        ));
    }

    let total = all_rows.len();
    let start = app.browse_scroll.min(total.saturating_sub(list_h));
    rects.browse_list_start = start;

    let rows: Vec<Row> = all_rows
        .iter()
        .enumerate()
        .skip(start)
        .take(list_h)
        .map(|(i, (label, _is_dir, _is_parent))| {
            let is_sel = i == app.browse_selected;
            let style = if is_sel {
                Style::default()
                    .fg(theme::SELECTED_FG)
                    .bg(theme::SELECTED_BG)
            } else if i % 2 == 0 {
                Style::default().bg(Color::Rgb(25, 25, 35))
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(label.as_str())]).style(style)
        })
        .collect();

    let row_count = rows.len();
    let table_w = vchunks[1].width.saturating_sub(1);
    let widths = [Constraint::Fill(1)];
    // Only highlight if selected item is in visible range
    let relative_sel = if app.browse_selected >= start && app.browse_selected < start + row_count {
        Some(app.browse_selected - start)
    } else {
        None
    };
    let mut table_state = TableState::default().with_selected(relative_sel);

    let list_area = Rect::new(vchunks[1].x, vchunks[1].y, table_w, vchunks[1].height);
    f.render_stateful_widget(
        Table::new(rows, widths).column_spacing(1),
        list_area,
        &mut table_state,
    );

    let sb_area = Rect::new(vchunks[1].x + table_w, vchunks[1].y, 1, vchunks[1].height);
    render_scrollbar(f, sb_area, total, list_h, start);

    for i in 0..row_count {
        rects.browse_rows.push(Rect::new(
            vchunks[1].x,
            vchunks[1].y + i as u16,
            vchunks[1].width,
            1,
        ));
    }

    // Footer: Confirm / Cancel buttons
    let footer = vchunks[2];
    let btn_w = 14u16;
    let total_w = btn_w * 2 + 2;
    let start_x = footer.x + (footer.width.saturating_sub(total_w)) / 2;
    let btn_y = footer.y + 1;

    let confirm_rect = Rect::new(start_x, btn_y, btn_w, 1);
    let cancel_rect = Rect::new(start_x + btn_w + 2, btn_y, btn_w, 1);

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  ✓ Confirm  ",
            Style::default()
                .fg(theme::SELECTED_FG)
                .bg(theme::SUCCESS)
                .bold(),
        )])),
        confirm_rect,
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  ✕ Cancel  ",
            Style::default().fg(theme::FG).bg(theme::DANGER),
        )])),
        cancel_rect,
    );

    rects.browse_confirm_btn = Some(confirm_rect);
    rects.browse_cancel_btn = Some(cancel_rect);
}

// ── Text scrolling helper ──

/// Given `text` and a byte cursor position, returns a substring that fits
/// within `vis_width` display columns, keeping the cursor in view, plus the
/// visual column offset where the cursor should be placed.
fn scroll_text_to_cursor(text: &str, cursor_byte: usize, vis_width: usize) -> (String, u16) {
    let cursor_byte = cursor_byte.min(text.len());
    let text_before_cursor = &text[..cursor_byte];
    let cursor_col = UnicodeWidthStr::width(text_before_cursor) as u16;
    let text_width = UnicodeWidthStr::width(text) as u16;

    // If everything fits, no scrolling needed
    if text_width <= vis_width as u16 {
        return (text.to_string(), cursor_col);
    }

    // We need to scroll. Determine the visible window.
    // Make the cursor column be somewhere in the right half of the visible area.
    let target_col = (vis_width as u16 * 3 / 4).min(cursor_col);
    let scroll_start_col = cursor_col.saturating_sub(target_col);

    // Find byte offset corresponding to scroll_start_col display columns
    let scroll_byte = byte_at_display_col(text, scroll_start_col as usize);
    // End: one past the last visible column
    let end_col = (scroll_start_col as usize + vis_width).min(text_width as usize);
    let end_byte = byte_at_display_col(text, end_col);

    let visible = text[scroll_byte..end_byte].to_string();
    let cursor_offset = cursor_col.saturating_sub(scroll_start_col);

    (visible, cursor_offset)
}

/// Find the byte offset in `text` that corresponds to `target_col` display columns.
fn byte_at_display_col(text: &str, target_col: usize) -> usize {
    let mut col = 0usize;
    for (i, c) in text.char_indices() {
        let s = &text[i..i + c.len_utf8()];
        let cw = UnicodeWidthStr::width(s);
        if col + cw > target_col {
            return i;
        }
        col += cw;
    }
    text.len()
}

// ── Scrollbar ──

fn render_scrollbar(f: &mut Frame, area: Rect, total: usize, visible: usize, offset: usize) {
    if total <= visible || area.height == 0 {
        return;
    }
    let h = area.height as usize;
    let thumb_start = ((offset as f64 / total as f64) * h as f64) as usize;
    let thumb_end = (((offset + visible) as f64 / total as f64) * h as f64) as usize;
    let thumb_end = thumb_end.max(thumb_start + 1).min(h);

    let lines: Vec<Line> = (0..h)
        .map(|i| {
            let ch = if i >= thumb_start && i < thumb_end {
                "█"
            } else {
                "│"
            };
            Line::from(Span::styled(ch, Style::default().fg(theme::FG_MUTED)))
        })
        .collect();

    f.render_widget(Paragraph::new(Text::from(lines)), area);
}
