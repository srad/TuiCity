use crate::core::map::{Map, Tile};
use crate::core::sim::util::for_each_in_radius;
use crate::core::sim::constants::{
    CRIME_BASE_DEFAULT, CRIME_BASE_HIGH_DENSITY, CRIME_BASE_MED_DENSITY, CRIME_BASE_RES_LOW,
    POLICE_CRIME_REDUCTION, POLICE_RADIUS,
};
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── PoliceSystem ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PoliceSystem;
impl SimSystem for PoliceSystem {
    fn name(&self) -> &str {
        "Police"
    }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        // Baseline crime (higher in dense zones)
        for i in 0..map.tiles.len() {
            map.overlays[i].crime = match map.tiles[i] {
                Tile::ResHigh | Tile::CommHigh | Tile::IndHeavy => CRIME_BASE_HIGH_DENSITY,
                Tile::ResMed | Tile::CommLow | Tile::IndLight => CRIME_BASE_MED_DENSITY,
                Tile::ResLow => CRIME_BASE_RES_LOW,
                _ => CRIME_BASE_DEFAULT,
            };
        }

        // Police stations reduce crime within radius (up to -POLICE_CRIME_REDUCTION)
        let stations: Vec<(usize, usize)> = (0..map.height)
            .flat_map(|y| (0..map.width).map(move |x| (x, y)))
            .filter(|&(x, y)| map.get(x, y) == Tile::Police)
            .collect();
        for (sx, sy) in stations {
            let mut reductions: Vec<(usize, u8)> = Vec::new();
            for_each_in_radius(map, sx, sy, POLICE_RADIUS, |_nx, _ny, idx, falloff| {
                reductions.push((idx, (POLICE_CRIME_REDUCTION * falloff) as u8));
            });
            for (idx, reduction) in reductions {
                map.overlays[idx].crime = map.overlays[idx].crime.saturating_sub(reduction);
            }
        }
    }
}
