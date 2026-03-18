use crate::core::{
    map::{Tile, TileOverlay},
    tool::Tool,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct InfoPanel<'a> {
    pub tile: Tile,
    pub overlay: TileOverlay,
    pub x: usize,
    pub y: usize,
    pub current_tool: Tool,
    pub demand_res: f32,
    pub demand_comm: f32,
    pub demand_ind: f32,
    pub demand_history_res: &'a [f32],
    pub demand_history_comm: &'a [f32],
    pub demand_history_ind: &'a [f32],
}

/// Map a demand value in [-1, 1] to a block character.
fn demand_block(v: f32) -> char {
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let idx = ((v + 1.0) / 2.0 * 8.0).clamp(0.0, 8.0) as usize;
    BLOCKS[idx]
}

fn sparkline_str(history: &[f32], max_cols: usize) -> String {
    if history.is_empty() { return String::new(); }
    let start = history.len().saturating_sub(max_cols);
    history[start..].iter().map(|&v| demand_block(v)).collect()
}

fn bar_str(val: f32, bar_w: usize) -> String {
    let fill = ((val + 1.0) / 2.0 * bar_w as f32).clamp(0.0, bar_w as f32) as usize;
    let mut s = String::with_capacity(bar_w);
    for i in 0..bar_w {
        s.push(if i < fill { '█' } else { '░' });
    }
    s
}

impl<'a> Widget for InfoPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(10, 10, 20));
            }
        }

        #[allow(unused_assignments)]
        let mut row = area.y;
        let w = area.width as usize;
        let bg = Color::Rgb(10, 10, 20);

        macro_rules! print_row {
            ($text:expr, $fg:expr) => {
                if row < area.y + area.height {
                    buf.set_string(area.x, row, truncate($text, w),
                        Style::default().fg($fg).bg(bg));
                    row += 1;
                }
            };
        }

        // Title
        print_row!("── INFO ────────", Color::Rgb(140, 140, 180));

        // ── RCI demand bars ───────────────────────────────────────────────────
        let bar_w = (w.saturating_sub(4)).min(10);

        let render_demand_row = |buf: &mut Buffer, row: u16, label: &str, val: f32, color: Color| {
            if row >= area.y + area.height { return; }
            buf.set_string(area.x, row, label, Style::default().fg(Color::White).bg(bg));
            let bar = bar_str(val, bar_w);
            buf.set_string(area.x + 3, row, truncate(&bar, w.saturating_sub(3)),
                Style::default().fg(color).bg(bg));
        };

        if area.height >= 4 {
            render_demand_row(buf, row, "R:", self.demand_res,  Color::Green);  row += 1;
            render_demand_row(buf, row, "C:", self.demand_comm, Color::Blue);   row += 1;
            render_demand_row(buf, row, "I:", self.demand_ind,  Color::Yellow); row += 1;
        }

        // ── Demand history sparklines (shown when panel is tall enough) ───────
        let has_history = !self.demand_history_res.is_empty();
        if has_history && area.height >= 10 && row < area.y + area.height {
            let spark_cols = w.saturating_sub(2);
            print_row!("Trend (24m):", Color::Rgb(100, 100, 140));

            if row < area.y + area.height {
                let spark_r = sparkline_str(self.demand_history_res,  spark_cols);
                buf.set_string(area.x, row, truncate(&spark_r, w),
                    Style::default().fg(Color::Rgb(80, 200, 80)).bg(bg));
                row += 1;
            }
            if row < area.y + area.height {
                let spark_c = sparkline_str(self.demand_history_comm, spark_cols);
                buf.set_string(area.x, row, truncate(&spark_c, w),
                    Style::default().fg(Color::Rgb(80, 120, 220)).bg(bg));
                row += 1;
            }
            if row < area.y + area.height {
                let spark_i = sparkline_str(self.demand_history_ind,  spark_cols);
                buf.set_string(area.x, row, truncate(&spark_i, w),
                    Style::default().fg(Color::Rgb(200, 180, 60)).bg(bg));
                row += 1;
            }
        }

        // Spacer
        row += 1;

        // ── Tile info ─────────────────────────────────────────────────────────
        let pos = format!("({},{})", self.x, self.y);
        print_row!(&pos, Color::Rgb(130, 130, 130));

        print_row!(self.tile.name(), Color::Rgb(220, 220, 100));

        if self.overlay.powered {
            print_row!("⚡ Powered", Color::Rgb(255, 230, 0));
        }

        // Pollution indicator
        if self.overlay.pollution > 10 {
            let pct = self.overlay.pollution as u16 * 100 / 255;
            let level = match self.overlay.pollution {
                0..=50   => "",
                51..=120 => " (Moderate)",
                121..=180 => " (High)",
                _        => " (Severe)",
            };
            let text = format!("💨 Pollut {}%{}", pct, level);
            let color = match self.overlay.pollution {
                0..=50   => Color::Rgb(160, 200, 160),
                51..=120 => Color::Rgb(210, 180, 80),
                121..=180 => Color::Rgb(220, 120, 60),
                _        => Color::Rgb(220, 60, 60),
            };
            print_row!(&text, color);
        }

        // Land value
        if self.overlay.land_value > 0 {
            let pct = self.overlay.land_value as u16 * 100 / 255;
            let text = format!("🏡 LV {}%", pct);
            let color = if pct >= 60 { Color::Rgb(100, 220, 100) }
                        else if pct >= 30 { Color::Rgb(180, 180, 100) }
                        else { Color::Rgb(160, 100, 100) };
            print_row!(&text, color);
        }

        // Crime
        if self.overlay.crime > 5 {
            let pct = self.overlay.crime as u16 * 100 / 255;
            let text = format!("🚨 Crime {}%", pct);
            let color = if pct >= 60 { Color::Rgb(220, 80, 80) }
                        else if pct >= 30 { Color::Rgb(210, 160, 60) }
                        else { Color::Rgb(160, 200, 160) };
            print_row!(&text, color);
        }

        // Tool cost
        if self.current_tool != Tool::Inspect {
            let cost = self.current_tool.cost();
            if cost > 0 {
                let text = format!("Cost: ${}", cost);
                print_row!(&text, Color::Rgb(180, 220, 180));
            }
        }
        
        let _ = row;
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}
