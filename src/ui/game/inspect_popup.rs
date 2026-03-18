use crate::{core::{map::Map, sim::economy::{tile_sector_capacity, TaxSector}}, ui::theme};

#[cfg(test)]
use crate::core::map::Tile;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};

#[cfg(test)]
/// Population / job estimate displayed for a tile (display-only approximation).
fn pop_estimate(tile: Tile) -> Option<(i32, &'static str)> {
    tile_sector_capacity(tile).map(|(sector, amount)| {
        let label = match sector {
            TaxSector::Residential => "Pop",
            TaxSector::Commercial | TaxSector::Industrial => "Jobs",
        };
        (amount as i32, label)
    })
}

fn pct(val: u8) -> u8 {
    (val as u16 * 100 / 255) as u8
}

fn lv_label(val: u8) -> &'static str {
    match val {
        0..=85  => "Low",
        86..=170 => "Med",
        _ => "High",
    }
}

// ── Content widget ─────────────────────────────────────────────────────────────

struct InspectContent<'a> {
    pos: (usize, usize),
    map: &'a Map,
}

impl<'a> Widget for InspectContent<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let ui = theme::ui_palette();
        let (x, y) = self.pos;
        if x >= self.map.width || y >= self.map.height {
            return;
        }
        let tile    = self.map.get(x, y);
        let overlay = self.map.get_overlay(x, y);

        let ix = area.x;
        #[allow(unused_assignments)]
        let mut row = area.y;
        let max_row = area.y + area.height.saturating_sub(1);

        macro_rules! put {
            ($text:expr, $style:expr) => {
                if row < max_row {
                    buf.set_string(ix, row, $text, $style);
                    row += 1;
                }
            };
        }

        put!(
            format!("Tile:       {}", tile.name()),
            Style::default().fg(ui.text_primary).bold()
        );

        if let Some((sector, amount)) = tile_sector_capacity(tile) {
            let label = match sector {
                TaxSector::Residential => "Pop",
                TaxSector::Commercial | TaxSector::Industrial => "Jobs",
            };
            put!(
                format!("{:<11} ~{}", format!("{}:", label), amount),
                Style::default().fg(theme::sector_color(sector))
            );
        }

        row += 1; // blank separator

        let (powered_text, powered_color) = if overlay.is_powered() {
            (format!("Power:      {}%", pct(overlay.power_level)), ui.success)
        } else {
            ("Power:      None".to_string(), ui.danger)
        };
        put!(powered_text, Style::default().fg(powered_color));

        if overlay.on_fire {
            put!("ON FIRE!", Style::default().fg(ui.danger).bold());
        }

        row += 1; // blank separator

        put!(
            format!("Land Value: {} ({}%)", lv_label(overlay.land_value), pct(overlay.land_value)),
            Style::default().fg(ui.warning)
        );
        put!(
            format!("Pollution:  {}%", pct(overlay.pollution)),
            Style::default().fg(ui.accent)
        );
        put!(
            format!("Crime:      {}%", pct(overlay.crime)),
            Style::default().fg(ui.danger)
        );
        put!(
            format!("Fire Risk:  {}%", pct(overlay.fire_risk)),
            Style::default().fg(ui.warning)
        );

        let _ = row;
        let hint_row = area.y + area.height.saturating_sub(1);
        buf.set_string(ix, hint_row, "ESC: close", Style::default().fg(ui.text_dim));
    }
}

/// Render inspect content directly into `inner` (the area inside the window border).
pub fn render_inspect_content(buf: &mut Buffer, inner: Rect, pos: (usize, usize), map: &Map) {
    InspectContent { pos, map }.render(inner, buf);
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::Tile;

    #[test]
    fn pop_estimate_residential_buildings() {
        assert_eq!(pop_estimate(Tile::ResLow),  Some((10,  "Pop")));
        assert_eq!(pop_estimate(Tile::ResMed),  Some((50,  "Pop")));
        assert_eq!(pop_estimate(Tile::ResHigh), Some((200, "Pop")));
    }

    #[test]
    fn pop_estimate_commercial_returns_jobs() {
        let (_, label) = pop_estimate(Tile::CommLow).unwrap();
        assert_eq!(label, "Jobs");
    }

    #[test]
    fn pop_estimate_grass_returns_none() {
        assert!(pop_estimate(Tile::Grass).is_none());
    }

    #[test]
    fn pct_converts_u8_to_percent() {
        assert_eq!(pct(0), 0);
        assert_eq!(pct(255), 100);
        assert_eq!(pct(128), 50);
    }

    #[test]
    fn lv_label_bands() {
        assert_eq!(lv_label(0), "Low");
        assert_eq!(lv_label(85), "Low");
        assert_eq!(lv_label(86), "Med");
        assert_eq!(lv_label(170), "Med");
        assert_eq!(lv_label(171), "High");
        assert_eq!(lv_label(255), "High");
    }
}
