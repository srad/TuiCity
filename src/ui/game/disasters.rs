use crate::{app::AppState, ui::theme};
use crate::core::sim::DisasterConfig;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, Widget},
};

// ── Disaster items ────────────────────────────────────────────────────────────

pub const DISASTER_COUNT: usize = 3;

pub fn disaster_name(idx: usize) -> &'static str {
    match idx {
        0 => "Fire & Explosions",
        1 => "Flooding",
        2 => "Tornado",
        _ => "",
    }
}

pub fn disaster_desc(idx: usize) -> &'static str {
    match idx {
        0 => "Buildings may ignite and spread flames",
        1 => "Water can flood adjacent land (annual)",
        2 => "Rare tornado carves a destructive path",
        _ => "",
    }
}

pub fn get_enabled(cfg: &DisasterConfig, idx: usize) -> bool {
    match idx {
        0 => cfg.fire_enabled,
        1 => cfg.flood_enabled,
        2 => cfg.tornado_enabled,
        _ => false,
    }
}

pub fn toggle(cfg: &mut DisasterConfig, idx: usize) {
    match idx {
        0 => cfg.fire_enabled    = !cfg.fire_enabled,
        1 => cfg.flood_enabled   = !cfg.flood_enabled,
        2 => cfg.tornado_enabled = !cfg.tornado_enabled,
        _ => {}
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

pub fn render_disasters(
    buf: &mut Buffer,
    area: Rect,
    app: &AppState,
    selected: usize,
) {
    let ui = theme::ui_palette();
    let popup_w = 48_u16;
    let popup_h = (DISASTER_COUNT as u16) * 3 + 7;
    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_area = Rect::new(x, y, popup_w.min(area.width), popup_h.min(area.height));

    Clear.render(popup_area, buf);

    Block::default()
        .title(" ☠ Disasters ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui.disaster_border))
        .bg(ui.disaster_bg)
        .render(popup_area, buf);

    let inner_x = popup_area.x + 2;
    let mut row = popup_area.y + 2;
    let max_x = popup_area.x + popup_area.width;

    let engine = app.engine.read().unwrap();
    let cfg = &engine.sim.disasters;

    for i in 0..DISASTER_COUNT {
        if row + 2 >= popup_area.y + popup_area.height { break; }

        let enabled = get_enabled(cfg, i);
        let is_sel  = i == selected;

        // Selection highlight bar
        if is_sel {
            for bx in inner_x..max_x.saturating_sub(2) {
                let cell = buf.cell_mut((bx, row)).unwrap();
                cell.set_bg(ui.disaster_select_bg);
            }
            for bx in inner_x..max_x.saturating_sub(2) {
                let cell = buf.cell_mut((bx, row + 1)).unwrap();
                cell.set_bg(ui.disaster_select_bg);
            }
        }

        // Checkbox
        let checkbox = if enabled { "[✓]" } else { "[ ]" };
        let cb_color = if enabled { ui.success } else { ui.text_muted };
        buf.set_string(inner_x, row, checkbox,
            Style::default().fg(cb_color).bg(if is_sel { ui.disaster_select_bg } else { ui.disaster_bg }));

        // Name
        let name = disaster_name(i);
        let name_style = if is_sel {
            Style::default().fg(ui.selection_fg).bg(ui.disaster_select_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui.text_primary).bg(ui.disaster_bg)
        };
        buf.set_string(inner_x + 4, row, name, name_style);

        // Description
        let desc = disaster_desc(i);
        buf.set_string(inner_x + 4, row + 1, desc,
            Style::default().fg(ui.text_muted)
                .bg(if is_sel { ui.disaster_select_bg } else { ui.disaster_bg }));

        row += 3;
    }

    // Footer
    let footer_row = popup_area.y + popup_area.height.saturating_sub(2);
    buf.set_string(inner_x, footer_row,
        "↑↓: navigate   SPACE/ENTER: toggle   ESC/D: close",
        Style::default().fg(ui.text_muted).bg(ui.disaster_bg));
}
