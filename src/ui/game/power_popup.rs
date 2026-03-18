use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::ui::theme;

pub fn render_power_popup(frame: &mut Frame, popup_area: Rect) {
    let ui = theme::ui_palette();

    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Power Plant Selection ")
        .title_style(Style::default().fg(ui.popup_title))
        .border_style(Style::default().fg(ui.popup_border))
        .style(Style::default().bg(ui.popup_bg));
    frame.render_widget(block, popup_area);

    if popup_area.width >= 5 {
        frame.buffer_mut().set_string(
            popup_area.x + popup_area.width - 5,
            popup_area.y,
            "[X]",
            Style::default().fg(ui.selection_fg).bg(ui.danger).add_modifier(Modifier::BOLD),
        );
    }

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

    put(buf, inner.x, row, "COAL PLANT", Style::default().fg(ui.accent).add_modifier(Modifier::BOLD));
    row += 1;
    put(buf, inner.x, row, "• Capacity: 500 MW | Life: 50y", Style::default().fg(ui.text_secondary));
    row += 1;
    put(buf, inner.x, row, "• Pros: Cheap | Cons: HIGH POLLUTION", Style::default().fg(ui.text_secondary));
    row += 1;
    put(
        buf,
        inner.x,
        row,
        "[ Build Coal ]",
        Style::default().fg(ui.button_fg).bg(ui.button_bg).add_modifier(Modifier::BOLD),
    );
    row += 2;

    put(buf, inner.x, row, "GAS PLANT", Style::default().fg(ui.info).add_modifier(Modifier::BOLD));
    row += 1;
    put(buf, inner.x, row, "• Capacity: 800 MW | Life: 60y", Style::default().fg(ui.text_secondary));
    row += 1;
    put(buf, inner.x, row, "• Pros: Cleaner | Cons: Expensive", Style::default().fg(ui.text_secondary));
    row += 1;
    put(
        buf,
        inner.x,
        row,
        "[ Build Gas  ]",
        Style::default().fg(ui.button_fg).bg(ui.button_bg).add_modifier(Modifier::BOLD),
    );
    row += 2;

    put(
        buf,
        inner.x,
        row,
        "Press ESC or click [X] to cancel.",
        Style::default().fg(ui.text_dim).add_modifier(Modifier::ITALIC),
    );
}
