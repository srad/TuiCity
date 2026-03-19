use crate::{
    app::{screens::LoadCityState, ClickArea},
    ui::{theme, view::LoadCityViewModel},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

const AUTHOR_LINE: &str = "by Saman Sedighi Rad";

pub fn render_load_city(
    frame: &mut Frame,
    area: Rect,
    view: &LoadCityViewModel,
    state: &mut LoadCityState,
) {
    let ui = theme::ui_palette();
    state.row_areas.clear();

    if area.width < 72 || area.height < 22 {
        render_compact_load_city(frame, area, view, state, &ui);
        return;
    }

    paint_background(frame.buffer_mut(), area);

    let title_y = area.y + 2;
    render_title(frame.buffer_mut(), area, title_y);

    let panel_w = area.width.saturating_sub(14).min(78).max(58);
    let panel_h = area.height.saturating_sub(14).min(18).max(12);
    let panel_x = area.x + area.width.saturating_sub(panel_w) / 2;
    let panel_y = title_y + 6;
    render_archive_panel(
        frame,
        Rect::new(panel_x, panel_y, panel_w, panel_h),
        view,
        state,
        &ui,
    );

    render_footer(frame.buffer_mut(), area);
}

fn render_compact_load_city(
    frame: &mut Frame,
    area: Rect,
    view: &LoadCityViewModel,
    state: &mut LoadCityState,
    ui: &theme::UiPalette,
) {
    paint_background(frame.buffer_mut(), area);

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
    set_centered_string(
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

    if view.saves.is_empty() {
        set_centered_string(
            buf,
            inner.x,
            inner.y + 2,
            inner.width,
            "No saved cities found.",
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(35, 34, 55)),
        );
        return;
    }

    for (i, entry) in view.saves.iter().enumerate() {
        let y = inner.y + 2 + i as u16;
        if y >= inner.y + inner.height.saturating_sub(1) {
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
            truncate(&entry.city_name, 18),
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
                truncate(&text, inner.width as usize),
                width = inner.width as usize
            ),
            style,
        );
    }
}

fn render_title(buf: &mut Buffer, area: Rect, y: u16) {
    set_centered_string(
        buf,
        area.x,
        y,
        area.width,
        "Load City",
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    );
    set_centered_string(
        buf,
        area.x,
        y + 1,
        area.width,
        "city archive",
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Reset),
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
        "CITY", "DATE", "POPULATION", "TREASURY"
    );
    buf.set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            truncate(&header, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Rgb(35, 34, 55))
            .add_modifier(Modifier::BOLD),
    );

    if view.saves.is_empty() {
        set_centered_string(
            buf,
            inner.x,
            inner.y + inner.height / 2,
            inner.width,
            "No saved cities found.",
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(35, 34, 55)),
        );
        return;
    }

    let list_top = inner.y + 2;
    let list_bottom = inner.y + inner.height.saturating_sub(1);
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
            "{:<24} {:<3} {:>4} {:>9} ${:>12}",
            truncate(&entry.city_name, 24),
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
            truncate(&line, inner.width as usize),
            width = inner.width as usize
        );
        buf.set_string(inner.x, row_y, padded, style);
    }
}

fn render_footer(buf: &mut Buffer, area: Rect) {
    if area.height < 3 {
        return;
    }

    set_centered_string(
        buf,
        area.x,
        area.y + area.height.saturating_sub(3),
        area.width,
        AUTHOR_LINE,
        Style::default()
            .fg(Color::Rgb(255, 205, 127))
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD),
    );
    set_centered_string(
        buf,
        area.x,
        area.y + area.height.saturating_sub(2),
        area.width,
        "Arrow Keys Select  •  Enter Load  •  Esc Back  •  Mouse Active",
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Reset),
    );
}

fn paint_background(buf: &mut Buffer, area: Rect) {
    let horizon_y = area.y + area.height.saturating_mul(2) / 3;
    let sun_x = area.x + area.width / 2;
    let sun_y = area.y + area.height / 4;

    for y in area.y..area.y + area.height {
        let t = (y - area.y) as f32 / area.height.max(1) as f32;
        let base = if t < 0.35 {
            lerp_color((35, 44, 100), (84, 61, 135), t / 0.35)
        } else if t < 0.70 {
            lerp_color((84, 61, 135), (214, 99, 110), (t - 0.35) / 0.35)
        } else {
            lerp_color((214, 99, 110), (255, 170, 91), (t - 0.70) / 0.30)
        };

        for x in area.x..area.x + area.width {
            let dx = x as i32 - sun_x as i32;
            let dy = y as i32 - sun_y as i32;
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            let glow = (1.0 - dist / (area.width.max(area.height) as f32 * 0.45)).clamp(0.0, 1.0);
            let color = blend_color(base, Color::Rgb(255, 224, 138), glow * 0.45);
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ").set_fg(color).set_bg(color);
            }
        }
    }

    paint_sun(buf, area, sun_x, sun_y, horizon_y);
    paint_clouds(buf, area);
    paint_grid(buf, area, horizon_y);
    paint_skyline(buf, area, horizon_y);
    paint_scanlines(buf, area);
}

fn paint_sun(buf: &mut Buffer, area: Rect, center_x: u16, center_y: u16, horizon_y: u16) {
    let radius_x = (area.width / 7).max(8);
    let radius_y = (area.height / 7).max(4);

    for y in center_y.saturating_sub(radius_y)..=(center_y + radius_y).min(horizon_y) {
        let dy = y as i32 - center_y as i32;
        let ny = dy as f32 / radius_y.max(1) as f32;
        for x in
            center_x.saturating_sub(radius_x)..=(center_x + radius_x).min(area.x + area.width - 1)
        {
            let dx = x as i32 - center_x as i32;
            let nx = dx as f32 / radius_x.max(1) as f32;
            let dist = nx * nx + ny * ny;
            if dist > 1.0 {
                continue;
            }
            if dy > 0 && dy % 2 != 0 {
                continue;
            }
            let color = lerp_color((255, 191, 98), (255, 237, 166), 1.0 - dist.min(1.0));
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ").set_fg(color).set_bg(color);
            }
        }
    }
}

fn paint_clouds(buf: &mut Buffer, area: Rect) {
    for y in area.y + 2..area.y + area.height / 2 {
        for x in area.x..area.x + area.width {
            let seed = hash_point(x, y);
            if seed % 97 == 0 || seed % 131 == 0 {
                let length = 3 + (seed % 7) as u16;
                for dx in 0..length {
                    let px = x + dx;
                    if px >= area.x + area.width {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((px, y)) {
                        cell.set_symbol("~").set_fg(Color::Rgb(255, 196, 185));
                    }
                }
            }
        }
    }
}

fn paint_grid(buf: &mut Buffer, area: Rect, horizon_y: u16) {
    let center_x = area.x + area.width / 2;
    let bottom_y = area.y + area.height - 1;

    for i in 0..6 {
        let t = i as f32 / 5.0;
        let y = horizon_y + ((bottom_y - horizon_y) as f32 * t * t).round() as u16;
        if y >= area.y + area.height {
            continue;
        }
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol("─").set_fg(Color::Rgb(255, 131, 110));
            }
        }
    }

    for offset in (0..=area.width).step_by(6) {
        let base_x = area.x + offset;
        for y in horizon_y..=bottom_y {
            let t = (y - horizon_y) as f32 / (bottom_y - horizon_y).max(1) as f32;
            let px = center_x as f32 + (base_x as f32 - center_x as f32) * t.powf(1.1);
            let x = px.round() as i32;
            if x < area.x as i32 || x >= (area.x + area.width) as i32 {
                continue;
            }
            if let Some(cell) = buf.cell_mut((x as u16, y)) {
                let symbol = if x as u16 >= center_x { "╱" } else { "╲" };
                cell.set_symbol(symbol).set_fg(Color::Rgb(115, 240, 232));
            }
        }
    }
}

fn paint_skyline(buf: &mut Buffer, area: Rect, base_y: u16) {
    let mut x = area.x;
    while x < area.x + area.width {
        let seed = hash_point(x, base_y);
        let width = 4 + (seed % 8) as u16;
        let height = 3 + ((seed / 13) % 7) as u16;
        let right = (x + width).min(area.x + area.width);
        let top = base_y.saturating_sub(height);

        for by in top..base_y {
            for bx in x..right {
                if let Some(cell) = buf.cell_mut((bx, by)) {
                    cell.set_symbol(" ")
                        .set_bg(Color::Rgb(22, 20, 34))
                        .set_fg(Color::Rgb(22, 20, 34));
                }
                let lit = bx > x
                    && bx + 1 < right
                    && by > top
                    && (bx - x) % 2 == 1
                    && (base_y - by) % 2 == 0
                    && hash_point(bx, by) % 3 == 0;
                if lit {
                    let color = if hash_point(bx + 1, by) % 2 == 0 {
                        Color::Rgb(255, 221, 119)
                    } else {
                        Color::Rgb(106, 226, 225)
                    };
                    if let Some(cell) = buf.cell_mut((bx, by)) {
                        cell.set_symbol("▪").set_fg(color);
                    }
                }
            }
        }

        x = right.saturating_add(1);
    }
}

fn paint_scanlines(buf: &mut Buffer, area: Rect) {
    for y in area.y..area.y + area.height {
        if (y - area.y) % 2 == 1 {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(darken(cell.bg, 0.93));
                }
            }
        }
    }
}

fn set_centered_string(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    let display = truncate(text, width as usize);
    let display_w = display.chars().count() as u16;
    let start_x = x + width.saturating_sub(display_w) / 2;
    buf.set_string(start_x, y, display, style);
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

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::Rgb(
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t).round() as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t).round() as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t).round() as u8,
    )
}

fn blend_color(base: Color, accent: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    match (base, accent) {
        (Color::Rgb(br, bg, bb), Color::Rgb(ar, ag, ab)) => Color::Rgb(
            (br as f32 + (ar as f32 - br as f32) * amount).round() as u8,
            (bg as f32 + (ag as f32 - bg as f32) * amount).round() as u8,
            (bb as f32 + (ab as f32 - bb as f32) * amount).round() as u8,
        ),
        _ => base,
    }
}

fn darken(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor).round() as u8,
            (g as f32 * factor).round() as u8,
            (b as f32 * factor).round() as u8,
        ),
        other => other,
    }
}

fn hash_point(x: u16, y: u16) -> u32 {
    let mut value = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
    value ^= value >> 13;
    value = value.wrapping_mul(1_274_126_177);
    value ^ (value >> 16)
}
