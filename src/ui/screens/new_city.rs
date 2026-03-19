use crate::{
    app::screens::{NewCityField, NewCityState},
    ui::{game::map_view::MapPreview, theme, view::NewCityViewModel},
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};

pub fn render_new_city(
    frame: &mut Frame,
    area: Rect,
    view: &NewCityViewModel,
    state: &mut NewCityState,
) {
    let ui = theme::ui_palette();
    let chunks = Layout::horizontal([Constraint::Fill(1), Constraint::Length(30)]).split(area);
    let map_area = chunks[0];
    let ctrl_area = chunks[1];

    let map_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui.window_border))
        .title(" MAP PREVIEW ")
        .title_style(Style::default().fg(ui.window_title))
        .style(Style::default().bg(ui.map_window_bg));
    let inner_map = map_block.inner(map_area);
    frame.render_widget(map_block, map_area);
    frame.render_widget(
        MapPreview {
            map: &view.preview_map,
        },
        inner_map,
    );

    let ctrl_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui.window_border))
        .title(" NEW CITY ")
        .title_style(Style::default().fg(ui.window_title))
        .style(Style::default().bg(ui.window_bg));
    let inner_ctrl = ctrl_block.inner(ctrl_area);
    frame.render_widget(ctrl_block, ctrl_area);

    render_controls(frame, inner_ctrl, view, state);
}

fn render_controls(
    frame: &mut Frame,
    area: Rect,
    view: &NewCityViewModel,
    state: &mut NewCityState,
) {
    if area.width < 4 || area.height < 14 {
        return;
    }

    let ui = theme::ui_palette();
    let buf = frame.buffer_mut();
    let w = area.width;
    let mut row = area.y;

    let draw_field = |buf: &mut ratatui::buffer::Buffer,
                      rect: Rect,
                      text: &str,
                      focused: bool,
                      fg: ratatui::style::Color,
                      bg: ratatui::style::Color| {
        let value = format!("{:<width$}", text, width = rect.width as usize);
        let style = if focused {
            Style::default().fg(ui.input_focus_fg).bg(ui.input_focus_bg)
        } else {
            Style::default().fg(fg).bg(bg)
        };
        buf.set_string(rect.x, rect.y, value, style);
    };

    {
        let is_focused = view.focused_field == NewCityField::CityName;
        let is_empty = view.city_name.is_empty();
        let label = if is_empty {
            "City Name: (REQUIRED)"
        } else {
            "City Name:"
        };
        let label_style = if is_empty {
            Style::default().fg(ui.danger)
        } else {
            Style::default().fg(ui.text_secondary)
        };
        buf.set_string(area.x, row, label, label_style);
        row += 1;
        state.field_areas[NewCityField::CityName as usize] = crate::app::ClickArea {
            x: area.x,
            y: row,
            width: w,
            height: 1,
        };
        draw_field(
            buf,
            Rect::new(area.x, row, w, 1),
            &view.city_name,
            is_focused,
            ui.input_fg,
            ui.input_bg,
        );
        row += 2;
    }

    {
        let is_focused = view.focused_field == NewCityField::SeedInput;
        buf.set_string(
            area.x,
            row,
            "Seed (hex):",
            Style::default().fg(ui.text_secondary),
        );
        row += 1;
        state.field_areas[NewCityField::SeedInput as usize] = crate::app::ClickArea {
            x: area.x,
            y: row,
            width: w,
            height: 1,
        };
        draw_field(
            buf,
            Rect::new(area.x, row, w, 1),
            &view.seed_text,
            is_focused,
            ui.accent_soft,
            ui.input_bg,
        );
        row += 2;
    }

    {
        let is_focused = view.focused_field == NewCityField::WaterSlider;
        buf.set_string(
            area.x,
            row,
            format!("Water: {}%", view.water_pct),
            Style::default().fg(ui.text_secondary),
        );
        row += 1;
        state.field_areas[NewCityField::WaterSlider as usize] = crate::app::ClickArea {
            x: area.x,
            y: row,
            width: w,
            height: 1,
        };
        render_bar(
            buf,
            Rect::new(area.x, row, w, 1),
            view.water_pct,
            ui.slider_water_fg,
            if is_focused {
                ui.slider_water_focus_bg
            } else {
                ui.slider_bg
            },
            ui,
        );
        row += 2;
    }

    {
        let is_focused = view.focused_field == NewCityField::TreesSlider;
        buf.set_string(
            area.x,
            row,
            format!("Trees: {}%", view.trees_pct),
            Style::default().fg(ui.text_secondary),
        );
        row += 1;
        state.field_areas[NewCityField::TreesSlider as usize] = crate::app::ClickArea {
            x: area.x,
            y: row,
            width: w,
            height: 1,
        };
        render_bar(
            buf,
            Rect::new(area.x, row, w, 1),
            view.trees_pct,
            ui.slider_trees_fg,
            if is_focused {
                ui.slider_trees_focus_bg
            } else {
                ui.slider_bg
            },
            ui,
        );
        row += 2;
    }

    for (field, text) in [
        (NewCityField::RegenerateBtn, "[Regenerate Map]"),
        (NewCityField::StartBtn, "  [▶ Start City]  "),
        (NewCityField::BackBtn, "      [Back]      "),
    ] {
        state.field_areas[field as usize] = crate::app::ClickArea {
            x: area.x,
            y: row,
            width: w,
            height: 1,
        };
        let style = if view.focused_field == field {
            Style::default()
                .fg(ui.button_focus_fg)
                .bg(ui.button_focus_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui.button_fg).bg(ui.button_bg)
        };
        buf.set_string(
            area.x,
            row,
            format!("{:<width$}", text, width = w as usize),
            style,
        );
        row += 2;
    }

    if row < area.y + area.height {
        let hint = "↑↓ Focus  ←→ Adjust  Enter Select";
        let trimmed: String = hint.chars().take(w as usize).collect();
        buf.set_string(
            area.x,
            area.y + area.height - 1,
            &trimmed,
            Style::default().fg(ui.text_dim),
        );
    }
}

fn render_bar(
    buf: &mut ratatui::buffer::Buffer,
    area: Rect,
    value: usize,
    fill: ratatui::style::Color,
    bg: ratatui::style::Color,
    ui: theme::UiPalette,
) {
    if area.width < 3 {
        return;
    }
    let inner = area.width.saturating_sub(2);
    let filled = ((value.min(100) as u32 * inner as u32) / 100) as u16;
    buf.set_string(
        area.x,
        area.y,
        "[",
        Style::default().fg(ui.text_secondary).bg(bg),
    );
    buf.set_string(
        area.x + area.width - 1,
        area.y,
        "]",
        Style::default().fg(ui.text_secondary).bg(bg),
    );
    for i in 0..inner {
        let x = area.x + 1 + i;
        let symbol = if i < filled { "█" } else { "░" };
        let fg = if i < filled { fill } else { ui.text_dim };
        buf.set_string(x, area.y, symbol, Style::default().fg(fg).bg(bg));
    }
}
