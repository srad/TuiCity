use crate::core::{
    map::{ResourceRole, Tile, TileOverlay, ZoneKind},
    sim::TaxSector,
    tool::Tool,
};
use crate::ui::theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use std::collections::VecDeque;

pub struct InfoPanel {
    pub tile: Tile,
    pub overlay: TileOverlay,
    pub zone: Option<ZoneKind>,
    pub x: usize,
    pub y: usize,
    pub current_tool: Tool,
    pub demand_res: f32,
    pub demand_comm: f32,
    pub demand_ind: f32,
    pub demand_history_res: VecDeque<f32>,
    pub demand_history_comm: VecDeque<f32>,
    pub demand_history_ind: VecDeque<f32>,
    pub power_produced: u32,
    pub power_consumed: u32,
}

/// Map a demand value in [-1, 1] to a block character.
fn demand_block(v: f32) -> char {
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let idx = ((v + 1.0) / 2.0 * 8.0).clamp(0.0, 8.0) as usize;
    BLOCKS[idx]
}

fn sparkline_str(history: &[f32], max_cols: usize) -> String {
    if history.is_empty() {
        return String::new();
    }
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

impl Widget for InfoPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let ui = theme::ui_palette();

        // Background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_char(' ');
                cell.set_bg(ui.panel_window_bg);
            }
        }

        #[allow(unused_assignments)]
        let mut row = area.y;
        let w = area.width as usize;
        let bg = ui.panel_window_bg;

        macro_rules! print_row {
            ($text:expr, $fg:expr) => {
                if row < area.y + area.height {
                    buf.set_string(
                        area.x,
                        row,
                        truncate($text, w),
                        Style::default().fg($fg).bg(bg),
                    );
                    row += 1;
                }
            };
        }

        // Title
        print_row!("── INFO ────────", ui.text_muted);

        // ── RCI demand bars ───────────────────────────────────────────────────
        let bar_w = (w.saturating_sub(4)).min(10);

        let render_demand_row =
            |buf: &mut Buffer, row: u16, label: &str, val: f32, color: Color| {
                if row >= area.y + area.height {
                    return;
                }
                buf.set_string(
                    area.x,
                    row,
                    label,
                    Style::default().fg(ui.text_primary).bg(bg),
                );
                let bar = bar_str(val, bar_w);
                buf.set_string(
                    area.x + 3,
                    row,
                    truncate(&bar, w.saturating_sub(3)),
                    Style::default().fg(color).bg(bg),
                );
            };

        if area.height >= 4 {
            render_demand_row(
                buf,
                row,
                "R:",
                self.demand_res,
                theme::sector_color(TaxSector::Residential),
            );
            row += 1;
            render_demand_row(
                buf,
                row,
                "C:",
                self.demand_comm,
                theme::sector_color(TaxSector::Commercial),
            );
            row += 1;
            render_demand_row(
                buf,
                row,
                "I:",
                self.demand_ind,
                theme::sector_color(TaxSector::Industrial),
            );
            row += 1;
        }

        // ── Demand history sparklines (shown when panel is tall enough) ───────
        let has_history = !self.demand_history_res.is_empty();
        if has_history && area.height >= 10 && row < area.y + area.height {
            let spark_cols = w.saturating_sub(2);
            print_row!("Trend (24m):", ui.text_dim);

            if row < area.y + area.height {
                let spark_r = {
                    let (front, back) = self.demand_history_res.as_slices();
                    let data = if back.is_empty() {
                        front.to_vec()
                    } else {
                        [front, back].concat()
                    };
                    sparkline_str(&data, spark_cols)
                };
                buf.set_string(
                    area.x,
                    row,
                    truncate(&spark_r, w),
                    Style::default()
                        .fg(theme::sector_color(TaxSector::Residential))
                        .bg(bg),
                );
                row += 1;
            }
            if row < area.y + area.height {
                let spark_c = {
                    let (front, back) = self.demand_history_comm.as_slices();
                    let data = if back.is_empty() {
                        front.to_vec()
                    } else {
                        [front, back].concat()
                    };
                    sparkline_str(&data, spark_cols)
                };
                buf.set_string(
                    area.x,
                    row,
                    truncate(&spark_c, w),
                    Style::default()
                        .fg(theme::sector_color(TaxSector::Commercial))
                        .bg(bg),
                );
                row += 1;
            }
            if row < area.y + area.height {
                let spark_i = {
                    let (front, back) = self.demand_history_ind.as_slices();
                    let data = if back.is_empty() {
                        front.to_vec()
                    } else {
                        [front, back].concat()
                    };
                    sparkline_str(&data, spark_cols)
                };
                buf.set_string(
                    area.x,
                    row,
                    truncate(&spark_i, w),
                    Style::default()
                        .fg(theme::sector_color(TaxSector::Industrial))
                        .bg(bg),
                );
                row += 1;
            }
        }

        // Spacer
        row += 1;

        // ── Tile info ─────────────────────────────────────────────────────────
        let pos = format!("({},{})", self.x, self.y);
        print_row!(&pos, ui.text_muted);

        print_row!(self.tile.name(), ui.accent);

        if let Some(zone) = self.zone {
            print_row!(&format!("Zone: {}", zone.label()), ui.text_secondary);
        }
        if self.overlay.water_service > 0 {
            let level = self.overlay.water_service as u16 * 100 / 255;
            print_row!(&format!("Water {}%", level), ui.info);
        } else if matches!(self.tile, Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd)
            || self.tile.is_building()
        {
            print_row!("NO WATER", ui.danger);
        }

        // Power info
        let surplus = self.power_produced as i32 - self.power_consumed as i32;
        let p_color = if surplus >= 0 { ui.success } else { ui.danger };
        print_row!(
            &format!("Pwr: {}/{} MW", self.power_produced, self.power_consumed),
            p_color
        );

        if self.tile.power_role() == ResourceRole::Producer {
            let level = self.overlay.power_level as u16 * 100 / 255;
            print_row!(&format!("Signal: {}% (Src)", level), ui.success);
        } else if self.overlay.is_powered() {
            let level = self.overlay.power_level as u16 * 100 / 255;
            print_row!(&format!("Signal: {}%", level), ui.warning);
        } else if self.tile.receives_power() {
            print_row!("NO POWER", ui.danger);
        }

        if self.overlay.trip_success {
            let mode = self
                .overlay
                .trip_mode
                .map(|mode| mode.label())
                .unwrap_or("Unknown");
            print_row!(
                &format!("Trip {} ({})", mode, self.overlay.trip_cost),
                ui.success
            );
        } else if let Some(failure) = self.overlay.trip_failure {
            print_row!(&format!("Trip {}", failure.label()), ui.danger);
        }

        // Pollution indicator
        if self.overlay.pollution > 10 {
            let pct = self.overlay.pollution as u16 * 100 / 255;
            let level = match self.overlay.pollution {
                0..=50 => "",
                51..=120 => " (Moderate)",
                121..=180 => " (High)",
                _ => " (Severe)",
            };
            let text = format!("Pollut {}%{}", pct, level);
            let color = match self.overlay.pollution {
                0..=50 => ui.text_secondary,
                51..=120 => ui.warning,
                121..=180 => ui.accent,
                _ => ui.danger,
            };
            print_row!(&text, color);
        }

        // Land value
        if self.overlay.land_value > 0 {
            let pct = self.overlay.land_value as u16 * 100 / 255;
            let text = format!("LV {}%", pct);
            let color = if pct >= 60 {
                ui.success
            } else if pct >= 30 {
                ui.warning
            } else {
                ui.text_muted
            };
            print_row!(&text, color);
        }

        // Crime
        if self.overlay.crime > 5 {
            let pct = self.overlay.crime as u16 * 100 / 255;
            let text = format!("Crime {}%", pct);
            let color = if pct >= 60 {
                ui.danger
            } else if pct >= 30 {
                ui.warning
            } else {
                ui.text_secondary
            };
            print_row!(&text, color);
        }

        if self.overlay.traffic > 5 {
            let pct = self.overlay.traffic as u16 * 100 / 255;
            let text = format!("Traffic {}%", pct);
            let color = if pct >= 60 {
                ui.danger
            } else if pct >= 30 {
                ui.warning
            } else {
                ui.text_secondary
            };
            print_row!(&text, color);
        }

        // Tool cost
        if self.current_tool != Tool::Inspect {
            let cost = self.current_tool.cost();
            if cost > 0 {
                let text = format!("Cost: ${}", cost);
                print_row!(&text, ui.text_secondary);
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, widgets::Widget};

    #[test]
    fn info_panel_uses_ascii_labels_for_narrow_rows() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 12));

        InfoPanel {
            tile: Tile::Police,
            overlay: TileOverlay {
                water_service: 128,
                power_level: 128,
                traffic: 128,
                ..TileOverlay::default()
            },
            zone: None,
            x: 1,
            y: 2,
            current_tool: Tool::Inspect,
            demand_res: 0.0,
            demand_comm: 0.0,
            demand_ind: 0.0,
            demand_history_res: VecDeque::new(),
            demand_history_comm: VecDeque::new(),
            demand_history_ind: VecDeque::new(),
            power_produced: 10,
            power_consumed: 8,
        }
        .render(Rect::new(0, 0, 20, 12), &mut buf);

        let rendered = (0..12)
            .map(|y| (0..20).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Water 50%"));
        assert!(rendered.contains("Pwr: 10/8 MW"));
        assert!(rendered.contains("Signal: 50%"));
        assert!(rendered.contains("Traffic 50%"));
    }
}
