use crate::core::map::{Map, Tile};
use crate::core::sim::util::for_each_in_radius;
use crate::core::sim::constants::{
    FIRE_RISK_DEFAULT, FIRE_RISK_IND_HEAVY, FIRE_RISK_IND_LIGHT_OR_COAL,
    FIRE_RISK_RES_HIGH_OR_COMM_HIGH_OR_GAS, FIRE_RISK_RES_LOW, FIRE_RISK_RES_MED_OR_COMM_LOW,
    FIRE_RISK_REDUCTION, FIRE_STATION_RADIUS,
};
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── FireSystem ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct FireSystem;
impl SimSystem for FireSystem {
    fn name(&self) -> &str {
        "Fire"
    }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        // Baseline fire risk (higher in dense/industrial areas)
        for i in 0..map.tiles.len() {
            map.overlays[i].fire_risk = match map.tiles[i] {
                Tile::IndHeavy => FIRE_RISK_IND_HEAVY,
                Tile::IndLight | Tile::PowerPlantCoal => FIRE_RISK_IND_LIGHT_OR_COAL,
                Tile::PowerPlantGas | Tile::ResHigh | Tile::CommHigh => {
                    FIRE_RISK_RES_HIGH_OR_COMM_HIGH_OR_GAS
                }
                Tile::ResMed | Tile::CommLow => FIRE_RISK_RES_MED_OR_COMM_LOW,
                Tile::ResLow => FIRE_RISK_RES_LOW,
                _ => FIRE_RISK_DEFAULT,
            };
        }

        // Fire stations reduce fire risk within radius (up to -FIRE_RISK_REDUCTION)
        let stations: Vec<(usize, usize)> = (0..map.height)
            .flat_map(|y| (0..map.width).map(move |x| (x, y)))
            .filter(|&(x, y)| map.get(x, y) == Tile::Fire)
            .collect();
        for (sx, sy) in stations {
            let mut reductions: Vec<(usize, u8)> = Vec::new();
            for_each_in_radius(map, sx, sy, FIRE_STATION_RADIUS, |_nx, _ny, idx, falloff| {
                reductions.push((idx, (FIRE_RISK_REDUCTION * falloff) as u8));
            });
            for (idx, reduction) in reductions {
                map.overlays[idx].fire_risk = map.overlays[idx].fire_risk.saturating_sub(reduction);
            }
        }
    }
}
