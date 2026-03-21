use crate::{
    app::{screens::ThemeSettingsState, ClickArea},
    ui::{theme, view::ThemeSettingsViewModel},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

const AUTHOR_LINE: &str = "by Saman Sedighi Rad";

pub fn render_theme_settings(
    frame: &mut Frame,
    area: Rect,
    view: &ThemeSettingsViewModel,
    state: &mut ThemeSettingsState,
) {
    state.row_areas.clear();
    paint_background(frame.buffer_mut(), area);

    let panel_w = area.width.saturating_sub(18).clamp(54, 72);
    let content_h = 5 + view.themes.len() as u16 * 3;
    let available_h = area.height.saturating_sub(6).max(8);
    let panel_h = content_h.min(available_h).max(available_h.min(16));
    let panel_x = area.x + area.width.saturating_sub(panel_w) / 2;
    let panel_y = area.y + 3;

    render_title(frame.buffer_mut(), area, area.y + 1);
    render_panel(
        frame,
        Rect::new(panel_x, panel_y, panel_w, panel_h),
        view,
        state,
    );
    render_footer(frame.buffer_mut(), area);
}

fn render_title(buf: &mut Buffer, area: Rect, y: u16) {
    set_centered_string(
        buf,
        area.x,
        y,
        area.width,
        "Theme Settings",
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
        "preview palettes live",
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Reset),
    );
}

fn render_panel(
    frame: &mut Frame,
    rect: Rect,
    view: &ThemeSettingsViewModel,
    state: &mut ThemeSettingsState,
) {
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" PALETTE LAB ")
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

    let buf = frame.buffer_mut();
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let hint = format!("Current: {}", view.active.label());
    buf.set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            truncate(&hint, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(Color::Rgb(170, 223, 219))
            .bg(Color::Rgb(35, 34, 55))
            .add_modifier(Modifier::BOLD),
    );

    for (idx, preset) in view.themes.iter().copied().enumerate() {
        let row_y = inner.y + 2 + idx as u16 * 3;
        if row_y + 1 >= inner.y + inner.height {
            break;
        }
        state.row_areas.push(ClickArea {
            x: inner.x,
            y: row_y,
            width: inner.width,
            height: 2,
        });

        let selected = idx == view.selected;
        let palette = theme::palette_for(preset);
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
        buf.set_string(inner.x, row_y + 1, blank, row_style);

        let name = if preset == view.active {
            format!("{}  (ACTIVE)", preset.label())
        } else {
            preset.label().to_string()
        };
        buf.set_string(
            inner.x + 2,
            row_y,
            truncate(&name, inner.width.saturating_sub(4) as usize),
            row_style,
        );

        render_swatch(
            buf,
            inner.x + 2,
            row_y + 1,
            palette.title,
            palette.selection_bg,
            palette.accent,
            palette.info,
            palette.success,
            palette.warning,
            row_style.bg.unwrap_or(Color::Rgb(56, 42, 78)),
        );
    }
}

fn render_swatch(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    a: Color,
    b: Color,
    c: Color,
    d: Color,
    e: Color,
    f: Color,
    bg: Color,
) {
    let colors = [a, b, c, d, e, f];
    for (idx, color) in colors.iter().enumerate() {
        buf.set_string(
            x + idx as u16 * 3,
            y,
            "██",
            Style::default().fg(*color).bg(bg),
        );
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
        "Arrow Keys Preview  •  Enter Back  •  Esc Back  •  Shift+P Cycle",
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
                    && (base_y - by).is_multiple_of(2)
                    && hash_point(bx, by).is_multiple_of(3);
                if lit {
                    let color = if hash_point(bx + 1, by).is_multiple_of(2) {
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
