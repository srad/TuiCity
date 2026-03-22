use crate::{
    app::ClickArea,
    core::{
        map::ViewLayer,
        sim::{SimState, TaxSector},
    },
    ui::{painter::StatusBarAreas, theme},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

pub fn render_statusbar(
    area: Rect,
    buf: &mut Buffer,
    sim: &SimState,
    paused: bool,
    view_layer: ViewLayer,
    message: Option<&str>,
) -> StatusBarAreas {
    let ui = theme::ui_palette();

    // Fill background
    for x in area.x..area.x + area.width {
        let cell = buf.cell_mut((x, area.y)).unwrap();
        cell.set_char(' ');
        cell.set_bg(ui.status_bg);
    }

    let controls = render_right_controls(area, buf, paused, view_layer);
    let mut col = area.x;
    let right_limit = controls.pause_btn.x.saturating_sub(1);

    // City name
    let name = format!(" {} ", sim.city_name);
    if write_status_text(
        buf,
        area.y,
        &mut col,
        right_limit,
        &name,
        Style::default()
            .fg(ui.status_city)
            .bg(ui.status_bg)
            .add_modifier(Modifier::BOLD),
    )
    .is_none()
    {
        return controls;
    }

    // Separator
    if !write_sep(buf, &mut col, area.y, right_limit) {
        return controls;
    }

    // Treasury
    let money = format!(" ${} ", fmt_number(sim.economy.treasury));
    let money_color = if sim.economy.treasury >= 0 {
        ui.success
    } else {
        ui.danger
    };
    if write_status_text(
        buf,
        area.y,
        &mut col,
        right_limit,
        &money,
        Style::default().fg(money_color).bg(ui.status_bg),
    )
    .is_none()
    {
        return controls;
    }

    // Separator
    if !write_sep(buf, &mut col, area.y, right_limit) {
        return controls;
    }

    // Population
    let pop = format!(" Pop: {} ", fmt_number(sim.pop.population as i64));
    if write_status_text(
        buf,
        area.y,
        &mut col,
        right_limit,
        &pop,
        Style::default()
            .fg(theme::sector_color(TaxSector::Residential))
            .bg(ui.status_bg),
    )
    .is_none()
    {
        return controls;
    }

    // Separator
    if !write_sep(buf, &mut col, area.y, right_limit) {
        return controls;
    }

    // Date
    let date = format!(" {} {} ", sim.month_name(), sim.year);
    if write_status_text(
        buf,
        area.y,
        &mut col,
        right_limit,
        &date,
        Style::default().fg(ui.status_date).bg(ui.status_bg),
    )
    .is_none()
    {
        return controls;
    }

    // Income
    if !write_sep(buf, &mut col, area.y, right_limit) {
        return controls;
    }
    let income_sign = if sim.economy.last_income >= 0 { "+" } else { "" };
    let income_str = format!(" {}${}/yr ", income_sign, fmt_number(sim.economy.last_income));
    let income_color = if sim.economy.last_income >= 0 {
        ui.success
    } else {
        ui.danger
    };
    if write_status_text(
        buf,
        area.y,
        &mut col,
        right_limit,
        &income_str,
        Style::default().fg(income_color).bg(ui.status_bg),
    )
    .is_none()
    {
        return controls;
    }

    // Message (ephemeral)
    if let Some(msg) = message {
        if !write_sep(buf, &mut col, area.y, right_limit) {
            return controls;
        }
        let msg_str = format!(" {} ", msg);
        let _ = write_status_text(
            buf,
            area.y,
            &mut col,
            right_limit,
            &msg_str,
            Style::default().fg(ui.status_message).bg(ui.status_bg),
        );
    }

    controls
}

fn render_right_controls(
    area: Rect,
    buf: &mut Buffer,
    paused: bool,
    view_layer: ViewLayer,
) -> StatusBarAreas {
    let ui = theme::ui_palette();
    let pause_text = if paused { "[Run]  " } else { "[Pause]" };
    let (layer_label, surface_text, underground_text) = if area.width >= 72 {
        (" Layer ", " Surface ", " Underground ")
    } else {
        (" L ", " S ", " U ")
    };
    let right_padding = 1;
    let layer_gap = 1;
    let pause_gap = 1;
    let total_width = layer_label.len() as u16
        + surface_text.len() as u16
        + layer_gap
        + underground_text.len() as u16
        + pause_gap
        + pause_text.len() as u16;
    let controls_start = area
        .x
        .saturating_add(area.width.saturating_sub(total_width + right_padding));
    let mut col = controls_start;

    let label_style = Style::default().fg(ui.status_date).bg(ui.status_bg);
    buf.set_string(col, area.y, layer_label, label_style);
    col += layer_label.len() as u16;

    let selected_style = Style::default()
        .fg(ui.selection_fg)
        .bg(ui.selection_bg)
        .add_modifier(Modifier::BOLD);
    let inactive_style = Style::default()
        .fg(ui.toolbar_button_fg)
        .bg(ui.toolbar_button_bg);

    let surface_btn = ClickArea {
        x: col,
        y: area.y,
        width: surface_text.len() as u16,
        height: 1,
    };
    buf.set_string(
        col,
        area.y,
        surface_text,
        if view_layer == ViewLayer::Surface {
            selected_style
        } else {
            inactive_style
        },
    );
    col += surface_text.len() as u16;

    buf.set_string(col, area.y, " ", Style::default().bg(ui.status_bg));
    col += layer_gap;

    let underground_btn = ClickArea {
        x: col,
        y: area.y,
        width: underground_text.len() as u16,
        height: 1,
    };
    buf.set_string(
        col,
        area.y,
        underground_text,
        if view_layer == ViewLayer::Underground {
            selected_style
        } else {
            inactive_style
        },
    );
    col += underground_text.len() as u16;

    buf.set_string(col, area.y, " ", Style::default().bg(ui.status_bg));
    col += pause_gap;

    let btn_col = col;
    let pause_style = if paused {
        Style::default()
            .fg(ui.status_button_run_fg)
            .bg(ui.status_button_run_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(ui.status_button_pause_fg)
            .bg(ui.status_button_pause_bg)
    };
    buf.set_string(btn_col, area.y, pause_text, pause_style);

    StatusBarAreas {
        pause_btn: ClickArea {
            x: btn_col,
            y: area.y,
            width: pause_text.len() as u16,
            height: 1,
        },
        layer_surface_btn: surface_btn,
        layer_underground_btn: underground_btn,
    }
}

fn put_sep(buf: &mut Buffer, x: u16, y: u16) {
    let ui = theme::ui_palette();
    let cell = buf.cell_mut((x, y)).unwrap();
    cell.set_char('│');
    cell.set_fg(ui.status_sep);
    cell.set_bg(ui.status_bg);
}

fn write_sep(buf: &mut Buffer, col: &mut u16, y: u16, right_limit: u16) -> bool {
    if *col >= right_limit {
        return false;
    }
    put_sep(buf, *col, y);
    *col += 1;
    true
}

fn write_status_text(
    buf: &mut Buffer,
    y: u16,
    col: &mut u16,
    right_limit: u16,
    text: &str,
    style: Style,
) -> Option<()> {
    if *col >= right_limit {
        return None;
    }
    let available = right_limit.saturating_sub(*col) as usize;
    if available == 0 {
        return None;
    }
    let clipped = truncate(text, available);
    if clipped.is_empty() {
        return None;
    }
    buf.set_string(*col, y, &clipped, style);
    *col += clipped.chars().count() as u16;
    Some(())
}

fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::SimState;
    use ratatui::buffer::Buffer;

    #[test]
    fn statusbar_returns_click_areas_for_layer_switch_and_pause() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 1));

        let areas = render_statusbar(
            Rect::new(0, 0, 120, 1),
            &mut buf,
            &SimState::default(),
            false,
            ViewLayer::Surface,
            Some("Ready"),
        );

        assert!(areas.pause_btn.width > 0);
        assert!(areas.layer_surface_btn.width > 0);
        assert!(areas.layer_underground_btn.width > 0);
        assert!(areas.layer_surface_btn.x < areas.layer_underground_btn.x);
        assert!(areas.layer_underground_btn.x < areas.pause_btn.x);
    }

    #[test]
    fn statusbar_highlights_active_layer_segment() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 1));

        let areas = render_statusbar(
            Rect::new(0, 0, 120, 1),
            &mut buf,
            &SimState::default(),
            false,
            ViewLayer::Underground,
            None,
        );

        let surface_bg = buf.cell((areas.layer_surface_btn.x, 0)).unwrap().bg;
        let underground_bg = buf.cell((areas.layer_underground_btn.x, 0)).unwrap().bg;
        assert_ne!(surface_bg, underground_bg);
    }
}
