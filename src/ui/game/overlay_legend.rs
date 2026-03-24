use crate::ui::theme::{self, OverlayMode};
use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

/// Renders a heatmap legend for the active overlay mode.
/// Takes 3 rows: a title/divider, a gradient color bar, and Low/High edge labels.
pub struct OverlayLegend {
    pub mode: OverlayMode,
}

impl Widget for OverlayLegend {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let Some(info) = theme::overlay_legend_info(self.mode) else {
            return;
        };
        let ui = theme::ui_palette();
        let bg = ui.panel_window_bg;
        let w = area.width as usize;
        let mut row = area.y;

        // Title row
        if row < area.y + area.height {
            let title = format!("── {} ──", info.title);
            let text: String = title.chars().take(w).collect();
            buf.set_string(area.x, row, text, Style::default().fg(ui.text_muted).bg(bg));
            row += 1;
        }

        // Gradient bar: each cell gets a background color from the heatmap
        if row < area.y + area.height {
            let bar_w = w;
            let denom = (bar_w.saturating_sub(1)).max(1);
            for i in 0..bar_w {
                let val = (i * 255 / denom).min(255) as u8;
                let color = theme::lerp_color3(val, info.low, info.mid, info.high);
                if let Some(cell) = buf.cell_mut((area.x + i as u16, row)) {
                    cell.set_char(' ');
                    cell.set_bg(color);
                }
            }
            row += 1;
        }

        // Edge labels: low_label on the left, high_label on the right
        if row < area.y + area.height {
            let low = info.low_label;
            let high = info.high_label;
            let right_x = area.x + area.width.saturating_sub(high.len() as u16);
            buf.set_string(area.x, row, low, Style::default().fg(ui.text_dim).bg(bg));
            // Only draw high label if it doesn't overlap low label
            if right_x > area.x + low.len() as u16 {
                buf.set_string(right_x, row, high, Style::default().fg(ui.text_dim).bg(bg));
            }
        }
    }
}
