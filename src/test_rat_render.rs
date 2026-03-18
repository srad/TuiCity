use ratatui::widgets::StatefulWidget;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use rat_widget::text_input::{TextInput, TextInputState};

pub fn render(area: Rect, buf: &mut Buffer, state: &mut TextInputState) {
    let w = TextInput::new();
    StatefulWidget::render(w, area, buf, state);
}
