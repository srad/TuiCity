use crate::{
    app::{NewCityField, NewCityState},
    ui::game::map_view::MapPreview,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};

pub fn render_new_city(frame: &mut Frame, area: Rect, state: &mut NewCityState) {
    // Split: left = map preview, right = controls
    let chunks = Layout::horizontal([Constraint::Fill(1), Constraint::Length(30)]).split(area);

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
    frame.render_widget(
        MapPreview {
            map: &state.preview_map,
        },
        inner_map,
    );

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

fn render_controls(frame: &mut Frame, area: Rect, state: &mut NewCityState) {
    if area.width < 4 || area.height < 14 {
        return;
    }

    use ratatui::widgets::StatefulWidget;
    use rat_widget::text_input::TextInput;
    use rat_widget::slider::Slider;
    use rat_widget::button::Button;

    let w = area.width;
    let mut row = area.y;

    // City Name
    {
        let is_focused = state.focused_field == NewCityField::CityName;
        frame.buffer_mut().set_string(
            area.x,
            row,
            "City Name:",
            Style::default().fg(Color::Rgb(180, 180, 220)),
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = TextInput::new().style(
            Style::default().fg(Color::Rgb(220, 220, 100)).bg(Color::Rgb(25, 25, 40))
        );
        if is_focused {
            widget = widget.focus_style(Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 60)));
        }
        StatefulWidget::render(widget, rect, frame.buffer_mut(), &mut state.city_name);
        row += 2;
    }

    // Seed field
    {
        let is_focused = state.focused_field == NewCityField::SeedInput;
        frame.buffer_mut().set_string(
            area.x,
            row,
            "Seed (hex):",
            Style::default().fg(Color::Rgb(180, 180, 220)),
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = TextInput::new().style(
            Style::default().fg(Color::Rgb(160, 200, 255)).bg(Color::Rgb(20, 20, 45))
        );
        if is_focused {
            widget = widget.focus_style(Style::default().fg(Color::Black).bg(Color::Rgb(200, 200, 60)));
        }
        StatefulWidget::render(widget, rect, frame.buffer_mut(), &mut state.seed_input);
        row += 2;
    }

    // Water slider
    {
        let is_focused = state.focused_field == NewCityField::WaterSlider;
        let label_str = format!("Water: {}%", state.water_slider.value());
        frame.buffer_mut().set_string(
            area.x,
            row,
            label_str,
            Style::default().fg(Color::Rgb(180, 180, 220)),
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = Slider::new().range((0, 100)).style(Style::default().fg(Color::Rgb(64, 164, 223)).bg(Color::Rgb(15, 15, 25)));
        if is_focused {
            widget = widget.focus_style(Style::default().bg(Color::Rgb(50, 50, 80)));
        }
        StatefulWidget::render(widget, rect, frame.buffer_mut(), &mut state.water_slider);
        row += 2;
    }

    // Trees slider
    {
        let is_focused = state.focused_field == NewCityField::TreesSlider;
        let label_str = format!("Trees: {}%", state.trees_slider.value());
        frame.buffer_mut().set_string(
            area.x,
            row,
            label_str,
            Style::default().fg(Color::Rgb(180, 180, 220)),
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = Slider::new().range((0, 100)).style(Style::default().fg(Color::Rgb(60, 160, 60)).bg(Color::Rgb(15, 15, 25)));
        if is_focused {
            widget = widget.focus_style(Style::default().bg(Color::Rgb(50, 80, 50)));
        }
        StatefulWidget::render(widget, rect, frame.buffer_mut(), &mut state.trees_slider);
        row += 2;
    }

    // Buttons
    let mut render_btn = |field, text, row: u16, state_ref: &mut rat_widget::button::ButtonState| {
        let is_focused = state.focused_field == field;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = Button::new(text).style(Style::default().fg(Color::Rgb(180, 180, 220)).bg(Color::Rgb(8, 8, 18)));
        if is_focused {
            widget = widget.focus_style(Style::default().fg(Color::Black).bg(Color::Rgb(220, 200, 60)).add_modifier(Modifier::BOLD));
        }
        StatefulWidget::render(widget, rect, frame.buffer_mut(), state_ref);
    };

    render_btn(NewCityField::RegenerateBtn, "[Regenerate Map]", row, &mut state.regen_btn);
    row += 2;
    render_btn(NewCityField::StartBtn, "  [▶ Start City]  ", row, &mut state.start_btn);
    row += 2;
    render_btn(NewCityField::BackBtn, "      [Back]      ", row, &mut state.back_btn);
    row += 2;

    if row < area.y + area.height {
        let hint = "↑↓ Focus  ←→ Adjust  Enter Select";
        let trimmed: String = hint.chars().take(w as usize).collect();
        frame.buffer_mut().set_string(
            area.x,
            area.y + area.height - 1,
            &trimmed,
            Style::default().fg(Color::Rgb(80, 80, 100)),
        );
    }
}
