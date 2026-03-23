use super::common::{self, SynthwaveBackground};
use crate::{
    app::{screens::LoadCityState, ClickArea},
    ui::frontends::terminal::render_confirm_dialog,
    ui::{theme, view::LoadCityViewModel},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Widget},
    Frame,
};
use tui_big_text::{BigText, PixelSize};

const BIG_TITLE_HEIGHT: u16 = 4;

pub fn render_load_city(
    frame: &mut Frame,
    area: Rect,
    view: &LoadCityViewModel,
    state: &mut LoadCityState,
) {
    let ui = theme::ui_palette();
    state.row_areas.clear();
    state.dialog_items.clear();

    if area.width < 72 || area.height < 22 {
        render_compact_load_city(frame, area, view, state, &ui);
        return;
    }

    common::paint_synthwave(
        frame.buffer_mut(),
        area,
        SynthwaveBackground {
            sun: true,
            clouds: true,
            lit_windows: true,
        },
    );

    let title_y = area.y + 2;
    render_title(frame.buffer_mut(), area, title_y, &ui);

    let panel_w = area.width.saturating_sub(14).clamp(58, 78);
    let panel_h = area.height.saturating_sub(14).clamp(12, 18);
    let panel_x = area.x + area.width.saturating_sub(panel_w) / 2;
    let panel_y = title_y + 6;
    render_archive_panel(
        frame,
        Rect::new(panel_x, panel_y, panel_w, panel_h),
        view,
        state,
        &ui,
    );

    common::render_footer(
        frame.buffer_mut(),
        area,
        "Arrow Keys Select  •  Enter Load  •  D Delete  •  Esc Back  •  Mouse Active",
    );

    if let Some(dialog) = &view.confirm_dialog {
        state.dialog_items = render_confirm_dialog(frame, area, dialog);
    }
}

fn render_compact_load_city(
    frame: &mut Frame,
    area: Rect,
    view: &LoadCityViewModel,
    state: &mut LoadCityState,
    ui: &theme::UiPalette,
) {
    common::paint_synthwave(
        frame.buffer_mut(),
        area,
        SynthwaveBackground {
            sun: true,
            clouds: true,
            lit_windows: true,
        },
    );

    let rect = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" CITY ARCHIVE ")
            .title_style(
                Style::default()
                    .fg(Color::Rgb(255, 221, 119))
                    .bg(Color::Rgb(35, 34, 55))
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(Color::Rgb(106, 226, 225)))
            .style(Style::default().bg(Color::Rgb(35, 34, 55))),
        rect,
    );

    let inner = Rect::new(
        rect.x + 1,
        rect.y + 1,
        rect.width.saturating_sub(2),
        rect.height.saturating_sub(2),
    );
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let buf = frame.buffer_mut();
    common::set_centered_string(
        buf,
        inner.x,
        inner.y,
        inner.width,
        "Load City",
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .bg(Color::Rgb(35, 34, 55))
            .add_modifier(Modifier::BOLD),
    );

    if view.is_loading && view.saves.is_empty() {
        render_loading_state(
            buf,
            inner,
            ui,
            view.loading_indicator,
            Color::Rgb(35, 34, 55),
        );
        return;
    }

    if view.saves.is_empty() {
        common::set_centered_string(
            buf,
            inner.x,
            inner.y + 2,
            inner.width,
            "No saved cities found.",
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(35, 34, 55)),
        );
        {
            let back_y = inner.y + inner.height.saturating_sub(4);
            if back_y > inner.y {
                let area = common::render_back_button(
                    buf,
                    inner,
                    back_y,
                    view.selected >= view.saves.len(),
                );
                state.row_areas.push(area);
            }
        }
        return;
    }

    for (i, entry) in view.saves.iter().enumerate() {
        let y = inner.y + 2 + i as u16;
        if y >= inner.y + inner.height.saturating_sub(2) {
            break;
        }
        state.row_areas.push(ClickArea {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        });
        let selected = i == view.selected;
        let text = format!(
            "{}  {} {}  ${}",
            if selected { ">" } else { " " },
            common::truncate(&entry.city_name, 18),
            entry.year,
            fmt_number(entry.treasury)
        );
        let style = if selected {
            Style::default()
                .fg(Color::Rgb(28, 28, 42))
                .bg(Color::Rgb(255, 221, 119))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(35, 34, 55))
        };
        buf.set_string(
            inner.x,
            y,
            format!(
                "{:<width$}",
                common::truncate(&text, inner.width as usize),
                width = inner.width as usize
            ),
            style,
        );
    }

    if view.is_loading {
        render_scan_status(
            buf,
            inner.x,
            inner.y + inner.height.saturating_sub(2),
            inner.width,
            view.loading_indicator,
            Color::Rgb(35, 34, 55),
        );
    }

    {
        let back_y = inner.y + inner.height.saturating_sub(4);
        if back_y > inner.y {
            let area =
                common::render_back_button(buf, inner, back_y, view.selected >= view.saves.len());
            state.row_areas.push(area);
        }
    }

    if let Some(dialog) = &view.confirm_dialog {
        state.dialog_items = render_confirm_dialog(frame, area, dialog);
    }
}

fn render_title(buf: &mut Buffer, area: Rect, y: u16, ui: &theme::UiPalette) {
    BigText::builder()
        .pixel_size(PixelSize::Quadrant)
        .centered()
        .lines(vec![Line::styled(
            "LOAD CITY",
            Style::default()
                .fg(ui.title)
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD),
        )])
        .build()
        .render(Rect::new(area.x, y, area.width, BIG_TITLE_HEIGHT), buf);

    common::set_centered_string(
        buf,
        area.x,
        y + BIG_TITLE_HEIGHT,
        area.width,
        "city archive",
        Style::default().fg(ui.subtitle).bg(Color::Reset),
    );
}

fn render_archive_panel(
    frame: &mut Frame,
    rect: Rect,
    view: &LoadCityViewModel,
    state: &mut LoadCityState,
    ui: &theme::UiPalette,
) {
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" CITY ARCHIVE ")
            .title_style(
                Style::default()
                    .fg(Color::Rgb(255, 221, 119))
                    .bg(Color::Rgb(35, 34, 55))
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(Color::Rgb(106, 226, 225)))
            .style(Style::default().bg(Color::Rgb(35, 34, 55))),
        rect,
    );

    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let buf = frame.buffer_mut();
    let header = format!(
        "{:<24} {:<10} {:>9} {:>13}",
        "CITY", "DATE", "POPULATION", "$ TREASURY"
    );
    buf.set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            common::truncate(&header, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Rgb(35, 34, 55))
            .add_modifier(Modifier::BOLD),
    );

    if view.is_loading && view.saves.is_empty() {
        render_loading_state(
            buf,
            Rect::new(
                inner.x,
                inner.y + 2,
                inner.width,
                inner.height.saturating_sub(2),
            ),
            ui,
            view.loading_indicator,
            Color::Rgb(35, 34, 55),
        );
        return;
    }

    if view.saves.is_empty() {
        common::set_centered_string(
            buf,
            inner.x,
            inner.y + 2,
            inner.width,
            "No saved cities found.",
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(35, 34, 55)),
        );
        // Back button even when empty
        {
            let back_y = inner.y + inner.height.saturating_sub(4);
            if back_y > inner.y {
                let area = common::render_back_button(
                    buf,
                    inner,
                    back_y,
                    view.selected >= view.saves.len(),
                );
                state.row_areas.push(area);
            }
        }
        return;
    }

    let list_top = inner.y + 2;
    let list_bottom = inner.y + inner.height.saturating_sub(2); // leave room for back
    for (i, entry) in view.saves.iter().enumerate() {
        let row_y = list_top + i as u16;
        if row_y >= list_bottom {
            break;
        }
        state.row_areas.push(ClickArea {
            x: inner.x,
            y: row_y,
            width: inner.width,
            height: 1,
        });

        let line = format!(
            "{:<24} {:<3} {:>4} {:>9} {:>13}",
            common::truncate(&entry.city_name, 24),
            month_name(entry.month),
            entry.year,
            fmt_number(entry.population as i64),
            fmt_number(entry.treasury),
        );
        let selected = i == view.selected;
        let style = if selected {
            Style::default()
                .fg(Color::Rgb(28, 28, 42))
                .bg(Color::Rgb(255, 221, 119))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(56, 42, 78))
        };
        let padded = format!(
            "{:<width$}",
            common::truncate(&line, inner.width as usize),
            width = inner.width as usize
        );
        buf.set_string(inner.x, row_y, padded, style);
    }

    if view.is_loading {
        render_scan_status(
            buf,
            inner.x,
            list_bottom,
            inner.width,
            view.loading_indicator,
            Color::Rgb(35, 34, 55),
        );
    }

    {
        let back_y = inner.y + inner.height.saturating_sub(4);
        if back_y > inner.y {
            let area =
                common::render_back_button(buf, inner, back_y, view.selected >= view.saves.len());
            state.row_areas.push(area);
        }
    }
}

fn render_loading_state(
    buf: &mut Buffer,
    area: Rect,
    ui: &theme::UiPalette,
    indicator: &str,
    bg: Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mid_y = area.y + area.height / 2;
    common::set_centered_string(
        buf,
        area.x,
        mid_y.saturating_sub(1),
        area.width,
        indicator,
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    );
    common::set_centered_string(
        buf,
        area.x,
        mid_y,
        area.width,
        "Loading saved cities...",
        Style::default().fg(ui.text_primary).bg(bg),
    );
    common::set_centered_string(
        buf,
        area.x,
        mid_y.saturating_add(1),
        area.width,
        "Large archives may take a moment.",
        Style::default().fg(Color::Rgb(170, 223, 219)).bg(bg),
    );
}

fn render_scan_status(buf: &mut Buffer, x: u16, y: u16, width: u16, indicator: &str, bg: Color) {
    if width == 0 {
        return;
    }

    let text = format!("{indicator} scanning archive...");
    buf.set_string(
        x,
        y,
        format!(
            "{:<width$}",
            common::truncate(&text, width as usize),
            width = width as usize
        ),
        Style::default().fg(Color::Rgb(170, 223, 219)).bg(bg),
    );
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
