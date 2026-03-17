pub mod gen;
pub mod tile;

pub use tile::{Tile, TileOverlay};

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
                result.push((
                    nx as usize,
                    ny as usize,
                    self.get(nx as usize, ny as usize),
                ));
            }
        }
        result
    }
}
