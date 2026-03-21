use crate::{
    app::{screens::SettingsState, ClickArea},
    ui::view::SettingsViewModel,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

const AUTHOR_LINE: &str = "by Saman Sedighi Rad";

pub fn render_settings(
    frame: &mut Frame,
    area: Rect,
    view: &SettingsViewModel,
    state: &mut SettingsState,
) {
    state.row_areas.clear();
    paint_background(frame.buffer_mut(), area);

    let panel_w = area.width.saturating_sub(18).clamp(48, 66);
    let available_h = area.height.saturating_sub(8).max(10);
    let panel_h = available_h.min(18);
    let panel_x = area.x + area.width.saturating_sub(panel_w) / 2;
    let panel_y = area.y + 4;

    render_title(frame.buffer_mut(), area, area.y + 1, view);
    render_panel(
        frame,
        Rect::new(panel_x, panel_y, panel_w, panel_h),
        view,
        state,
    );
    render_footer(frame.buffer_mut(), area);
}

fn render_title(buf: &mut Buffer, area: Rect, y: u16, view: &SettingsViewModel) {
    set_centered_string(
        buf,
        area.x,
        y,
        area.width,
        "Settings",
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
        &format!("Current Theme: {}", view.current_theme_label),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Reset),
    );
}

fn render_panel(
    frame: &mut Frame,
    rect: Rect,
    view: &SettingsViewModel,
    state: &mut SettingsState,
) {
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" OPTIONS ")
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
    let hint = "Open the palette browser or quick-cycle through themes.";
    buf.set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            truncate(hint, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Rgb(35, 34, 55)),
    );

    for (idx, option) in view.options.iter().enumerate() {
        let row_y = inner.y + 2 + idx as u16 * 4;
        if row_y + 2 >= inner.y + inner.height {
            break;
        }

        state.row_areas.push(ClickArea {
            x: inner.x,
            y: row_y,
            width: inner.width,
            height: 3,
        });

        let selected = idx == view.selected;
        let row_style = if selected {
            Style::default()
                .fg(Color::Rgb(28, 28, 42))
                .bg(Color::Rgb(255, 221, 119))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Rgb(238, 232, 225))
                .bg(Color::Rgb(56, 42, 78))
        };

        let blank = format!("{:^width$}", " ", width = inner.width as usize);
        buf.set_string(inner.x, row_y, blank.clone(), row_style);
        buf.set_string(
            inner.x,
            row_y + 1,
            format!(
                "{:^width$}",
                truncate(option, inner.width.saturating_sub(4) as usize),
                width = inner.width as usize
            ),
            row_style,
        );
        buf.set_string(inner.x, row_y + 2, blank, row_style);
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
        "Arrow Keys Move  •  Enter Select  •  Esc Back  •  Shift+P Cycle",
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

    paint_grid(buf, area, horizon_y);
    paint_skyline(buf, area, horizon_y);
    paint_scanlines(buf, area);
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
        let left_x = center_x.saturating_sub(offset);
        let right_x = center_x
            .saturating_add(offset)
            .min(area.x + area.width.saturating_sub(1));
        let steps = bottom_y.saturating_sub(horizon_y).max(1);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let y = horizon_y + step;
            let left = center_x as f32 + (left_x as f32 - center_x as f32) * t;
            let right = center_x as f32 + (right_x as f32 - center_x as f32) * t;
            if let Some(cell) = buf.cell_mut((left.round() as u16, y)) {
                cell.set_symbol("╱").set_fg(Color::Rgb(255, 131, 110));
            }
            if let Some(cell) = buf.cell_mut((right.round() as u16, y)) {
                cell.set_symbol("╲").set_fg(Color::Rgb(255, 131, 110));
            }
        }
    }
}

fn paint_skyline(buf: &mut Buffer, area: Rect, base_y: u16) {
    let mut x = area.x;
    let mut idx = 0u16;
    while x < area.x + area.width {
        let width = 4 + (idx % 5);
        let height = 3 + (idx * 3 % 8);
        let top = base_y.saturating_sub(height);
        let building_color = if idx.is_multiple_of(2) {
            Color::Rgb(47, 35, 66)
        } else {
            Color::Rgb(63, 43, 85)
        };
        for bx in x..(x + width).min(area.x + area.width) {
            for by in top..base_y {
                if let Some(cell) = buf.cell_mut((bx, by)) {
                    cell.set_symbol(" ")
                        .set_fg(building_color)
                        .set_bg(building_color);
                }
            }
        }
        x = x.saturating_add(width + 1);
        idx = idx.saturating_add(1);
    }
}

fn paint_scanlines(buf: &mut Buffer, area: Rect) {
    for y in (area.y..area.y + area.height).step_by(2) {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_bg(dim_color(cell.bg, 0.94));
            }
        }
    }
}

fn set_centered_string(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    let content = truncate(text, width as usize);
    let start_x = x + width.saturating_sub(content.chars().count() as u16) / 2;
    buf.set_string(start_x, y, content, style);
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::Rgb(
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t).round() as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t).round() as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t).round() as u8,
    )
}

fn blend_color(base: Color, glow: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    let (br, bg, bb) = rgb_components(base);
    let (gr, gg, gb) = rgb_components(glow);
    Color::Rgb(
        (br as f32 + (gr as f32 - br as f32) * amount).round() as u8,
        (bg as f32 + (gg as f32 - bg as f32) * amount).round() as u8,
        (bb as f32 + (gb as f32 - bb as f32) * amount).round() as u8,
    )
}

fn dim_color(color: Color, factor: f32) -> Color {
    let (r, g, b) = rgb_components(color);
    Color::Rgb(
        (r as f32 * factor).round() as u8,
        (g as f32 * factor).round() as u8,
        (b as f32 * factor).round() as u8,
    )
}

fn rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Reset => (0, 0, 0),
        _ => (0, 0, 0),
    }
}
