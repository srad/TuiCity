use crate::core::map::{Map, Tile, ZoneDensity, ZoneKind};
use crate::core::sim::constants::WATER_FALLOFF_PER_TILE;
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── WaterSystem ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct WaterSystem;

impl WaterSystem {
    fn production_for_tile(map: &Map, x: usize, y: usize, tile: Tile, powered: bool) -> u32 {
        if !powered {
            return 0;
        }
        match tile {
            Tile::WaterPump => {
                let adjacent_water = map
                    .neighbors4(x, y)
                    .into_iter()
                    .any(|(_, _, neighbor)| neighbor == Tile::Water);
                if adjacent_water {
                    200
                } else {
                    120
                }
            }
            Tile::WaterTower => 150,
            Tile::WaterTreatment => 250,
            Tile::Desalination => 320,
            _ => 0,
        }
    }

    fn demand_for_tile(
        tile: Tile,
        zone_kind: Option<ZoneKind>,
        zone_density: Option<ZoneDensity>,
    ) -> u32 {
        match tile {
            Tile::ResLow | Tile::CommLow | Tile::IndLight => 6,
            Tile::ResMed | Tile::CommHigh | Tile::IndHeavy => 18,
            Tile::ResHigh => 40,
            Tile::Police | Tile::Fire | Tile::Hospital => 10,
            Tile::BusDepot | Tile::RailDepot | Tile::SubwayStation => 8,
            Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd => match (zone_kind, zone_density) {
                (Some(_), Some(ZoneDensity::Dense)) => 4,
                (Some(_), _) => 2,
                _ => 0,
            },
            _ => 0,
        }
    }
}

impl SimSystem for WaterSystem {
    fn name(&self) -> &str {
        "Water"
    }

    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        let mut queue = std::collections::VecDeque::new();
        let mut total_capacity = 0;

        for y in 0..map.height {
            for x in 0..map.width {
                let tile = map.get(x, y);
                let production =
                    Self::production_for_tile(map, x, y, tile, map.get_overlay(x, y).is_powered());
                if production == 0 {
                    continue;
                }
                total_capacity += production;
                let idx = y * map.width + x;
                map.overlays[idx].water_service = 255;
                queue.push_back((x, y, 255u8));
            }
        }

        while let Some((x, y, level)) = queue.pop_front() {
            if level <= 1 {
                continue;
            }
            let next_level = level.saturating_sub(WATER_FALLOFF_PER_TILE);

            for (nx, ny, _tile) in map.neighbors4(x, y) {
                let idx = ny * map.width + nx;
                let underground = map.underground_at(nx, ny);
                let lot_tile = map.surface_lot_tile(nx, ny);
                let conductive = underground.water_pipe
                    || lot_tile.is_building()
                    || matches!(
                        lot_tile,
                        Tile::Police
                            | Tile::Fire
                            | Tile::Hospital
                            | Tile::WaterPump
                            | Tile::WaterTower
                            | Tile::WaterTreatment
                            | Tile::Desalination
                    );
                let receivable = conductive || lot_tile.is_zone();
                if !receivable || map.overlays[idx].water_service >= next_level {
                    continue;
                }
                map.overlays[idx].water_service = next_level;
                if conductive {
                    queue.push_back((nx, ny, next_level));
                }
            }
        }

        let mut connected_demand = 0;
        let mut connected_consumers = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                let tile = map.surface_lot_tile(x, y);
                let demand = Self::demand_for_tile(
                    tile,
                    map.effective_zone_kind(x, y),
                    map.zone_density(x, y),
                );
                if demand == 0 {
                    continue;
                }
                let idx = y * map.width + x;
                if map.overlays[idx].water_service > 0 {
                    connected_demand += demand;
                    connected_consumers.push((idx, map.overlays[idx].water_service));
                }
            }
        }

        sim.utilities.water_produced_units = total_capacity;
        sim.utilities.water_consumed_units = connected_demand;

        let shortage_factor = if total_capacity == 0 {
            0.0
        } else if connected_demand > total_capacity {
            total_capacity as f32 / connected_demand as f32
        } else {
            1.0
        };

        for (idx, level) in connected_consumers {
            map.overlays[idx].water_service = (level as f32 * shortage_factor) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{ZoneSpec};
    use crate::core::sim::system::SimSystem;
    use crate::core::sim::SimState;

    #[test]
    fn water_buildings_relay_to_adjacent_zones_but_empty_zones_do_not_chain() {
        let mut map = Map::new(6, 1);
        let mut sim = SimState::default();

        map.set(0, 0, Tile::WaterTower);
        map.overlays[0].power_level = 255;
        map.set(1, 0, Tile::ResLow);
        map.set(2, 0, Tile::ZoneRes);
        map.set(3, 0, Tile::ZoneRes);

        WaterSystem.tick(&mut map, &mut sim);

        assert!(map.get_overlay(1, 0).has_water());
        assert!(map.get_overlay(2, 0).has_water());
        assert_eq!(map.get_overlay(3, 0).water_service, 0);
    }

    #[test]
    fn water_reaches_zone_hidden_under_powerline() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();

        map.set(0, 0, Tile::WaterTower);
        map.overlays[0].power_level = 255;
        map.set_zone_spec(
            1,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
        );
        map.set_power_line(1, 0, true);

        WaterSystem.tick(&mut map, &mut sim);

        assert_eq!(map.get(1, 0), Tile::PowerLine);
        assert_eq!(map.surface_lot_tile(1, 0), Tile::ZoneRes);
        assert!(map.get_overlay(1, 0).has_water());
    }

    #[test]
    fn disconnected_water_load_does_not_reduce_connected_network() {
        let mut map = Map::new(8, 1);
        let mut sim = SimState::default();

        map.set(0, 0, Tile::WaterPump);
        map.overlays[0].power_level = 255;
        map.set_water_pipe(1, 0, true);
        map.set(2, 0, Tile::ResHigh);
        map.set(7, 0, Tile::ResHigh);

        WaterSystem.tick(&mut map, &mut sim);

        assert_eq!(sim.utilities.water_consumed_units, 40);
        assert!(map.get_overlay(2, 0).has_water());
        assert_eq!(map.get_overlay(7, 0).water_service, 0);
    }
}
