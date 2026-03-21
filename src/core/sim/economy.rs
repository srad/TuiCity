use crate::core::map::{Map, Tile};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaxSector {
    Residential,
    Commercial,
    Industrial,
}

impl TaxSector {
    pub fn label(self) -> &'static str {
        match self {
            TaxSector::Residential => "Residential",
            TaxSector::Commercial => "Commercial",
            TaxSector::Industrial => "Industrial",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaxRates {
    pub residential: u8,
    pub commercial: u8,
    pub industrial: u8,
}

impl Default for TaxRates {
    fn default() -> Self {
        Self {
            residential: 9,
            commercial: 9,
            industrial: 9,
        }
    }
}

impl TaxRates {
    pub fn get(self, sector: TaxSector) -> u8 {
        match sector {
            TaxSector::Residential => self.residential,
            TaxSector::Commercial => self.commercial,
            TaxSector::Industrial => self.industrial,
        }
    }

    pub fn set(&mut self, sector: TaxSector, rate: u8) {
        let rate = rate.clamp(0, 100);
        match sector {
            TaxSector::Residential => self.residential = rate,
            TaxSector::Commercial => self.commercial = rate,
            TaxSector::Industrial => self.industrial = rate,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SectorStats {
    pub residential_population: u64,
    pub commercial_jobs: u64,
    pub industrial_jobs: u64,
}

pub fn tile_sector_capacity(tile: Tile) -> Option<(TaxSector, u64)> {
    match tile {
        Tile::ResLow => Some((TaxSector::Residential, 10)),
        Tile::ResMed => Some((TaxSector::Residential, 50)),
        Tile::ResHigh => Some((TaxSector::Residential, 200)),
        Tile::CommLow => Some((TaxSector::Commercial, 5)),
        Tile::CommHigh => Some((TaxSector::Commercial, 20)),
        Tile::IndLight => Some((TaxSector::Industrial, 10)),
        Tile::IndHeavy => Some((TaxSector::Industrial, 30)),
        _ => None,
    }
}

fn count_tiles_in_radius<F>(map: &Map, cx: i32, cy: i32, radius: i32, pred: F) -> usize
where
    F: Fn(Tile) -> bool,
{
    let w = map.width as i32;
    let h = map.height as i32;
    let mut count = 0;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx.abs() + dy.abs() > radius {
                continue;
            }
            let nx = cx + dx;
            let ny = cy + dy;
            if nx >= 0
                && nx < w
                && ny >= 0
                && ny < h
                && pred(map.tiles[ny as usize * map.width + nx as usize])
            {
                count += 1;
            }
        }
    }
    count
}

pub fn compute_sector_stats(map: &Map) -> SectorStats {
    let mut stats = SectorStats::default();

    for i in 0..map.tiles.len() {
        let tile = map.tiles[i];
        if let Some((sector, base_amount)) = tile_sector_capacity(tile) {
            let amount = match sector {
                TaxSector::Residential => {
                    let crime = map.overlays[i].crime;
                    let crime_penalty = 1.0 - (crime as f32 / 255.0) * 0.7;
                    (base_amount as f32 * crime_penalty) as u64
                }
                TaxSector::Commercial => {
                    let x = (i as i32) % map.width as i32;
                    let y = (i as i32) / map.width as i32;
                    let industrial_tiles = count_tiles_in_radius(map, x, y, 5, |t| {
                        matches!(t, Tile::IndLight | Tile::IndHeavy)
                    });
                    let supply_factor = if industrial_tiles > 0 { 1.0 } else { 0.3 };
                    (base_amount as f32 * supply_factor) as u64
                }
                _ => base_amount,
            };
            match sector {
                TaxSector::Residential => stats.residential_population += amount,
                TaxSector::Commercial => stats.commercial_jobs += amount,
                TaxSector::Industrial => stats.industrial_jobs += amount,
            }
        }
    }

    stats
}

pub fn annual_tax_from_base(base: u64, rate: u8) -> i64 {
    let tax_per_unit = (rate.clamp(0, 100) as f32 / 9.0) * 5.0;
    (base as f32 * tax_per_unit) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::TileOverlay;

    #[test]
    fn tax_rates_default_to_neutral() {
        let rates = TaxRates::default();
        assert_eq!(rates.residential, 9);
        assert_eq!(rates.commercial, 9);
        assert_eq!(rates.industrial, 9);
    }

    #[test]
    fn annual_tax_from_base_matches_existing_formula() {
        assert_eq!(annual_tax_from_base(1_000, 9), 5_000);
    }

    #[test]
    fn crime_at_max_reduces_residential_population_to_30_pct() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::ResHigh);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                crime: 255,
                ..TileOverlay::default()
            },
        );
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.residential_population, 200 * 3 / 10);
    }

    #[test]
    fn crime_at_zero_leaves_residential_population_unchanged() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::ResHigh);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                crime: 0,
                ..TileOverlay::default()
            },
        );
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.residential_population, 200);
    }

    #[test]
    fn crime_does_not_affect_commercial_jobs() {
        let mut map = Map::new(2, 1);
        map.set(0, 0, Tile::CommHigh);
        map.set(1, 0, Tile::IndHeavy);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                crime: 255,
                ..TileOverlay::default()
            },
        );
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 20);
    }

    #[test]
    fn crime_does_not_affect_industrial_jobs() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::IndHeavy);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                crime: 255,
                ..TileOverlay::default()
            },
        );
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.industrial_jobs, 30);
    }

    #[test]
    fn crime_penalty_is_consistent_with_growth_formula() {
        let mut map = Map::new(2, 1);
        map.set(0, 0, Tile::ResLow);
        map.set(1, 0, Tile::ResMed);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                crime: 0,
                ..TileOverlay::default()
            },
        );
        map.set_overlay(
            1,
            0,
            TileOverlay {
                crime: 255,
                ..TileOverlay::default()
            },
        );
        let stats = compute_sector_stats(&map);
        let expected = 10 + (50 * 3 / 10);
        assert_eq!(stats.residential_population, expected);
    }

    #[test]
    fn industrial_supply_nearby_boosts_commercial_capacity() {
        let mut map = Map::new(3, 3);
        map.set(0, 0, Tile::CommHigh);
        map.set(1, 0, Tile::IndHeavy);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 20);
    }

    #[test]
    fn industrial_supply_no_industry_reduces_commercial_to_30_pct() {
        let mut map = Map::new(10, 10);
        map.set(5, 5, Tile::CommHigh);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 20 * 3 / 10);
    }

    #[test]
    fn industrial_supply_does_not_affect_industrial_jobs() {
        let mut map = Map::new(3, 3);
        map.set(0, 0, Tile::IndHeavy);
        map.set(1, 0, Tile::CommHigh);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.industrial_jobs, 30);
    }

    #[test]
    fn industrial_supply_does_not_affect_residential() {
        let mut map = Map::new(10, 10);
        map.set(5, 5, Tile::ResHigh);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.residential_population, 200);
    }

    #[test]
    fn industrial_supply_affects_comm_low() {
        let mut map = Map::new(2, 1);
        map.set(0, 0, Tile::CommLow);
        map.set(1, 0, Tile::IndHeavy);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 5);
    }

    #[test]
    fn industrial_supply_comm_low_no_industry() {
        let mut map = Map::new(10, 10);
        map.set(5, 5, Tile::CommLow);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 1);
    }

    #[test]
    fn industrial_supply_comm_high_on_map_corner() {
        let mut map = Map::new(10, 10);
        map.set(0, 0, Tile::CommHigh);
        map.set(1, 0, Tile::IndHeavy);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 20);
    }

    #[test]
    fn industrial_supply_comm_high_no_industry_on_corner_map() {
        let mut map = Map::new(10, 10);
        map.set(0, 0, Tile::CommHigh);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 6);
    }

    #[test]
    fn industrial_supply_on_1x1_map_is_zero() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::CommHigh);
        let stats = compute_sector_stats(&map);
        assert_eq!(stats.commercial_jobs, 6);
    }
}
