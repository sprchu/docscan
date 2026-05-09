use std::sync::mpsc;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::input::ScanMessage;
use super::state::App;
use super::types::{Focus, Mode, UiRects};

/// Handle a mouse event, dispatching based on current mode and hit regions.
/// The `UiRects` must have been populated by the most recent render.
pub fn handle_mouse(
    app: &mut App,
    mouse: MouseEvent,
    scan_tx: &mpsc::Sender<ScanMessage>,
    rects: &UiRects,
) {
    let (col, row) = (mouse.column, mouse.row);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(app, col, row, scan_tx, rects);
        }
        MouseEventKind::ScrollDown => {
            handle_scroll_down(app, col, row, rects);
        }
        MouseEventKind::ScrollUp => {
            handle_scroll_up(app, col, row, rects);
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════
//  Left click — dispatched by mode, then by hit region
// ═══════════════════════════════════════════════════════════════

fn handle_left_click(
    app: &mut App,
    col: u16,
    row: u16,
    scan_tx: &mpsc::Sender<ScanMessage>,
    rects: &UiRects,
) {
    // ── Browse popup (modal — takes priority) ──
    if app.mode == Mode::Browse {
        handle_browse_click(app, col, row, rects);
        return;
    }

    // ── Quit button ──
    if hit_opt(&rects.quit_button, col, row) {
        app.should_quit = true;
        return;
    }

    // ── Keyword input ──
    if hit_opt(&rects.keyword_input, col, row) {
        app.focus = Focus::ConfigLeft;
        app.config_left_row = 0;
        return;
    }

    // ── Scan button ──
    if hit_opt(&rects.scan_button, col, row) {
        start_scan(app, scan_tx);
        return;
    }

    // ── Threads dec / inc ──
    if hit_opt(&rects.threads_dec_btn, col, row) {
        app.threads_dec();
        app.focus = Focus::ConfigLeft;
        app.config_left_row = 1;
        return;
    }
    if hit_opt(&rects.threads_inc_btn, col, row) {
        app.threads_inc();
        app.focus = Focus::ConfigLeft;
        app.config_left_row = 1;
        return;
    }

    // ── Type toggle buttons ──
    for (i, r) in rects.type_btns.iter().enumerate() {
        if hit(r, col, row) {
            app.toggle_file_type(i);
            app.focus = Focus::ConfigLeft;
            app.config_left_row = 2;
            app.ft_cursor = i;
            return;
        }
    }

    // ── Path Add / Edit / Delete buttons ──
    if hit_opt(&rects.path_add_btn, col, row) {
        app.add_new_path_via_browse();
        return;
    }
    if hit_opt(&rects.path_edit_btn, col, row) {
        app.edit_path_via_browse();
        return;
    }
    if hit_opt(&rects.path_del_btn, col, row) {
        app.delete_selected_path();
        return;
    }

    // ── Path rows ──
    for (i, r) in rects.path_rows.iter().enumerate() {
        if hit(r, col, row) {
            app.focus = Focus::ConfigRight;
            app.dir_selected = rects.path_list_start + i;
            return;
        }
    }
    // Click in paths panel area (but not on a row) → focus
    if hit_opt(&rects.paths_panel, col, row) {
        app.focus = Focus::ConfigRight;
        return;
    }

    // ── Filter input ──
    if hit_opt(&rects.filter_input, col, row) {
        app.focus = Focus::Results;
        app.filter_focused = true;
        return;
    }

    // ── Filter type buttons ──
    for (ft, r) in &rects.filter_type_btns {
        if hit(r, col, row) {
            app.set_filter_type(Some(ft.clone()));
            return;
        }
    }
    if hit_opt(&rects.filter_all_btn, col, row) {
        app.set_filter_type(None);
        return;
    }
    if hit_opt(&rects.filter_clear_btn, col, row) {
        app.filter_text.clear();
        app.filter_text_cursor = 0;
        app.filter_focused = false;
        return;
    }

    // ── Result rows ──
    for (i, r) in rects.result_rows.iter().enumerate() {
        if hit(r, col, row) {
            app.focus = Focus::Results;
            app.filter_focused = false;
            app.selected = rects.result_list_start + i;
            return;
        }
    }
    // Click in results panel area → focus
    if hit_opt(&rects.results_panel, col, row) {
        app.focus = Focus::Results;
        return;
    }

    // Click elsewhere → focus keyword
    app.focus = Focus::ConfigLeft;
    app.config_left_row = 0;
}

// ── Browse popup clicks ──

fn handle_browse_click(app: &mut App, col: u16, row: u16, rects: &UiRects) {
    // Confirm / Cancel buttons
    if hit_opt(&rects.browse_confirm_btn, col, row) {
        app.select_browse_dir();
        return;
    }
    if hit_opt(&rects.browse_cancel_btn, col, row) {
        app.exit_browse_mode();
        return;
    }

    // Click on a browse row — single=select, double=enter
    for (i, r) in rects.browse_rows.iter().enumerate() {
        if hit(r, col, row) {
            let idx = rects.browse_list_start + i;
            let now = std::time::Instant::now();
            let is_double = app
                .last_browse_click
                .map_or(false, |(prev_idx, prev_time)| {
                    prev_idx == idx && now.duration_since(prev_time).as_millis() < 400
                });
            if is_double {
                app.browse_selected = idx;
                app.browse_enter();
                app.last_browse_click = None;
            } else {
                app.browse_selected = idx;
                app.last_browse_click = Some((idx, now));
            }
            return;
        }
    }

    // Click on browse panel but not on rows → ignore
    if hit_opt(&rects.browse_panel, col, row) {
        return;
    }

    // Click outside browse popup → cancel
    app.exit_browse_mode();
}

// ═══════════════════════════════════════════════════════════════
//  Scroll
// ═══════════════════════════════════════════════════════════════

fn handle_scroll_down(app: &mut App, col: u16, row: u16, rects: &UiRects) {
    // Browse popup has priority when open (modal)
    if app.mode == Mode::Browse {
        if hit_opt(&rects.browse_panel, col, row) {
            app.browse_down();
            return;
        }
    }
    if hit_opt(&rects.paths_panel, col, row) {
        app.focus = Focus::ConfigRight;
        app.dir_selected_down();
        return;
    }
    if hit_opt(&rects.results_panel, col, row) {
        app.focus = Focus::Results;
        app.selected_down();
    }
}

fn handle_scroll_up(app: &mut App, col: u16, row: u16, rects: &UiRects) {
    if app.mode == Mode::Browse {
        if hit_opt(&rects.browse_panel, col, row) {
            app.browse_up();
            return;
        }
    }
    if hit_opt(&rects.paths_panel, col, row) {
        app.focus = Focus::ConfigRight;
        app.dir_selected_up();
        return;
    }
    if hit_opt(&rects.results_panel, col, row) {
        app.focus = Focus::Results;
        app.selected_up();
    }
}

// ═══════════════════════════════════════════════════════════════
//  Hit-test helpers
// ═══════════════════════════════════════════════════════════════

fn hit(rect: &Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

fn hit_opt(opt: &Option<Rect>, col: u16, row: u16) -> bool {
    opt.as_ref().map_or(false, |r| hit(r, col, row))
}

/// Re-export for convenience — delegates to `start_scan` in the input module
fn start_scan(app: &mut App, tx: &mpsc::Sender<ScanMessage>) {
    // Trigger scan (uses same path as keyboard-initiated scans)
    super::input::start_scan(app, tx);
}
