use super::{Map, Tile};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

pub struct GenParams {
    pub water_pct: u8,
    pub trees_pct: u8,
    pub seed: u64,
    pub width: usize,
    pub height: usize,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            water_pct: 20,
            trees_pct: 30,
            seed: 42,
            width: 128,
            height: 128,
        }
    }
}

// ── Classic 2-D Perlin noise ──────────────────────────────────────────────────

struct Perlin {
    perm: [u8; 512],
}

impl Perlin {
    fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut p: Vec<u8> = (0u8..=255).collect();
        p.shuffle(&mut rng);
        let mut perm = [0u8; 512];
        perm[..256].copy_from_slice(&p);
        perm[256..].copy_from_slice(&p);
        Self { perm }
    }

    #[inline]
    fn fade(t: f32) -> f32 {
        // Ken Perlin's improved fade curve
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    #[inline]
    fn lerp(t: f32, a: f32, b: f32) -> f32 {
        a + t * (b - a)
    }

    #[inline]
    fn grad(hash: u8, x: f32, y: f32) -> f32 {
        // 8 gradient directions in 2-D
        match hash & 7 {
            0 => x + y,
            1 => -x + y,
            2 => x - y,
            3 => -x - y,
            4 => x,
            5 => -x,
            6 => y,
            _ => -y,
        }
    }

    fn noise(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - xi as f32;
        let yf = y - yi as f32;

        let xi = (xi & 255) as usize;
        let yi = (yi & 255) as usize;

        let u = Self::fade(xf);
        let v = Self::fade(yf);

        // Hash the four corners
        let aa = self.perm[self.perm[xi] as usize + yi];
        let ab = self.perm[self.perm[xi] as usize + yi + 1];
        let ba = self.perm[self.perm[xi + 1] as usize + yi];
        let bb = self.perm[self.perm[xi + 1] as usize + yi + 1];

        let x1 = Self::lerp(u, Self::grad(aa, xf, yf), Self::grad(ba, xf - 1.0, yf));
        let x2 = Self::lerp(
            u,
            Self::grad(ab, xf, yf - 1.0),
            Self::grad(bb, xf - 1.0, yf - 1.0),
        );
        Self::lerp(v, x1, x2)
    }

    /// Fractional Brownian Motion — stacks `octaves` noise layers.
    fn fbm(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut value = 0.0f32;
        let mut amplitude = 1.0f32;
        let mut frequency = 1.0f32;
        let mut max_amp = 0.0f32;
        for _ in 0..octaves {
            value += self.noise(x * frequency, y * frequency) * amplitude;
            max_amp += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }
        value / max_amp // normalised to roughly [-1, 1]
    }
}

// ── Map generation ────────────────────────────────────────────────────────────

pub fn generate(p: &GenParams) -> Map {
    let w = p.width;
    let h = p.height;
    let mut map = Map::new(w, h);

    // Two independent noise generators:
    //   elevation — determines water / land boundary
    //   forest    — determines tree patches on land (independent shape)
    let elev_noise = Perlin::new(p.seed);
    let forest_noise = Perlin::new(p.seed.wrapping_add(0x9e3779b97f4a7c15));

    // Scale: ~0.025 → feature width ≈ 40 tiles on a 128-tile map.
    // A second, slightly larger scale adds continental-scale variation.
    let base_scale = 0.025_f32;

    let mut elevation: Vec<f32> = Vec::with_capacity(w * h);
    let mut forest: Vec<f32> = Vec::with_capacity(w * h);

    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 * base_scale;
            let fy = y as f32 * base_scale;

            // 6-octave fBm for elevation gives coasts + inland detail
            let e = elev_noise.fbm(fx, fy, 6);
            elevation.push(e);

            // 4-octave fBm at a slightly tighter scale for distinct forest patches
            let f = forest_noise.fbm(fx * 1.7, fy * 1.7, 4);
            forest.push(f);
        }
    }

    // ── Derive water threshold from the elevation percentile ─────────────────
    let mut sorted_elev = elevation.clone();
    sorted_elev.sort_by(|a: &f32, b: &f32| a.partial_cmp(b).unwrap());
    let water_idx = (w * h * p.water_pct as usize / 100).min(w * h - 1);
    let water_thresh = sorted_elev[water_idx];

    // ── Derive forest threshold from land-only forest values ──────────────────
    // Collect the forest noise values for every tile that will be land.
    let mut land_forest: Vec<f32> = elevation
        .iter()
        .zip(forest.iter())
        .filter(|(&e, _)| e > water_thresh)
        .map(|(_, &f)| f)
        .collect();
    land_forest.sort_by(|a: &f32, b: &f32| a.partial_cmp(b).unwrap());

    // We want the top `trees_pct`% of land tiles to become trees.
    // So threshold = the value at the (100 - trees_pct)% mark.
    let tree_cut = land_forest
        .len()
        .saturating_sub(land_forest.len() * p.trees_pct as usize / 100);
    let forest_thresh = if land_forest.is_empty() {
        f32::MAX
    } else {
        land_forest[tree_cut.min(land_forest.len() - 1)]
    };

    // ── Assign tiles ──────────────────────────────────────────────────────────
    for y in 0..h {
        for x in 0..w {
            let e = elevation[y * w + x];
            let f = forest[y * w + x];

            let tile = if e <= water_thresh {
                Tile::Water
            } else if f >= forest_thresh {
                Tile::Trees
            } else {
                Tile::Grass
            };

            map.set(x, y, tile);
        }
    }

    map
}
