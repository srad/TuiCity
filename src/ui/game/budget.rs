use crate::app::AppState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, Widget},
};

pub fn render_budget_v2(buf: &mut Buffer, area: Rect, app: &AppState, _screen: &crate::app::screens::InGameScreen) {
    let popup_width = 40;
    let popup_height = 10;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    Clear.render(popup_area, buf);
    
    let block = Block::default()
        .title(" Budget & Taxes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .bg(Color::Rgb(20, 20, 30));
    block.render(popup_area, buf);

    let inner = Rect::new(popup_area.x + 2, popup_area.y + 2, popup_area.width - 4, popup_area.height - 4);
    
    let engine = app.engine.read().unwrap();
    let sim = &engine.sim;
    
    // Line 1: Treasury
    buf.set_string(inner.x, inner.y, format!("Treasury: ${}", sim.treasury), Style::default().fg(Color::White));
    
    // Line 2: Population
    buf.set_string(inner.x, inner.y + 1, format!("Population: {}", sim.population), Style::default().fg(Color::White));

    // Line 3: Tax Rate
    let tax_str = format!("Tax Rate: {}%", sim.tax_rate);
    buf.set_string(inner.x, inner.y + 3, tax_str, Style::default().fg(Color::Cyan).bold());
    
    // Line 4: Controls
    buf.set_string(inner.x, inner.y + 5, "Use UP/DOWN arrows to change tax", Style::default().fg(Color::DarkGray));
    buf.set_string(inner.x, inner.y + 6, "Press ESC or 'B' to close", Style::default().fg(Color::DarkGray));
}
