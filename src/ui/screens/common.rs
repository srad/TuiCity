use crate::app::ClickArea;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear},
    Frame,
};

// ── Constants ────────────────────────────────────────────────────────────────

pub const AUTHOR_LINE: &str = "by Saman Sedighi Rad";

// ── Color utilities ──────────────────────────────────────────────────────────

pub fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::Rgb(
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t).round() as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t).round() as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t).round() as u8,
    )
}

pub fn blend_color(base: Color, accent: Color, amount: f32) -> Color {
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

pub fn darken(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor).round() as u8,
            (g as f32 * factor).round() as u8,
            (b as f32 * factor).round() as u8,
        ),
        other => other,
    }
}

pub fn hash_point(x: u16, y: u16) -> u32 {
    let mut value = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
    value ^= value >> 13;
    value = value.wrapping_mul(1_274_126_177);
    value ^ (value >> 16)
}

// ── Text utilities ───────────────────────────────────────────────────────────

pub fn set_centered_string(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    let display = truncate(text, width as usize);
    let display_w = display.chars().count() as u16;
    let start_x = x + width.saturating_sub(display_w) / 2;
    buf.set_string(start_x, y, display, style);
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

// ── Synthwave background ─────────────────────────────────────────────────────

pub struct SynthwaveBackground {
    pub sun: bool,
    pub clouds: bool,
    pub lit_windows: bool,
}

impl Default for SynthwaveBackground {
    fn default() -> Self {
        Self {
            sun: false,
            clouds: false,
            lit_windows: false,
        }
    }
}

#[derive(Clone, Copy)]
struct ScreenChrome {
    author: Style,
    hint: Style,
    panel_title: Style,
    panel_border: Style,
    panel_fill: Style,
    menu_selected: Style,
    menu_normal: Style,
    menu_muted: Style,
    indicator: Style,
    border_type: BorderType,
}

fn screen_chrome() -> ScreenChrome {
    if crate::ui::theme::is_pixel_style() {
        ScreenChrome {
            author: Style::default()
                .fg(Color::Rgb(255, 255, 85))
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD),
            hint: Style::default()
                .fg(Color::Rgb(170, 255, 255))
                .bg(Color::Reset),
            panel_title: Style::default()
                .fg(Color::Rgb(255, 255, 85))
                .bg(Color::Rgb(0, 0, 96))
                .add_modifier(Modifier::BOLD),
            panel_border: Style::default().fg(Color::Rgb(170, 170, 170)),
            panel_fill: Style::default().bg(Color::Rgb(0, 0, 96)),
            menu_selected: Style::default()
                .fg(Color::Rgb(0, 0, 96))
                .bg(Color::Rgb(170, 170, 170))
                .add_modifier(Modifier::BOLD),
            menu_normal: Style::default()
                .fg(Color::Rgb(170, 170, 170))
                .bg(Color::Rgb(0, 0, 128)),
            menu_muted: Style::default()
                .fg(Color::Rgb(98, 118, 152))
                .bg(Color::Rgb(0, 0, 80)),
            indicator: Style::default()
                .fg(Color::Rgb(85, 255, 255))
                .bg(Color::Rgb(0, 0, 96)),
            border_type: BorderType::Double,
        }
    } else {
        ScreenChrome {
            author: Style::default()
                .fg(Color::Rgb(255, 205, 127))
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD),
            hint: Style::default()
                .fg(Color::Rgb(170, 223, 219))
                .bg(Color::Reset),
            panel_title: Style::default()
                .fg(Color::Rgb(255, 221, 119))
                .bg(Color::Rgb(35, 34, 55))
                .add_modifier(Modifier::BOLD),
            panel_border: Style::default().fg(Color::Rgb(106, 226, 225)),
            panel_fill: Style::default().bg(Color::Rgb(35, 34, 55)),
            menu_selected: Style::default()
                .fg(Color::Rgb(28, 28, 42))
                .bg(Color::Rgb(255, 221, 119))
                .add_modifier(Modifier::BOLD),
            menu_normal: Style::default()
                .fg(Color::Rgb(238, 232, 225))
                .bg(Color::Rgb(56, 42, 78)),
            menu_muted: Style::default()
                .fg(Color::Rgb(100, 100, 110))
                .bg(Color::Rgb(40, 36, 58)),
            indicator: Style::default()
                .fg(Color::Rgb(170, 223, 219))
                .bg(Color::Rgb(35, 34, 55)),
            border_type: BorderType::Plain,
        }
    }
}

pub fn paint_synthwave(buf: &mut Buffer, area: Rect, opts: SynthwaveBackground) {
    if crate::ui::theme::is_pixel_style() {
        paint_pixel_backdrop(buf, area, opts);
        return;
    }

    if area.width == 0 || area.height == 0 {
        return;
    }

    let horizon_y = area.y + area.height.saturating_mul(2) / 3;
    let sun_x = area.x + area.width / 2;
    let sun_y = area.y + area.height / 4;

    // Gradient sky
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

    if opts.sun {
        paint_sun(buf, area, sun_x, sun_y, horizon_y);
    }
    if opts.clouds {
        paint_clouds(buf, area);
    }

    if opts.lit_windows {
        paint_grid_perspective(buf, area, horizon_y);
        paint_skyline_lit(buf, area, horizon_y);
    } else {
        paint_grid_simple(buf, area, horizon_y);
        paint_skyline_dark(buf, area, horizon_y);
    }

    paint_scanlines(buf, area);
}

fn paint_pixel_backdrop(buf: &mut Buffer, area: Rect, opts: SynthwaveBackground) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    for y in area.y..area.y + area.height {
        let t = (y - area.y) as f32 / area.height.max(1) as f32;
        let row = if t < 0.45 {
            lerp_color((0, 0, 32), (0, 0, 96), t / 0.45)
        } else {
            lerp_color((0, 0, 96), (0, 0, 48), (t - 0.45) / 0.55)
        };
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ").set_fg(row).set_bg(row);
            }
        }
    }

    let horizon = area.y + area.height.saturating_mul(3) / 5;
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, horizon)) {
            cell.set_symbol("═").set_fg(Color::Rgb(85, 255, 255));
        }
    }

    for y in area.y + 1..horizon.saturating_sub(1) {
        for x in area.x + 1..area.x + area.width.saturating_sub(1) {
            let seed = hash_point(x, y);
            if seed.is_multiple_of(151) || (opts.clouds && seed.is_multiple_of(197)) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    let symbol = if seed.is_multiple_of(2) { "•" } else { "·" };
                    let color = if seed.is_multiple_of(3) {
                        Color::Rgb(255, 255, 85)
                    } else {
                        Color::Rgb(170, 170, 170)
                    };
                    cell.set_symbol(symbol).set_fg(color);
                }
            }
        }
    }

    let mut x = area.x;
    while x < area.x + area.width {
        let seed = hash_point(x, horizon);
        let width = 5 + (seed % 7) as u16;
        let height = 4 + ((seed / 17) % 6) as u16;
        let top = horizon.saturating_sub(height);
        let right = (x + width).min(area.x + area.width);
        for bx in x..right {
            for by in top..horizon {
                if let Some(cell) = buf.cell_mut((bx, by)) {
                    cell.set_symbol(" ")
                        .set_fg(Color::Rgb(0, 0, 64))
                        .set_bg(Color::Rgb(0, 0, 64));
                }
                let lit = opts.lit_windows
                    && bx > x
                    && bx + 1 < right
                    && by > top
                    && (bx - x) % 2 == 1
                    && (horizon - by).is_multiple_of(2)
                    && hash_point(bx, by).is_multiple_of(3);
                if lit {
                    if let Some(cell) = buf.cell_mut((bx, by)) {
                        cell.set_symbol("■").set_fg(Color::Rgb(255, 255, 85));
                    }
                }
            }
        }
        x = right.saturating_add(1);
    }

    for y in horizon + 1..area.y + area.height {
        if (y - horizon).is_multiple_of(2) {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol("░").set_fg(Color::Rgb(0, 32, 96));
                }
            }
        }
    }
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
            if seed.is_multiple_of(97) || seed.is_multiple_of(131) {
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

/// Perspective grid used by start and load_city screens.
fn paint_grid_perspective(buf: &mut Buffer, area: Rect, horizon_y: u16) {
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

/// Simple grid used by settings, llm_setup, and theme_settings screens.
fn paint_grid_simple(buf: &mut Buffer, area: Rect, horizon_y: u16) {
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

/// Skyline with lit windows, used by start, load_city, and theme_settings.
fn paint_skyline_lit(buf: &mut Buffer, area: Rect, base_y: u16) {
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

/// Plain dark skyline used by settings and llm_setup.
fn paint_skyline_dark(buf: &mut Buffer, area: Rect, base_y: u16) {
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

// ── Footer ───────────────────────────────────────────────────────────────────

pub fn render_footer(buf: &mut Buffer, area: Rect, hint: &str) {
    if area.height < 3 {
        return;
    }

    let chrome = screen_chrome();
    set_centered_string(
        buf,
        area.x,
        area.y + area.height.saturating_sub(3),
        area.width,
        AUTHOR_LINE,
        chrome.author,
    );
    set_centered_string(
        buf,
        area.x,
        area.y + area.height.saturating_sub(2),
        area.width,
        hint,
        chrome.hint,
    );
}

// ── Panel layout ─────────────────────────────────────────────────────────────

pub struct PanelLayout {
    pub panel: Rect,
    pub inner: Rect,
}

pub fn centered_panel(
    area: Rect,
    min_w: u16,
    max_w: u16,
    max_h: u16,
    top_offset: u16,
) -> PanelLayout {
    let panel_w = area.width.saturating_sub(18).clamp(min_w, max_w);
    let available_h = area.height.saturating_sub(top_offset + 4).max(8);
    let panel_h = available_h.min(max_h);
    let panel_x = area.x + area.width.saturating_sub(panel_w) / 2;
    let panel_y = area.y + top_offset;

    let panel = Rect::new(panel_x, panel_y, panel_w, panel_h);
    let inner = Rect::new(
        panel.x + 2,
        panel.y + 1,
        panel.width.saturating_sub(4),
        panel.height.saturating_sub(2),
    );

    PanelLayout { panel, inner }
}

pub fn render_bordered_panel(frame: &mut Frame, layout: &PanelLayout, title: &str) {
    let chrome = screen_chrome();
    frame.render_widget(Clear, layout.panel);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(chrome.border_type)
            .title(format!(" {} ", title))
            .title_style(chrome.panel_title)
            .border_style(chrome.panel_border)
            .style(chrome.panel_fill),
        layout.panel,
    );
}

// ── Menu items ───────────────────────────────────────────────────────────────

pub struct MenuItem<'a> {
    pub label: &'a str,
    pub greyed: bool,
}

pub struct MenuConfig<'a> {
    pub items: &'a [MenuItem<'a>],
    pub selected: usize,
    pub start_y_offset: u16,
    /// If true, a "Back" item is automatically appended after all items.
    pub back_button: bool,
}

/// Renders a standalone "Back" button at a given y position.
/// Used by screens with custom item rendering that still want a standard back button.
pub fn render_back_button(buf: &mut Buffer, inner: Rect, y: u16, selected: bool) -> ClickArea {
    let chrome = screen_chrome();
    let row_style = if selected {
        chrome.menu_selected
    } else {
        chrome.menu_normal
    };

    let blank = format!("{:^width$}", " ", width = inner.width as usize);
    buf.set_string(inner.x, y, blank.clone(), row_style);
    buf.set_string(
        inner.x,
        y + 1,
        format!("{:^width$}", "Back", width = inner.width as usize),
        row_style,
    );
    buf.set_string(inner.x, y + 2, blank, row_style);

    ClickArea {
        x: inner.x,
        y,
        width: inner.width,
        height: 3,
    }
}

/// How many 3-row items (each followed by a 1-row gap) fit in `avail` rows.
fn items_that_fit(avail: usize) -> usize {
    if avail >= 3 {
        (avail - 3) / 4 + 1
    } else {
        0
    }
}

pub fn render_menu_items(buf: &mut Buffer, inner: Rect, config: MenuConfig) -> Vec<ClickArea> {
    let chrome = screen_chrome();
    let back_item = MenuItem {
        label: "Back",
        greyed: false,
    };

    let total: Vec<&MenuItem> = config
        .items
        .iter()
        .chain(if config.back_button {
            std::slice::from_ref(&back_item)
        } else {
            &[]
        })
        .collect();

    let n = total.len();
    let mut areas = vec![ClickArea::default(); n];

    let avail = inner.height.saturating_sub(config.start_y_offset) as usize;
    let max_no_scroll = items_that_fit(avail);

    if n <= max_no_scroll {
        // Everything fits — render without scrolling
        for (idx, item) in total.iter().enumerate() {
            let row_y = inner.y + config.start_y_offset + idx as u16 * 4;
            areas[idx] = render_single_item(buf, inner, row_y, item, idx == config.selected);
        }
    } else {
        // Scrolling needed — reserve 1 row top + 1 row bottom for indicators
        let scroll_avail = avail.saturating_sub(2);
        let visible = items_that_fit(scroll_avail).max(1).min(n);

        // Keep selected item in view, centering when possible
        let half = visible / 2;
        let scroll_start = if config.selected <= half {
            0
        } else if config.selected + visible.saturating_sub(half) >= n {
            n.saturating_sub(visible)
        } else {
            config.selected.saturating_sub(half)
        };
        let scroll_end = (scroll_start + visible).min(n);

        let has_above = scroll_start > 0;
        let has_below = scroll_end < n;

        // Up indicator
        let top_y = inner.y + config.start_y_offset;
        if has_above {
            set_centered_string(
                buf,
                inner.x,
                top_y,
                inner.width,
                "▲ more ▲",
                chrome.indicator,
            );
        }

        // Render visible items
        let items_y = top_y + 1; // 1 row reserved for up indicator
        for (vis_idx, item_idx) in (scroll_start..scroll_end).enumerate() {
            let row_y = items_y + vis_idx as u16 * 4;
            areas[item_idx] = render_single_item(
                buf,
                inner,
                row_y,
                &total[item_idx],
                item_idx == config.selected,
            );
        }

        // Down indicator
        if has_below {
            let bottom_y = inner.y + inner.height - 1;
            set_centered_string(
                buf,
                inner.x,
                bottom_y,
                inner.width,
                "▼ more ▼",
                chrome.indicator,
            );
        }
    }

    areas
}

fn render_single_item(
    buf: &mut Buffer,
    inner: Rect,
    row_y: u16,
    item: &MenuItem,
    selected: bool,
) -> ClickArea {
    let chrome = screen_chrome();
    let row_style = if selected {
        chrome.menu_selected
    } else if item.greyed {
        chrome.menu_muted
    } else {
        chrome.menu_normal
    };

    let blank = format!("{:^width$}", " ", width = inner.width as usize);
    buf.set_string(inner.x, row_y, blank.clone(), row_style);
    buf.set_string(
        inner.x,
        row_y + 1,
        format!(
            "{:^width$}",
            truncate(item.label, inner.width.saturating_sub(4) as usize),
            width = inner.width as usize
        ),
        row_style,
    );
    buf.set_string(inner.x, row_y + 2, blank, row_style);

    ClickArea {
        x: inner.x,
        y: row_y,
        width: inner.width,
        height: 3,
    }
}
