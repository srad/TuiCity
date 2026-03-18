#![allow(dead_code)]
pub mod gen;
pub mod tile;

pub use tile::{Tile, TileOverlay};

#[derive(Debug, Clone)]
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
        // Clear power grid
        for overlay in self.overlays.iter_mut() {
            overlay.powered = false;
        }

        let mut queue = std::collections::VecDeque::new();

        // Find all power plants
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) == Tile::PowerPlant {
                    queue.push_back((x, y));
                    let idx = y * self.width + x;
                    self.overlays[idx].powered = true;
                }
            }
        }

        // BFS to spread power
        while let Some((x, y)) = queue.pop_front() {
            for (nx, ny, tile) in self.neighbors4(x, y) {
                let n_idx = ny * self.width + nx;
                if !self.overlays[n_idx].powered && tile.power_connects() {
                    self.overlays[n_idx].powered = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        
        // Spread power to adjacent buildings/zones (receivers do not transmit)
        let mut receiver_idxs = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                if self.overlays[idx].powered {
                    for (nx, ny, tile) in self.neighbors4(x, y) {
                        let n_idx = ny * self.width + nx;
                        if !self.overlays[n_idx].powered && tile.receives_power() {
                            receiver_idxs.push(n_idx);
                        }
                    }
                }
            }
        }
        for idx in receiver_idxs {
             self.overlays[idx].powered = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_spread_to_zones() {
        let mut map = Map::new(5, 5);
        map.set(0, 0, Tile::PowerPlant);
        map.set(1, 0, Tile::PowerLine);
        map.set(2, 0, Tile::ZoneRes);
        
        map.update_power_grid();
        
        assert!(map.get_overlay(0, 0).powered, "Power plant should be powered");
        assert!(map.get_overlay(1, 0).powered, "Power line should be powered");
        assert!(map.get_overlay(2, 0).powered, "Zone adjacent to power line should be powered");
    }
}
