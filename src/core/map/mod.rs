#![allow(dead_code)]
pub mod gen;
pub mod tile;

pub use tile::{Tile, TileOverlay};

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Map {
    pub tiles: Vec<Tile>,
    pub overlays: Vec<TileOverlay>,
    pub width: usize,
    pub height: usize,
}

impl Map {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            tiles: vec![Tile::default(); w * h],
            overlays: vec![TileOverlay::default(); w * h],
            width: w,
            height: h,
        }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    pub fn get(&self, x: usize, y: usize) -> Tile {
        self.tiles[y * self.width + x]
    }

    pub fn set(&mut self, x: usize, y: usize, tile: Tile) {
        self.tiles[y * self.width + x] = tile;
    }

    pub fn get_overlay(&self, x: usize, y: usize) -> TileOverlay {
        self.overlays[y * self.width + x]
    }

    pub fn set_overlay(&mut self, x: usize, y: usize, overlay: TileOverlay) {
        self.overlays[y * self.width + x] = overlay;
    }

    pub fn neighbors4(&self, x: usize, y: usize) -> Vec<(usize, usize, Tile)> {
        let mut result = Vec::new();
        let ix = x as i32;
        let iy = y as i32;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = ix + dx;
            let ny = iy + dy;
            if self.in_bounds(nx, ny) {
                result.push((nx as usize, ny as usize, self.get(nx as usize, ny as usize)));
            }
        }
        result
    }

    pub fn update_power_grid(&mut self) {
        // This is now handled by PowerSystem in sim/systems/mod.rs
        // We'll keep this as a no-op or proxy if needed, but it's redundant.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_spread_to_zones() {
        let mut map = Map::new(5, 5);
        map.set(0, 0, Tile::PowerPlantCoal);
        map.set(1, 0, Tile::PowerLine);
        map.set(2, 0, Tile::ZoneRes);

        // Use the new PowerSystem for testing
        use crate::core::sim::system::SimSystem;
        use crate::core::sim::systems::PowerSystem;
        use crate::core::sim::{PlantState, SimState};

        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
            },
        );

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(0, 0).is_powered(),
            "Power plant should be powered"
        );
        assert!(
            map.get_overlay(1, 0).is_powered(),
            "Power line should be powered"
        );
        assert!(
            map.get_overlay(2, 0).is_powered(),
            "Zone adjacent to power line should be powered"
        );
    }
}
