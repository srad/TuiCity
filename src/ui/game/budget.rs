use crate::app::AppState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, Sparkline, Widget},
};

pub fn render_budget_v2(buf: &mut Buffer, area: Rect, app: &AppState, _screen: &crate::app::screens::InGameScreen) {
    let popup_width = 44;
    let popup_height = 14;
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
    
    let mut row = inner.y;

    // Line 1: Treasury
    buf.set_string(inner.x, row, format!("Treasury: ${}", sim.treasury), Style::default().fg(Color::White));
    row += 1;
    
    // Line 2: Population
    buf.set_string(inner.x, row, format!("Population: {}", sim.population), Style::default().fg(Color::White));
    row += 2;

    // Line 3: Tax Rate
    let tax_str = format!("Tax Rate: {}%", sim.tax_rate);
    buf.set_string(inner.x, row, tax_str, Style::default().fg(Color::Cyan).bold());
    row += 2;

    // Treasury Sparkline
    if !sim.treasury_history.is_empty() {
        buf.set_string(inner.x, row, "Treasury History (24m):", Style::default().fg(Color::DarkGray));
        row += 1;
        let data: Vec<u64> = sim.treasury_history.iter().map(|&v| v.max(0) as u64).collect();
        let sparkline = Sparkline::default()
            .data(&data)
            .style(Style::default().fg(Color::Green));
        sparkline.render(Rect::new(inner.x, row, inner.width, 3), buf);
    }
    
    // Controls
    buf.set_string(inner.x, inner.y + inner.height - 2, "UP/DOWN: tax | ESC/B: close", Style::default().fg(Color::DarkGray));
}
