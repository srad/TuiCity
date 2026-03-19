use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::{
    app::ClickArea,
    ui::view::{ConfirmDialogButtonRole, ConfirmDialogViewModel},
};

pub(crate) fn render_confirm_dialog(
    frame: &mut Frame,
    area: Rect,
    dialog: &ConfirmDialogViewModel,
) -> Vec<ClickArea> {
    let ui = crate::ui::theme::ui_palette();
    let max_width = area.width.saturating_sub(4).max(18);
    let longest_line = dialog
        .buttons
        .iter()
        .map(|button| button.label.chars().count())
        .chain([dialog.title.chars().count(), dialog.message.chars().count()])
        .max()
        .unwrap_or(18) as u16;
    let width = longest_line.saturating_add(4).min(max_width).max(18);
    let height = (dialog.buttons.len() as u16 + 4)
        .min(area.height.saturating_sub(2).max(5))
        .max(5);
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(dialog.title.as_str())
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.popup_bg)),
        rect,
    );

    let inner = Rect::new(
        rect.x + 1,
        rect.y + 1,
        rect.width.saturating_sub(2),
        rect.height.saturating_sub(2),
    );
    let mut hits = Vec::new();
    if inner.width == 0 || inner.height == 0 {
        return hits;
    }

    frame.buffer_mut().set_string(
        inner.x,
        inner.y,
        format!(
            "{:<width$}",
            truncate(&dialog.message, inner.width as usize),
            width = inner.width as usize
        ),
        Style::default()
            .fg(ui.text_primary)
            .bg(ui.popup_bg)
            .add_modifier(Modifier::BOLD),
    );

    for (index, button) in dialog.buttons.iter().enumerate() {
        let y = inner.y + 2 + index as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let style = if index == dialog.selected {
            Style::default()
                .fg(ui.selection_fg)
                .bg(button_bg(button.role, true))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(ui.button_fg)
                .bg(button_bg(button.role, false))
        };
        frame.buffer_mut().set_string(
            inner.x,
            y,
            format!(
                "{:<width$}",
                truncate(&button.label, inner.width as usize),
                width = inner.width as usize
            ),
            style,
        );
        hits.push(ClickArea {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        });
    }

    hits
}

fn button_bg(role: ConfirmDialogButtonRole, selected: bool) -> ratatui::style::Color {
    let ui = crate::ui::theme::ui_palette();
    match (role, selected) {
        (_, true) => ui.selection_bg,
        (ConfirmDialogButtonRole::Cancel, false) => ui.button_bg,
        (ConfirmDialogButtonRole::Alternate, false) => ui.button_bg,
        (ConfirmDialogButtonRole::Accept, false) => ui.button_bg,
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
