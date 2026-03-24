use crate::core::map::{Map, ResourceRole, Tile};
use crate::core::sim::constants::WATER_FALLOFF_PIPE;
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
}

impl SimSystem for WaterSystem {
    fn name(&self) -> &str {
        "Water"
    }

    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        let mut queue = std::collections::VecDeque::new();
        let mut total_capacity = 0;

        // Seed BFS from water producers
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

        // BFS propagation with per-conductor falloff
        while let Some((x, y, level)) = queue.pop_front() {
            if level <= 1 {
                continue;
            }

            for (nx, ny, _tile) in map.neighbors4(x, y) {
                let idx = ny * map.width + nx;
                let underground = map.underground_at(nx, ny);
                let lot_tile = map.surface_lot_tile(nx, ny);

                // Determine role: underground pipe overrides surface role
                let role = if underground.water_pipe {
                    ResourceRole::Conductor { falloff: WATER_FALLOFF_PIPE }
                } else {
                    lot_tile.water_role()
                };

                let falloff = match role {
                    ResourceRole::None => continue,
                    ResourceRole::Producer => continue,
                    ResourceRole::Consumer => {
                        let next_level = level.saturating_sub(1);
                        if next_level > map.overlays[idx].water_service {
                            map.overlays[idx].water_service = next_level;
                        }
                        continue;
                    }
                    ResourceRole::Conductor { falloff } => falloff,
                };

                let next_level = level.saturating_sub(falloff);
                if next_level == 0 || next_level <= map.overlays[idx].water_service {
                    continue;
                }

                map.overlays[idx].water_service = next_level;
                queue.push_back((nx, ny, next_level));
            }
        }

        // Calculate connected demand
        let mut connected_demand = 0u32;
        for y in 0..map.height {
            for x in 0..map.width {
                let tile = map.surface_lot_tile(x, y);
                let demand = tile.water_demand(map.zone_density(x, y));
                if demand == 0 {
                    continue;
                }
                let idx = y * map.width + x;
                if map.overlays[idx].water_service > 0 {
                    connected_demand += demand;
                }
            }
        }

        sim.utilities.water_produced_units = total_capacity;
        sim.utilities.water_consumed_units = connected_demand;

        // NOTE: water_service overlay stores raw BFS signal strength.
        // Water shortage (supply < demand) is communicated via
        // sim.utilities and consumed by the growth system separately.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{ZoneDensity, ZoneKind, ZoneSpec};
    use crate::core::sim::system::SimSystem;
    use crate::core::sim::SimState;

    #[test]
    fn water_pipes_relay_to_adjacent_zones_but_empty_zones_do_not_chain() {
        let mut map = Map::new(6, 1);
        let mut sim = SimState::default();

        map.set(0, 0, Tile::WaterTower);
        map.overlays[0].power_level = 255;
        map.set(1, 0, Tile::ResLow);
        map.set_water_pipe(1, 0, true);
        map.set(2, 0, Tile::ZoneRes);
        map.set_water_pipe(2, 0, true);
        map.set(3, 0, Tile::ZoneRes);
        map.set_water_pipe(3, 0, true);

        WaterSystem.tick(&mut map, &mut sim);

        assert!(map.get_overlay(1, 0).has_water());
        assert!(map.get_overlay(2, 0).has_water());
        assert!(map.get_overlay(3, 0).has_water());
    }

    #[test]
    fn building_without_pipe_receives_but_does_not_relay() {
        let mut map = Map::new(4, 1);
        let mut sim = SimState::default();

        map.set(0, 0, Tile::WaterTower);
        map.overlays[0].power_level = 255;
        // Building without pipe: receives water but doesn't relay it further.
        map.set(1, 0, Tile::ResLow);
        map.set(2, 0, Tile::ZoneRes);
        map.set_water_pipe(2, 0, true);

        WaterSystem.tick(&mut map, &mut sim);

        assert!(map.get_overlay(1, 0).has_water());
        assert_eq!(map.get_overlay(2, 0).water_service, 0);
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
        map.set_water_pipe(1, 0, true);

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
        map.set_water_pipe(2, 0, true);
        map.set(7, 0, Tile::ResHigh);

        WaterSystem.tick(&mut map, &mut sim);

        assert_eq!(sim.utilities.water_consumed_units, 40);
        assert!(map.get_overlay(2, 0).has_water());
        assert_eq!(map.get_overlay(7, 0).water_service, 0);
    }

    // ── ResourceRole classification (water) ───────────────────────────────────

    #[test]
    fn water_role_producers() {
        use crate::core::map::ResourceRole;
        for tile in [Tile::WaterPump, Tile::WaterTower, Tile::WaterTreatment, Tile::Desalination] {
            assert_eq!(tile.water_role(), ResourceRole::Producer, "{:?}", tile);
        }
    }

    #[test]
    fn water_role_consumers() {
        use crate::core::map::ResourceRole;
        for tile in [
            Tile::ResLow, Tile::CommHigh, Tile::IndHeavy, Tile::Hospital,
            Tile::ZoneRes, Tile::ZoneComm, Tile::ZoneInd,
        ] {
            assert_eq!(tile.water_role(), ResourceRole::Consumer, "{:?}", tile);
        }
    }

    #[test]
    fn water_role_none_for_non_participants() {
        use crate::core::map::ResourceRole;
        for tile in [Tile::Road, Tile::Highway, Tile::PowerLine, Tile::Grass] {
            assert_eq!(tile.water_role(), ResourceRole::None, "{:?}", tile);
        }
    }

    #[test]
    fn water_demand_values() {
        assert_eq!(Tile::ResLow.water_demand(None), 6);
        assert_eq!(Tile::ResHigh.water_demand(None), 40);
        assert_eq!(Tile::Stadium.water_demand(None), 50);
        assert_eq!(Tile::ZoneRes.water_demand(Some(ZoneDensity::Dense)), 4);
        assert_eq!(Tile::ZoneRes.water_demand(Some(ZoneDensity::Light)), 2);
        assert_eq!(Tile::ZoneRes.water_demand(None), 0);
        assert_eq!(Tile::Road.water_demand(None), 0);
    }

    // ── Water shortage consistency ────────────────────────────────────────────

    #[test]
    fn water_shortage_scales_all_tiles_uniformly() {
        let mut map = Map::new(10, 1);
        let mut sim = SimState::default();

        // Small water tower
        map.set(0, 0, Tile::WaterTower);
        map.overlays[0].power_level = 255;
        // Lots of heavy consumers via pipe
        for x in 1..10 {
            map.set(x, 0, Tile::IndHeavy);
            map.set_water_pipe(x, 0, true);
        }

        WaterSystem.tick(&mut map, &mut sim);

        // All connected consumers should have similar (low) levels - no huge gap
        let level_1 = map.get_overlay(1, 0).water_service;
        let level_5 = map.get_overlay(5, 0).water_service;
        if level_1 > 0 && level_5 > 0 {
            let gap = level_1.abs_diff(level_5);
            assert!(
                gap < 30,
                "Water levels between adjacent pipes should not have huge gaps ({} vs {})",
                level_1, level_5
            );
        }
    }

    #[test]
    fn water_overlay_shows_raw_signal_during_shortage() {
        use crate::core::map::ResourceRole;
        let mut map = Map::new(10, 1);
        let mut sim = SimState::default();

        map.set(0, 0, Tile::WaterTower);
        map.overlays[0].power_level = 255;
        for x in 1..10 {
            map.set(x, 0, Tile::IndHeavy);
            map.set_water_pipe(x, 0, true);
        }

        WaterSystem.tick(&mut map, &mut sim);

        assert_eq!(Tile::WaterTower.water_role(), ResourceRole::Producer);
        // Overlay keeps raw BFS signal
        let producer_level = map.get_overlay(0, 0).water_service;
        assert_eq!(producer_level, 255, "Producer keeps raw signal level 255");
        // Shortage reported in utilities
        assert!(
            sim.utilities.water_consumed_units > sim.utilities.water_produced_units,
            "Should report water shortage: consumed={} > produced={}",
            sim.utilities.water_consumed_units, sim.utilities.water_produced_units
        );
    }
}
