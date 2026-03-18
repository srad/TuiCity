use crate::{
    app::screens::{NewCityField, NewCityState},
    ui::game::map_view::MapPreview,
    ui::theme,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders},
    Frame,
};

pub fn render_new_city(frame: &mut Frame, area: Rect, state: &mut NewCityState) {
    let ui = theme::ui_palette();

    // Split: left = map preview, right = controls
    let chunks = Layout::horizontal([Constraint::Fill(1), Constraint::Length(30)]).split(area);

    let map_area = chunks[0];
    let ctrl_area = chunks[1];

    // Map preview (left)
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
            map: &state.preview_map,
        },
        inner_map,
    );

    // Controls (right)
    let ctrl_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui.window_border))
        .title(" NEW CITY ")
        .title_style(Style::default().fg(ui.window_title))
        .style(Style::default().bg(ui.window_bg));
    let inner_ctrl = ctrl_block.inner(ctrl_area);
    frame.render_widget(ctrl_block, ctrl_area);

    render_controls(frame, inner_ctrl, state);
}

fn render_controls(frame: &mut Frame, area: Rect, state: &mut NewCityState) {
    if area.width < 4 || area.height < 14 {
        return;
    }

    let ui = theme::ui_palette();

    use ratatui::widgets::StatefulWidget;
    use rat_widget::text_input::TextInput;
    use rat_widget::slider::Slider;
    use rat_widget::button::Button;

    let w = area.width;
    let mut row = area.y;

    // City Name
    {
        let is_focused = state.focused_field == NewCityField::CityName;
        let is_empty = state.city_name.text().is_empty();
        let label = if is_empty { "City Name: (REQUIRED)" } else { "City Name:" };
        let label_style = if is_empty {
            Style::default().fg(ui.danger)
        } else {
            Style::default().fg(ui.text_secondary)
        };
        
        frame.buffer_mut().set_string(
            area.x,
            row,
            label,
            label_style,
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = TextInput::new().style(Style::default().fg(ui.input_fg).bg(ui.input_bg));
        if is_focused {
            widget = widget.focus_style(Style::default().fg(ui.input_focus_fg).bg(ui.input_focus_bg));
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
            Style::default().fg(ui.text_secondary),
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = TextInput::new().style(Style::default().fg(ui.accent_soft).bg(ui.input_bg));
        if is_focused {
            widget = widget.focus_style(Style::default().fg(ui.input_focus_fg).bg(ui.input_focus_bg));
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
            Style::default().fg(ui.text_secondary),
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = Slider::new()
            .range((0, 100))
            .style(Style::default().fg(ui.slider_water_fg).bg(ui.slider_bg));
        if is_focused {
            widget = widget.focus_style(Style::default().bg(ui.slider_water_focus_bg));
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
            Style::default().fg(ui.text_secondary),
        );
        row += 1;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = Slider::new()
            .range((0, 100))
            .style(Style::default().fg(ui.slider_trees_fg).bg(ui.slider_bg));
        if is_focused {
            widget = widget.focus_style(Style::default().bg(ui.slider_trees_focus_bg));
        }
        StatefulWidget::render(widget, rect, frame.buffer_mut(), &mut state.trees_slider);
        row += 2;
    }

    // Buttons
    let mut render_btn = |field, text, row: u16, state_ref: &mut rat_widget::button::ButtonState| {
        let is_focused = state.focused_field == field;
        let rect = Rect::new(area.x, row, w, 1);
        let mut widget = Button::new(text).style(Style::default().fg(ui.button_fg).bg(ui.button_bg));
        if is_focused {
            widget = widget.focus_style(
                Style::default()
                    .fg(ui.button_focus_fg)
                    .bg(ui.button_focus_bg)
                    .add_modifier(Modifier::BOLD),
            );
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
            Style::default().fg(ui.text_dim),
        );
    }
}
