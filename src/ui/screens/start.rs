use crate::app::StartState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

pub fn render_start(frame: &mut Frame, area: Rect, state: &mut StartState) {
    // Dark background
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(8, 8, 18))),
        area,
    );

    let buf = frame.buffer_mut();

    // Title
    let title_lines = [
        "████████╗██╗   ██╗██╗ ██████╗██╗████████╗██╗   ██╗  ██████╗ ",
        "╚══██╔══╝██║   ██║██║██╔════╝██║╚══██╔══╝╚██╗ ██╔╝ ╚════██╗",
        "   ██║   ██║   ██║██║██║     ██║   ██║    ╚████╔╝   █████╔╝ ",
        "   ██║   ██║   ██║██║██║     ██║   ██║     ╚██╔╝   ██╔═══╝  ",
        "   ██║   ╚██████╔╝██║╚██████╗██║   ██║      ██║    ███████╗  ",
        "   ╚═╝    ╚═════╝ ╚═╝ ╚═════╝╚═╝   ╚═╝      ╚═╝    ╚══════╝  ",
    ];

    let title_w = 65u16;
    let title_h = title_lines.len() as u16;
    let title_x = area.x + area.width.saturating_sub(title_w) / 2;
    let title_y = area.y + 2;

    for (i, line) in title_lines.iter().enumerate() {
        let y = title_y + i as u16;
        if y < area.y + area.height {
            buf.set_string(
                title_x,
                y,
                line,
                Style::default()
                    .fg(Color::Rgb(100, 180, 255))
                    .bg(Color::Rgb(8, 8, 18)),
            );
        }
    }

    let subtitle = "Terminal City Builder";
    let sub_x = area.x + area.width.saturating_sub(subtitle.len() as u16) / 2;
    let sub_y = title_y + title_h + 1;
    if sub_y < area.y + area.height {
        buf.set_string(
            sub_x,
            sub_y,
            subtitle,
            Style::default()
                .fg(Color::Rgb(160, 160, 200))
                .bg(Color::Rgb(8, 8, 18)),
        );
    }

    // Menu box
    let menu_w = 32u16;
    let menu_h = 7u16;
    let menu_x = area.x + area.width.saturating_sub(menu_w) / 2;
    let menu_y = sub_y + 2;

    if menu_y + menu_h <= area.y + area.height {
        let menu_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);
        frame.render_widget(Clear, menu_rect);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(80, 100, 160)))
                .style(Style::default().bg(Color::Rgb(12, 12, 28))),
            menu_rect,
        );

        let buf = frame.buffer_mut();
        let options = ["Load Existing City", "Create New City", "Quit"];
        for (i, opt) in options.iter().enumerate() {
            let y = menu_y + 1 + i as u16 * 2;
            if y >= menu_y + menu_h - 1 {
                break;
            }
            
            // Record click area
            state.menu_areas[i] = crate::app::ClickArea {
                x: menu_x + 1,
                y,
                width: menu_w - 2,
                height: 1,
            };

            let is_sel = i == state.selected;
            let prefix = if is_sel { "▶ " } else { "  " };
            let text = format!("{}{}", prefix, opt);
            let style = if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(200, 180, 60))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Rgb(200, 200, 220))
                    .bg(Color::Rgb(12, 12, 28))
            };
            // Pad to menu width
            let padded = format!("{:<width$}", text, width = (menu_w - 2) as usize);
            buf.set_string(menu_x + 1, y, &padded, style);
        }
    }

    // Controls hint
    let hint = "↑↓ Navigate   Enter Select   q Quit";
    let hint_x = area.x + area.width.saturating_sub(hint.len() as u16) / 2;
    let hint_y = area.y + area.height.saturating_sub(2);
    frame.buffer_mut().set_string(
        hint_x,
        hint_y,
        hint,
        Style::default()
            .fg(Color::Rgb(80, 80, 100))
            .bg(Color::Rgb(8, 8, 18)),
    );
}
