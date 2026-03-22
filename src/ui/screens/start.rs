use super::common::{self, MenuConfig, MenuItem, SynthwaveBackground};
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
    state.menu_areas = [ClickArea::default(); 5];

    if area.width < 72 || area.height < 27 {
        render_compact_start(frame, area, view, state);
        return;
    }

    let ui = theme::ui_palette();
    common::paint_synthwave(
        frame.buffer_mut(),
        area,
        SynthwaveBackground {
            sun: true,
            clouds: true,
            lit_windows: true,
        },
    );

    let menu_w = area.width.saturating_sub(20).clamp(42, 50);
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

    common::render_footer(
        frame.buffer_mut(),
        area,
        "Arrow Keys Move  •  Enter Select  •  Mouse Active",
    );
}

fn render_compact_start(
    frame: &mut Frame,
    area: Rect,
    view: &StartViewModel,
    state: &mut StartState,
) {
    let ui = theme::ui_palette();
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
    common::set_centered_string(
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
    common::set_centered_string(
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
        common::set_centered_string(
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

    common::set_centered_string(
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
    _ui: &theme::UiPalette,
) {
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
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

    let items: Vec<MenuItem> = view
        .options
        .iter()
        .map(|opt| MenuItem {
            label: opt,
            greyed: false,
        })
        .collect();

    let areas = common::render_menu_items(
        frame.buffer_mut(),
        inner,
        MenuConfig {
            items: &items,
            selected: view.selected,
            start_y_offset: 0,
            back_button: false,
        },
    );
    for (i, area) in areas.into_iter().enumerate() {
        if i < state.menu_areas.len() {
            state.menu_areas[i] = area;
        }
    }
}
