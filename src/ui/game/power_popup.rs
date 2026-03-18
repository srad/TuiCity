use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, StatefulWidget},
    Frame,
};
use crate::{app::screens::InGameScreen, ui::theme};

pub fn render_power_popup(frame: &mut Frame, area: Rect, screen: &mut InGameScreen) {
    let ui = theme::ui_palette();
    let popup_area = Rect::new(
        area.x + (area.width.saturating_sub(50)) / 2,
        area.y + (area.height.saturating_sub(20)) / 2,
        50,
        20,
    )
    .intersection(area);

    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Power Plant Selection ")
        .title_style(Style::default().fg(ui.popup_title))
        .border_style(Style::default().fg(ui.popup_border))
        .style(Style::default().bg(ui.popup_bg));
    frame.render_widget(block, popup_area);
    render_close_button(frame, popup_area, &mut screen.plant_close_btn);

    let inner = Rect::new(
        popup_area.x + 2,
        popup_area.y + 2,
        popup_area.width.saturating_sub(4),
        popup_area.height.saturating_sub(4),
    );
    let buf = frame.buffer_mut();

    let mut row = inner.y;
    let put = |buf: &mut ratatui::buffer::Buffer, x, y, text: &str, style: Style| {
        buf.set_string(x, y, text, style);
    };

    put(
        buf,
        inner.x,
        row,
        "COAL PLANT",
        Style::default().fg(ui.accent).add_modifier(Modifier::BOLD),
    );
    row += 1;
    put(buf, inner.x, row, "• Capacity: 500 MW | Life: 50y", Style::default().fg(ui.text_secondary));
    row += 1;
    put(buf, inner.x, row, "• Pros: Cheap | Cons: HIGH POLLUTION", Style::default().fg(ui.text_secondary));
    row += 1;

    let coal_btn_area = Rect::new(inner.x, row, 20, 1);
    let coal_btn = rat_widget::button::Button::new(" [ Build Coal ] ")
        .styles(rat_widget::button::ButtonStyle {
            style: Style::default().bg(ui.button_bg).fg(ui.button_fg),
            armed: Some(Style::default().bg(ui.button_armed_bg)),
            ..Default::default()
        });
    coal_btn.render(coal_btn_area, buf, &mut screen.coal_picker_btn);
    row += 2;

    put(
        buf,
        inner.x,
        row,
        "GAS PLANT",
        Style::default().fg(ui.info).add_modifier(Modifier::BOLD),
    );
    row += 1;
    put(buf, inner.x, row, "• Capacity: 800 MW | Life: 60y", Style::default().fg(ui.text_secondary));
    row += 1;
    put(buf, inner.x, row, "• Pros: Cleaner | Cons: Expensive", Style::default().fg(ui.text_secondary));
    row += 1;

    let gas_btn_area = Rect::new(inner.x, row, 20, 1);
    let gas_btn = rat_widget::button::Button::new(" [ Build Gas  ] ")
        .styles(rat_widget::button::ButtonStyle {
            style: Style::default().bg(ui.button_bg).fg(ui.button_fg),
            armed: Some(Style::default().bg(ui.button_armed_bg)),
            ..Default::default()
        });
    gas_btn.render(gas_btn_area, buf, &mut screen.gas_picker_btn);
    row += 2;

    put(
        buf,
        inner.x,
        row,
        "Press ESC or 'E' to cancel.",
        Style::default().fg(ui.text_dim).add_modifier(Modifier::ITALIC),
    );
}

fn render_close_button(
    frame: &mut Frame,
    rect: Rect,
    state: &mut rat_widget::button::ButtonState,
) {
    let ui = theme::ui_palette();
    if rect.width < 5 || rect.height == 0 {
        return;
    }
    let button_area = Rect::new(rect.x + rect.width.saturating_sub(5), rect.y, 5, 1);
    let button = rat_widget::button::Button::new("[X]").styles(rat_widget::button::ButtonStyle {
        style: Style::default().fg(ui.selection_fg).bg(ui.danger),
        focus: Some(Style::default().fg(ui.selection_fg).bg(ui.danger)),
        armed: Some(Style::default().fg(ui.selection_fg).bg(ui.selection_bg)),
        ..Default::default()
    });
    button.render(button_area, frame.buffer_mut(), state);
}
