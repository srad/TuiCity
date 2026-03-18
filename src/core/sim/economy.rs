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

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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

pub fn compute_sector_stats(map: &Map) -> SectorStats {
    let mut stats = SectorStats::default();

    for &tile in &map.tiles {
        if let Some((sector, amount)) = tile_sector_capacity(tile) {
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
}
