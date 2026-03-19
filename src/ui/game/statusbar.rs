use crate::{
    app::ClickArea,
    core::sim::{SimState, TaxSector},
    ui::theme,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

/// Renders the status bar and returns the Rect of the pause/resume button.
pub fn render_statusbar(
    area: Rect,
    buf: &mut Buffer,
    sim: &SimState,
    paused: bool,
    message: Option<&str>,
) -> ClickArea {
    let ui = theme::ui_palette();

    // Fill background
    for x in area.x..area.x + area.width {
        let cell = buf.cell_mut((x, area.y)).unwrap();
        cell.set_char(' ');
        cell.set_bg(ui.status_bg);
    }

    let mut col = area.x;

    // City name
    let name = format!(" {} ", sim.city_name);
    buf.set_string(
        col,
        area.y,
        &name,
        Style::default()
            .fg(ui.status_city)
            .bg(ui.status_bg)
            .add_modifier(Modifier::BOLD),
    );
    col += name.len() as u16;

    // Separator
    put_sep(buf, col, area.y);
    col += 1;

    // Treasury
    let money = format!(" ${} ", fmt_number(sim.treasury));
    let money_color = if sim.treasury >= 0 {
        ui.success
    } else {
        ui.danger
    };
    buf.set_string(
        col,
        area.y,
        &money,
        Style::default().fg(money_color).bg(ui.status_bg),
    );
    col += money.len() as u16;

    // Separator
    put_sep(buf, col, area.y);
    col += 1;

    // Population
    let pop = format!(" Pop: {} ", fmt_number(sim.population as i64));
    buf.set_string(
        col,
        area.y,
        &pop,
        Style::default()
            .fg(theme::sector_color(TaxSector::Residential))
            .bg(ui.status_bg),
    );
    col += pop.len() as u16;

    // Separator
    put_sep(buf, col, area.y);
    col += 1;

    // Date
    let date = format!(" {} {} ", sim.month_name(), sim.year);
    buf.set_string(
        col,
        area.y,
        &date,
        Style::default().fg(ui.status_date).bg(ui.status_bg),
    );
    col += date.len() as u16;

    // Income
    put_sep(buf, col, area.y);
    col += 1;
    let income_sign = if sim.last_income >= 0 { "+" } else { "" };
    let income_str = format!(" {}${}/yr ", income_sign, fmt_number(sim.last_income));
    let income_color = if sim.last_income >= 0 {
        ui.success
    } else {
        ui.danger
    };
    buf.set_string(
        col,
        area.y,
        &income_str,
        Style::default().fg(income_color).bg(ui.status_bg),
    );
    col += income_str.len() as u16;

    // Message (ephemeral)
    if let Some(msg) = message {
        put_sep(buf, col, area.y);
        col += 1;
        let msg_str = format!(" {} ", msg);
        buf.set_string(
            col,
            area.y,
            &msg_str,
            Style::default().fg(ui.status_message).bg(ui.status_bg),
        );
        col += msg_str.len() as u16;
    }

    // Pause button — right-aligned
    let pause_text = if paused {
        "[▶ Run]  "
    } else {
        "[⏸ Paused]"
    };
    let btn_col = area.x + area.width.saturating_sub(pause_text.len() as u16 + 1);
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

    let _ = col; // suppress unused warning

    ClickArea {
        x: btn_col,
        y: area.y,
        width: pause_text.len() as u16,
        height: 1,
    }
}

fn put_sep(buf: &mut Buffer, x: u16, y: u16) {
    let ui = theme::ui_palette();
    let cell = buf.cell_mut((x, y)).unwrap();
    cell.set_char('│');
    cell.set_fg(ui.status_sep);
    cell.set_bg(ui.status_bg);
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
