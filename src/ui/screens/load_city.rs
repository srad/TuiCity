use crate::app::LoadCityState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};

pub fn render_load_city(frame: &mut Frame, area: Rect, state: &LoadCityState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 80, 120)))
        .title(" LOAD CITY ")
        .title_style(Style::default().fg(Color::Rgb(150, 180, 255)))
        .style(Style::default().bg(Color::Rgb(8, 8, 18)));
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
            .fg(Color::Rgb(140, 140, 180))
            .bg(Color::Rgb(8, 8, 18))
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
                    .fg(Color::Rgb(120, 120, 140))
                    .bg(Color::Rgb(8, 8, 18)),
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
                    .fg(Color::Black)
                    .bg(Color::Rgb(200, 180, 60))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Rgb(200, 200, 220))
                    .bg(Color::Rgb(8, 8, 18))
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
        Style::default()
            .fg(Color::Rgb(80, 80, 100))
            .bg(Color::Rgb(8, 8, 18)),
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
