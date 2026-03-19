use crate::ui::{theme, view::NewsTickerViewModel};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
};

pub fn render_news_ticker(area: Rect, buf: &mut Buffer, ticker: &NewsTickerViewModel) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let ui = theme::ui_palette();
    for x in area.x..area.x + area.width {
        let cell = buf.cell_mut((x, area.y)).unwrap();
        cell.set_char(' ');
        cell.set_bg(ui.news_ticker_bg);
        cell.set_fg(ui.news_ticker_text);
    }

    let label = if ticker.is_alerting {
        " ALERT "
    } else {
        " NEWS "
    };
    let label_fg = if ticker.is_alerting {
        ui.news_ticker_alert
    } else {
        ui.news_ticker_label_fg
    };
    let label_bg = if ticker.is_alerting {
        ui.danger
    } else {
        ui.news_ticker_label_bg
    };
    buf.set_string(
        area.x,
        area.y,
        label,
        Style::default()
            .fg(label_fg)
            .bg(label_bg)
            .add_modifier(Modifier::BOLD),
    );

    let available = area.width.saturating_sub(label.len() as u16 + 1) as usize;
    if available == 0 {
        return;
    }

    let text = marquee_window(&ticker.full_text, ticker.scroll_offset, available);
    buf.set_string(
        area.x + label.len() as u16 + 1,
        area.y,
        &text,
        Style::default()
            .fg(if ticker.is_alerting {
                ui.news_ticker_alert
            } else {
                ui.news_ticker_text
            })
            .bg(ui.news_ticker_bg),
    );
}

fn marquee_window(text: &str, offset: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let source = if text.is_empty() {
        "City desk warming up.   "
    } else {
        text
    };
    let chars: Vec<char> = source.chars().collect();
    if chars.is_empty() {
        return " ".repeat(width);
    }
    (0..width)
        .map(|idx| chars[(offset + idx) % chars.len()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marquee_wraps_text() {
        let window = marquee_window("ABC", 2, 5);
        assert_eq!(window, "CABCA");
    }

    #[test]
    fn render_news_ticker_writes_label_and_text() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
        render_news_ticker(
            Rect::new(0, 0, 30, 1),
            &mut buf,
            &NewsTickerViewModel {
                full_text: "hello world".to_string(),
                scroll_offset: 0,
                is_alerting: false,
            },
        );

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "N");
        assert_eq!(buf.cell((7, 0)).unwrap().symbol(), "h");
    }
}
