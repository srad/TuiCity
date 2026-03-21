use crate::core::map::{Map, Tile};
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── PollutionSystem ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PollutionSystem;
impl SimSystem for PollutionSystem {
    fn name(&self) -> &str {
        "Pollution"
    }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        // Reset
        for o in map.overlays.iter_mut() {
            o.pollution = 0;
        }

        // Collect industrial sources
        let mut sources: Vec<(usize, usize, u8)> = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                let strength: u8 = match map.get(x, y) {
                    Tile::IndHeavy => 200,
                    Tile::IndLight => 120,
                    Tile::PowerPlantCoal => 250,
                    Tile::PowerPlantGas => 80,
                    Tile::Highway => 35,
                    Tile::Road | Tile::RoadPowerLine => map.get_overlay(x, y).traffic / 4,
                    _ => 0,
                };
                if strength > 0 {
                    sources.push((x, y, strength));
                }
            }
        }

        // Radial diffusion with distance falloff
        use crate::core::sim::constants::POLLUTION_RADIUS;
        let radius_sq = (POLLUTION_RADIUS * POLLUTION_RADIUS) as f32;
        for (sx, sy, strength) in sources {
            for dy in -POLLUTION_RADIUS..=POLLUTION_RADIUS {
                for dx in -POLLUTION_RADIUS..=POLLUTION_RADIUS {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    if dist_sq > radius_sq {
                        continue;
                    }
                    let nx = sx as i32 + dx;
                    let ny = sy as i32 + dy;
                    if !map.in_bounds(nx, ny) {
                        continue;
                    }
                    let falloff = 1.0 - (dist_sq / radius_sq);
                    let amount = (strength as f32 * falloff) as u8;
                    let idx = ny as usize * map.width + nx as usize;
                    map.overlays[idx].pollution =
                        map.overlays[idx].pollution.saturating_add(amount);
                }
            }
        }

        // Parks scrub nearby pollution (radius 3, -20 per tile)
        let mut park_scrubs: Vec<(usize, usize)> = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) == Tile::Park {
                    park_scrubs.push((x, y));
                }
            }
        }
        for (px, py) in park_scrubs {
            for dy in -3_i32..=3 {
                for dx in -3_i32..=3 {
                    let nx = px as i32 + dx;
                    let ny = py as i32 + dy;
                    if !map.in_bounds(nx, ny) {
                        continue;
                    }
                    let idx = ny as usize * map.width + nx as usize;
                    map.overlays[idx].pollution = map.overlays[idx].pollution.saturating_sub(20);
                }
            }
        }
    }
}
