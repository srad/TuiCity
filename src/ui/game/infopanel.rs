use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use crate::core::{
    map::{Tile, TileOverlay},
    tool::Tool,
};

pub struct InfoPanel {
    pub tile: Tile,
    pub overlay: TileOverlay,
    pub x: usize,
    pub y: usize,
    pub current_tool: Tool,
}

impl Widget for InfoPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(10, 10, 20));
            }
        }

        let mut row = area.y;
        let w = area.width as usize;

        let title = "── INFO ────────";
        buf.set_string(
            area.x,
            row,
            &truncate(title, w),
            Style::default()
                .fg(Color::Rgb(140, 140, 180))
                .bg(Color::Rgb(10, 10, 20)),
        );
        row += 1;

        if row >= area.y + area.height {
            return;
        }

        let pos = format!("({},{})", self.x, self.y);
        buf.set_string(
            area.x,
            row,
            &truncate(&pos, w),
            Style::default()
                .fg(Color::Rgb(160, 160, 160))
                .bg(Color::Rgb(10, 10, 20)),
        );
        row += 1;

        if row >= area.y + area.height {
            return;
        }

        let tile_name = self.tile.name();
        buf.set_string(
            area.x,
            row,
            &truncate(tile_name, w),
            Style::default()
                .fg(Color::Rgb(220, 220, 100))
                .bg(Color::Rgb(10, 10, 20)),
        );
        row += 1;

        if row >= area.y + area.height {
            return;
        }

        if self.overlay.powered {
            buf.set_string(
                area.x,
                row,
                &truncate("⚡ Powered", w),
                Style::default()
                    .fg(Color::Rgb(255, 230, 0))
                    .bg(Color::Rgb(10, 10, 20)),
            );
            row += 1;
        }

        if row >= area.y + area.height {
            return;
        }

        // Show tool cost
        if self.current_tool != Tool::Inspect {
            let cost = self.current_tool.cost();
            if cost > 0 {
                let cost_str = format!("Cost: ${}", cost);
                buf.set_string(
                    area.x,
                    row,
                    &truncate(&cost_str, w),
                    Style::default()
                        .fg(Color::Rgb(180, 220, 180))
                        .bg(Color::Rgb(10, 10, 20)),
                );
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}
