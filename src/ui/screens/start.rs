use crate::{
    app::{screens::StartState, ClickArea},
    ui::{theme, view::StartViewModel},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

const AUTHOR_LINE: &str = "by Saman Sedighi Rad";
const FOOTER_HEIGHT: u16 = 3;
const MENU_HEIGHT: u16 = 17;
const TITLE_TO_MENU_GAP: u16 = 1;
const TITLE_ART: [&str; 4] = [
    "████████╗██╗   ██╗██╗ ██████╗██╗████████╗██╗   ██╗",
    "╚══██╔══╝██║   ██║██║██╔════╝██║╚══██╔══╝╚██╗ ██╔╝",
    "   ██║   ██║   ██║██║██║     ██║   ██║    ╚████╔╝ ",
    "   ██║   ╚██████╔╝██║╚██████╗██║   ██║      ██║   ",
];
const TITLE_ART_LARGE: [&str; 6] = [
    "████████╗██╗   ██╗██╗ ██████╗██╗████████╗██╗   ██╗",
    "╚══██╔══╝██║   ██║██║██╔════╝██║╚══██╔══╝╚██╗ ██╔╝",
    "   ██║   ██║   ██║██║██║     ██║   ██║    ╚████╔╝ ",
    "   ██║   ██║   ██║██║██║     ██║   ██║     ╚██╔╝  ",
    "   ██║   ╚██████╔╝██║╚██████╗██║   ██║      ██║   ",
    "   ╚═╝    ╚═════╝ ╚═╝ ╚═════╝╚═╝   ╚═╝      ╚═╝   ",
];

pub fn render_start(frame: &mut Frame, area: Rect, view: &StartViewModel, state: &mut StartState) {
    state.menu_areas = [ClickArea::default(); 4];

    if area.width < 72 || area.height < 27 {
        render_compact_start(frame, area, view, state);
        return;
    }

    let ui = theme::ui_palette();
    paint_background(frame.buffer_mut(), area);

    let menu_w = area.width.saturating_sub(20).min(50).max(42);
    let menu_x = area.x + area.width.saturating_sub(menu_w) / 2;
    let title_art = choose_title_art(area);
    let content_h = title_block_height(title_art) + TITLE_TO_MENU_GAP + MENU_HEIGHT;
    let content_y = area.y + area.height.saturating_sub(FOOTER_HEIGHT + content_h) / 2;
    let title_y = content_y;
    let menu_y = title_y + title_block_height(title_art) + TITLE_TO_MENU_GAP;

    paint_title_backdrop(frame.buffer_mut(), area, title_y, title_art);
    render_title(frame.buffer_mut(), area, title_y, title_art);
    render_menu(
        frame,
        Rect::new(menu_x, menu_y, menu_w, MENU_HEIGHT),
        view,
        state,
        &ui,
    );

    render_footer(frame.buffer_mut(), area);
}

fn render_compact_start(
    frame: &mut Frame,
    area: Rect,
    view: &StartViewModel,
    state: &mut StartState,
) {
    let ui = theme::ui_palette();
    paint_background(frame.buffer_mut(), area);

    let rect = Rect::new(
        area.x + 2,
        area.y + area.height.saturating_sub(12) / 2,
        area.width.saturating_sub(4),
        10.min(area.height.saturating_sub(2)),
    );
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(106, 226, 225)))
            .style(Style::default().bg(Color::Rgb(33, 35, 54))),
        rect,
    );

    let buf = frame.buffer_mut();
    set_centered_string(
        buf,
        rect.x,
        rect.y + 1,
        rect.width,
        "TuiCity 2000",
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .bg(Color::Rgb(33, 35, 54))
            .add_modifier(Modifier::BOLD),
    );
    set_centered_string(
        buf,
        rect.x,
        rect.y + 2,
        rect.width,
        "terminal city builder",
        Style::default()
            .fg(Color::Rgb(162, 235, 228))
            .bg(Color::Rgb(33, 35, 54)),
    );

    for (i, opt) in view.options.iter().enumerate() {
        let y = rect.y + 4 + i as u16;
        if y >= rect.y + rect.height.saturating_sub(1) {
            break;
        }
        state.menu_areas[i] = ClickArea {
            x: rect.x + 2,
            y,
            width: rect.width.saturating_sub(4),
            height: 1,
        };
        let selected = i == view.selected;
        let text = if selected {
            format!("> {} <", opt)
        } else {
            opt.to_string()
        };
        let padded = format!(
            "{:^width$}",
            text,
            width = rect.width.saturating_sub(4) as usize
        );
        let style = if selected {
            Style::default()
                .fg(Color::Rgb(27, 27, 42))
                .bg(Color::Rgb(255, 221, 119))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(33, 35, 54))
        };
        buf.set_string(rect.x + 2, y, padded, style);
    }
}

fn choose_title_art(area: Rect) -> &'static [&'static str] {
    let required_height =
        title_block_height(&TITLE_ART_LARGE) + TITLE_TO_MENU_GAP + MENU_HEIGHT + FOOTER_HEIGHT;
    if area.width >= 80 && area.height >= required_height + 2 {
        &TITLE_ART_LARGE
    } else {
        &TITLE_ART
    }
}

fn title_block_height(lines: &[&str]) -> u16 {
    lines.len() as u16 + 1
}

fn title_block_width(lines: &[&str]) -> u16 {
    lines
        .iter()
        .map(|line| line.chars().count() as u16)
        .max()
        .unwrap_or(0)
}

fn paint_title_backdrop(buf: &mut Buffer, area: Rect, title_y: u16, lines: &[&str]) {
    let title_w = title_block_width(lines);
    let title_h = title_block_height(lines);
    if title_w == 0 || title_h == 0 {
        return;
    }

    let center_x = area.x + area.width / 2;
    let center_y = title_y + title_h / 2;
    let radius_x = (title_w / 2).saturating_add(8).min(area.width / 2);
    let radius_y = (title_h / 2).saturating_add(2);
    let start_x = center_x.saturating_sub(radius_x);
    let end_x = (center_x + radius_x).min(area.x + area.width.saturating_sub(1));
    let start_y = title_y.saturating_sub(1);
    let end_y = (title_y + title_h + 1).min(area.y + area.height.saturating_sub(1));

    for y in start_y..=end_y {
        for x in start_x..=end_x {
            let dx = x as i32 - center_x as i32;
            let dy = y as i32 - center_y as i32;
            let nx = dx as f32 / radius_x.max(1) as f32;
            let ny = dy as f32 / radius_y.max(1) as f32;
            let dist = nx * nx + ny * ny;
            if dist > 1.0 {
                continue;
            }

            let color = if dist < 0.28 {
                Color::Rgb(24, 28, 68)
            } else if dist < 0.62 {
                Color::Rgb(37, 36, 82)
            } else {
                Color::Rgb(54, 46, 98)
            };

            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ").set_fg(color).set_bg(color);
            }
        }
    }
}

fn render_title(buf: &mut Buffer, area: Rect, y: u16, lines: &[&str]) {
    let title_colors = [
        Color::Rgb(255, 147, 94),
        Color::Rgb(255, 183, 107),
        Color::Rgb(164, 241, 219),
        Color::Rgb(106, 226, 225),
    ];

    for (idx, line) in lines.iter().enumerate() {
        set_centered_string(
            buf,
            area.x,
            y + idx as u16,
            area.width,
            line,
            Style::default()
                .fg(title_colors[idx % title_colors.len()])
                .add_modifier(Modifier::BOLD),
        );
    }

    set_centered_string(
        buf,
        area.x,
        y + lines.len() as u16,
        area.width,
        "2 0 0 0",
        Style::default()
            .fg(Color::Rgb(255, 221, 119))
            .add_modifier(Modifier::BOLD),
    );
}

fn render_menu(
    frame: &mut Frame,
    rect: Rect,
    view: &StartViewModel,
    state: &mut StartState,
    ui: &theme::UiPalette,
) {
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
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

    for (i, opt) in view.options.iter().enumerate() {
        let option_y = inner.y + i as u16 * 4;
        if option_y + 2 >= inner.y + inner.height {
            break;
        }
        state.menu_areas[i] = ClickArea {
            x: inner.x,
            y: option_y,
            width: inner.width,
            height: 3,
        };

        let selected = i == view.selected;
        let line_style = if selected {
            Style::default()
                .fg(Color::Rgb(28, 28, 42))
                .bg(Color::Rgb(255, 221, 119))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(ui.text_primary)
                .bg(Color::Rgb(56, 42, 78))
        };

        let blank = format!("{:^width$}", " ", width = inner.width as usize);
        let label = format!("  {}  ", opt);
        let mid = format!("{:^width$}", label, width = inner.width as usize);
        buf.set_string(inner.x, option_y, blank.clone(), line_style);
        buf.set_string(inner.x, option_y + 1, mid, line_style);
        buf.set_string(inner.x, option_y + 2, blank, line_style);
    }
}

fn render_footer(buf: &mut Buffer, area: Rect) {
    if area.height < FOOTER_HEIGHT {
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
        "Arrow Keys Move  •  Enter Select  •  Mouse Active",
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
