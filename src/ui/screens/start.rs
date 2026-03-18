use crate::app::screens::StartState;
use crate::ui::theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

pub fn render_start(frame: &mut Frame, area: Rect, state: &mut StartState) {
    let ui = theme::ui_palette();

    frame.render_widget(
        Block::default().style(Style::default().bg(ui.desktop_bg)),
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
                Style::default().fg(ui.title).bg(ui.desktop_bg),
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
            Style::default().fg(ui.subtitle).bg(ui.desktop_bg),
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
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.window_bg)),
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
                    .fg(ui.selection_fg)
                    .bg(ui.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui.text_primary).bg(ui.window_bg)
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
        Style::default().fg(ui.text_dim).bg(ui.desktop_bg),
    );
}
