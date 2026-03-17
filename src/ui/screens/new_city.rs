use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};
use crate::{
    app::{NewCityField, NewCityState},
    ui::game::map_view::MapPreview,
};

pub fn render_new_city(frame: &mut Frame, area: Rect, state: &NewCityState) {
    // Split: left = map preview, right = controls
    let chunks = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(30),
    ])
    .split(area);

    let map_area = chunks[0];
    let ctrl_area = chunks[1];

    // Map preview (left)
    let map_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 80, 120)))
        .title(" MAP PREVIEW ")
        .title_style(Style::default().fg(Color::Rgb(150, 180, 255)))
        .style(Style::default().bg(Color::Rgb(8, 12, 8)));
    let inner_map = map_block.inner(map_area);
    frame.render_widget(map_block, map_area);
    frame.render_widget(MapPreview { map: &state.preview_map }, inner_map);

    // Controls (right)
    let ctrl_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 80, 120)))
        .title(" NEW CITY ")
        .title_style(Style::default().fg(Color::Rgb(150, 180, 255)))
        .style(Style::default().bg(Color::Rgb(8, 8, 18)));
    let inner_ctrl = ctrl_block.inner(ctrl_area);
    frame.render_widget(ctrl_block, ctrl_area);

    render_controls(frame, inner_ctrl, state);
}

fn render_controls(frame: &mut Frame, area: Rect, state: &NewCityState) {
    if area.width < 4 || area.height < 8 {
        return;
    }

    let buf = frame.buffer_mut();
    let w = area.width as usize;
    let mut row = area.y;

    // City Name
    {
        let is_focused = state.focused_field == NewCityField::CityName;
        let label = "City Name:";
        buf.set_string(
            area.x,
            row,
            label,
            Style::default()
                .fg(Color::Rgb(180, 180, 220))
                .bg(Color::Rgb(8, 8, 18)),
        );
        row += 1;

        if row >= area.y + area.height { return; }

        let display = if state.city_name.is_empty() {
            "_".repeat((w - 2).min(20))
        } else {
            format!("{}_", state.city_name)
        };
        let padded = format!("{:<width$}", display, width = w);
        let style = if is_focused {
            Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 60))
        } else {
            Style::default()
                .fg(Color::Rgb(220, 220, 100))
                .bg(Color::Rgb(25, 25, 40))
        };
        buf.set_string(area.x, row, &padded, style);
        row += 2;
    }

    if row >= area.y + area.height { return; }

    // Seed field
    {
        let is_focused = state.focused_field == NewCityField::SeedInput;
        buf.set_string(
            area.x,
            row,
            "Seed (hex):",
            Style::default()
                .fg(Color::Rgb(180, 180, 220))
                .bg(Color::Rgb(8, 8, 18)),
        );
        row += 1;

        if row >= area.y + area.height { return; }

        let display = format!("{}_", state.seed_input);
        let padded = format!("{:<width$}", display, width = w);
        let style = if is_focused {
            Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 60))
        } else {
            Style::default()
                .fg(Color::Rgb(160, 200, 255))
                .bg(Color::Rgb(20, 20, 45))
        };
        buf.set_string(area.x, row, &padded, style);
        row += 2;
    }

    if row >= area.y + area.height { return; }

    // Water slider
    render_slider(
        buf,
        area.x,
        row,
        w,
        "Water %",
        state.water_pct,
        100,
        state.focused_field == NewCityField::WaterSlider,
        Color::Rgb(64, 164, 223),
    );
    row += 2;

    if row >= area.y + area.height { return; }

    // Trees slider
    render_slider(
        buf,
        area.x,
        row,
        w,
        "Trees %",
        state.trees_pct,
        100,
        state.focused_field == NewCityField::TreesSlider,
        Color::Rgb(60, 160, 60),
    );
    row += 2;

    if row >= area.y + area.height { return; }

    row += 1; // spacer

    // Buttons
    let btns = [
        (NewCityField::RegenerateBtn, "[Regenerate Map]"),
        (NewCityField::StartBtn, "  [▶ Start City]  "),
        (NewCityField::BackBtn, "      [Back]      "),
    ];

    for (field, label) in &btns {
        if row >= area.y + area.height { break; }
        let is_focused = state.focused_field == *field;
        let style = if is_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(220, 200, 60))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Rgb(180, 180, 220))
                .bg(Color::Rgb(8, 8, 18))
        };
        let padded = format!("{:^width$}", label, width = w);
        buf.set_string(area.x, row, &padded, style);
        row += 1;

        if row >= area.y + area.height { break; }
        row += 1; // spacing between buttons
    }

    if row < area.y + area.height {
        let hint = "↑↓ Focus  ←→ Adjust  Enter Select";
        let trimmed: String = hint.chars().take(w).collect();
        buf.set_string(
            area.x,
            area.y + area.height - 1,
            &trimmed,
            Style::default()
                .fg(Color::Rgb(80, 80, 100))
                .bg(Color::Rgb(8, 8, 18)),
        );
    }
}

fn render_slider(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    w: usize,
    label: &str,
    value: u8,
    max: u8,
    focused: bool,
    fill_color: Color,
) {
    // Label line
    let label_str = format!("{}: {}%", label, value);
    buf.set_string(
        x,
        y,
        &format!("{:<width$}", label_str, width = w),
        Style::default()
            .fg(if focused {
                Color::Rgb(255, 220, 60)
            } else {
                Color::Rgb(180, 180, 220)
            })
            .bg(Color::Rgb(8, 8, 18)),
    );

    // Slider bar
    let bar_w = (w.saturating_sub(2)) as u16;
    let filled = ((value as u32 * bar_w as u32) / max as u32) as u16;

    buf.set_string(
        x,
        y + 1,
        "[",
        Style::default()
            .fg(Color::Rgb(120, 120, 140))
            .bg(Color::Rgb(8, 8, 18)),
    );

    for i in 0..bar_w {
        let ch = if i < filled { '█' } else { '─' };
        let fg = if i < filled {
            fill_color
        } else {
            Color::Rgb(50, 50, 60)
        };
        let cell = buf.cell_mut((x + 1 + i, y + 1)).unwrap();
        cell.set_char(ch);
        cell.set_fg(fg);
        cell.set_bg(Color::Rgb(15, 15, 25));
    }

    buf.set_string(
        x + 1 + bar_w,
        y + 1,
        "]",
        Style::default()
            .fg(Color::Rgb(120, 120, 140))
            .bg(Color::Rgb(8, 8, 18)),
    );
}
