use crate::{
    core::{
        map::Map,
        sim::economy::{tile_sector_capacity, TaxSector},
    },
    ui::theme,
};

#[cfg(test)]
use crate::core::map::Tile;
use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

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

#[derive(Clone, Debug)]
pub struct PlantInfo {
    pub age_months: u32,
    pub max_life_months: u32,
    pub capacity_mw: u32,
    pub efficiency: f32,
}

impl PlantInfo {
    pub fn from_state(state: &crate::core::sim::PlantState) -> Self {
        Self {
            age_months: state.age_months,
            max_life_months: state.max_life_months,
            capacity_mw: state.capacity_mw,
            efficiency: state.efficiency,
        }
    }
}

fn pct(val: u8) -> u8 {
    (val as u16 * 100 / 255) as u8
}

fn lv_label(val: u8) -> &'static str {
    match val {
        0..=85 => "Low",
        86..=170 => "Med",
        _ => "High",
    }
}

// ── Content widget ─────────────────────────────────────────────────────────────

struct InspectContent<'a> {
    pos: (usize, usize),
    map: &'a Map,
    plant_info: Option<PlantInfo>,
}

impl<'a> Widget for InspectContent<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let ui = theme::ui_palette();
        let (x, y) = self.pos;
        if x >= self.map.width || y >= self.map.height {
            return;
        }
        let tile = self.map.surface_lot_tile(x, y);
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

        if let Some(zone) = self.map.effective_zone_kind(x, y) {
            put!(
                format!("Zone:       {}", zone.label()),
                Style::default().fg(ui.text_secondary)
            );
        }
        if let Some(density) = self.map.zone_density(x, y) {
            put!(
                format!("Density:    {}", density.label()),
                Style::default().fg(ui.text_secondary)
            );
        }

        if let Some(ref info) = self.plant_info {
            let eff_pct = (info.efficiency * 100.0) as u8;
            let remaining = info.max_life_months.saturating_sub(info.age_months);
            let eff_color = if info.efficiency < 1.0 {
                ui.warning
            } else {
                ui.success
            };
            put!(
                format!("Capacity:   {} MW", info.capacity_mw),
                Style::default().fg(ui.text_secondary)
            );
            put!(
                format!("Efficiency: {}%", eff_pct),
                Style::default().fg(eff_color)
            );
            put!(
                format!("Age:       {}/{} mo", remaining, info.max_life_months),
                Style::default().fg(ui.text_secondary)
            );
        }

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
            (
                format!("Power:      {}%", pct(overlay.power_level)),
                ui.success,
            )
        } else {
            ("Power:      None".to_string(), ui.danger)
        };
        put!(powered_text, Style::default().fg(powered_color));
        put!(
            if overlay.has_water() {
                format!("Water:      {}%", pct(overlay.water_service))
            } else {
                "Water:      None".to_string()
            },
            Style::default().fg(if overlay.has_water() {
                ui.info
            } else {
                ui.danger
            })
        );
        if overlay.trip_success {
            let mode = overlay
                .trip_mode
                .map(|mode| mode.label())
                .unwrap_or("Unknown");
            put!(
                format!("Trips:      {} ({})", mode, overlay.trip_cost),
                Style::default().fg(ui.success)
            );
        } else if let Some(failure) = overlay.trip_failure {
            put!(
                format!("Trips:      {}", failure.label()),
                Style::default().fg(ui.danger)
            );
        }

        if overlay.on_fire {
            put!("ON FIRE!", Style::default().fg(ui.danger).bold());
        }

        row += 1; // blank separator

        put!(
            format!(
                "Land Value: {} ({}%)",
                lv_label(overlay.land_value),
                pct(overlay.land_value)
            ),
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
            format!("Traffic:    {}%", pct(overlay.traffic)),
            Style::default().fg(ui.info)
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
pub fn render_inspect_content(
    buf: &mut Buffer,
    inner: Rect,
    pos: (usize, usize),
    map: &Map,
    plant_info: Option<PlantInfo>,
) {
    InspectContent {
        pos,
        map,
        plant_info,
    }
    .render(inner, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::Tile;

    #[test]
    fn pop_estimate_residential_buildings() {
        assert_eq!(pop_estimate(Tile::ResLow), Some((10, "Pop")));
        assert_eq!(pop_estimate(Tile::ResMed), Some((50, "Pop")));
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
