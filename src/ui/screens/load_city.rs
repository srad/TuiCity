use crate::app::screens::LoadCityState;
use crate::ui::theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};

pub fn render_load_city(frame: &mut Frame, area: Rect, state: &LoadCityState) {
    let ui = theme::ui_palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui.window_border))
        .title(" LOAD CITY ")
        .title_style(Style::default().fg(ui.window_title))
        .style(Style::default().bg(ui.window_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let buf = frame.buffer_mut();
    let w = inner.width as usize;
    let mut row = inner.y;

    // Header
    let header = format!(
        "{:<24} {:>6}  {:>8}  {:>8}",
        "City Name", "Date", "Pop", "Treasury"
    );
    buf.set_string(
        inner.x,
        row,
        truncate(&header, w),
        Style::default()
            .fg(ui.text_muted)
            .bg(ui.window_bg)
            .add_modifier(Modifier::UNDERLINED),
    );
    row += 1;

    if state.saves.is_empty() {
        if row < inner.y + inner.height {
            buf.set_string(
                inner.x,
                row + 1,
                "  No saved cities found.",
                Style::default()
                    .fg(ui.text_muted)
                    .bg(ui.window_bg),
            );
        }
    } else {
        for (i, entry) in state.saves.iter().enumerate() {
            if row >= inner.y + inner.height - 3 {
                break;
            }
            let is_sel = i == state.selected;
            let prefix = if is_sel { "▶ " } else { "  " };
            let line = format!(
                "{}{:<22} {} {:>4}  {:>8}  ${:>8}",
                prefix,
                truncate(&entry.city_name, 22),
                month_name(entry.month),
                entry.year,
                fmt_number(entry.population as i64),
                fmt_number(entry.treasury),
            );
            let style = if is_sel {
                Style::default()
                    .fg(ui.selection_fg)
                    .bg(ui.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui.text_primary).bg(ui.window_bg)
            };
            buf.set_string(inner.x, row, truncate(&line, w), style);
            row += 1;
        }
    }

    // Controls
    let hint = "↑↓ Select   Enter Load   Esc Back";
    let hint_y = inner.y + inner.height - 1;
    buf.set_string(
        inner.x,
        hint_y,
        truncate(hint, w),
        Style::default().fg(ui.text_dim).bg(ui.window_bg),
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn fmt_number(n: i64) -> String {
    if n < 0 {
        return format!("-{}", fmt_number(-n));
    }
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
