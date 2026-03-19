use super::{economy::compute_sector_stats, SimState};
use crate::core::map::TransportTile;
use crate::core::map::{Map, Tile, ZoneDensity, ZoneKind, ZoneSpec};
use rand::Rng;

pub fn tick_growth(map: &mut Map, sim: &mut SimState) {
    let mut rng = rand::thread_rng();
    let w = map.width;
    let h = map.height;

    let mut changes: Vec<(usize, usize, Tile)> = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let zone_spec = map.effective_zone_spec(x, y);
            let tile = current_growth_tile(map, x, y);
            let overlay = map.get_overlay(x, y);
            let powered = overlay.is_powered();
            // Transport reachability is computed by TransportSystem once per month and recorded
            // on the overlay so growth stays independent from routing implementation details.
            let functional = powered && overlay.trip_success;
            let bootstrap_ready = powered && has_local_road_access(map, x, y, 3);
            let watered = overlay.has_water();
            let covered_by_power_line = map.has_power_line(x, y);

            let pollution_penalty = 1.0 - (overlay.pollution as f32 / 255.0) * 0.7;
            let lv_bonus = overlay.land_value as f32 / 255.0 * 0.1;
            let crime_penalty = 1.0 - (overlay.crime as f32 / 255.0) * 0.7;
            let traffic_penalty = 1.0 - (overlay.traffic as f32 / 255.0) * 0.5;

            match tile {
                Tile::ZoneRes => {
                    let chance = (sim.demand_res * 0.15 + lv_bonus)
                        * pollution_penalty
                        * crime_penalty
                        * traffic_penalty;
                    if covered_by_power_line
                        && has_local_road_access(map, x, y, 3)
                        && sim.demand_res > 0.0
                    {
                        changes.push((x, y, Tile::ResLow));
                    } else if (functional || bootstrap_ready) && rng.gen::<f32>() < chance {
                        changes.push((x, y, Tile::ResLow));
                    }
                }
                Tile::ZoneComm => {
                    let chance =
                        (sim.demand_comm * 0.08 + lv_bonus * 0.5) * crime_penalty * traffic_penalty;
                    if covered_by_power_line
                        && has_local_road_access(map, x, y, 3)
                        && sim.demand_comm > 0.0
                    {
                        changes.push((x, y, Tile::CommLow));
                    } else if (functional || bootstrap_ready) && rng.gen::<f32>() < chance {
                        changes.push((x, y, Tile::CommLow));
                    }
                }
                Tile::ZoneInd => {
                    if covered_by_power_line
                        && has_local_road_access(map, x, y, 3)
                        && sim.demand_ind > 0.0
                    {
                        changes.push((x, y, Tile::IndLight));
                    } else if (functional || bootstrap_ready)
                        && rng.gen::<f32>() < sim.demand_ind * 0.08 * traffic_penalty
                    {
                        changes.push((x, y, Tile::IndLight));
                    }
                }
                Tile::ResLow => {
                    let chance = (sim.demand_res * 0.03 + lv_bonus)
                        * pollution_penalty
                        * crime_penalty
                        * traffic_penalty;
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < chance
                    {
                        changes.push((x, y, Tile::ResMed));
                    } else if !functional && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneRes));
                    }
                }
                Tile::ResMed => {
                    let chance = (sim.demand_res * 0.015 + lv_bonus * 0.5)
                        * pollution_penalty
                        * crime_penalty
                        * traffic_penalty;
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < chance
                    {
                        changes.push((x, y, Tile::ResHigh));
                    } else if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.05 {
                            changes.push((x, y, Tile::ResLow));
                        }
                    }
                }
                Tile::ResHigh => {
                    if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.10 {
                            changes.push((x, y, Tile::ResMed));
                        }
                    }
                }
                Tile::CommLow => {
                    let chance =
                        (sim.demand_comm * 0.02 + lv_bonus * 0.5) * crime_penalty * traffic_penalty;
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < chance
                    {
                        changes.push((x, y, Tile::CommHigh));
                    } else if !functional && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneComm));
                    }
                }
                Tile::CommHigh => {
                    if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.05 {
                            changes.push((x, y, Tile::CommLow));
                        }
                    }
                }
                Tile::IndLight => {
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < sim.demand_ind * 0.02 * traffic_penalty
                    {
                        changes.push((x, y, Tile::IndHeavy));
                    } else if !functional && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneInd));
                    }
                }
                Tile::IndHeavy => {
                    if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.05 {
                            changes.push((x, y, Tile::IndLight));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for (x, y, tile) in changes {
        apply_growth_change(map, x, y, tile);
    }
    let stats = compute_sector_stats(map);
    sim.residential_population = stats.residential_population;
    sim.commercial_jobs = stats.commercial_jobs;
    sim.industrial_jobs = stats.industrial_jobs;
    sim.population = stats.residential_population;
}

fn current_growth_tile(map: &Map, x: usize, y: usize) -> Tile {
    map.surface_lot_tile(x, y)
}

fn zone_allows_dense_upgrade(zone_spec: Option<ZoneSpec>, watered: bool) -> bool {
    // Dense zoning is a policy choice plus a utility requirement. Light zones stay simple and
    // never "accidentally" drift into dense buildings.
    matches!(
        zone_spec,
        Some(ZoneSpec {
            density: ZoneDensity::Dense,
            ..
        })
    ) && watered
}

fn has_local_road_access(map: &Map, start_x: usize, start_y: usize, walk_dist: i32) -> bool {
    let ix = start_x as i32;
    let iy = start_y as i32;

    for dy in -walk_dist..=walk_dist {
        for dx in -walk_dist..=walk_dist {
            if dx.abs() + dy.abs() > walk_dist {
                continue;
            }
            let nx = ix + dx;
            let ny = iy + dy;
            if !map.in_bounds(nx, ny) {
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            match map.transport_at(nx, ny) {
                Some(TransportTile::Road) => return true,
                Some(TransportTile::Onramp) => {
                    if map
                        .neighbors4(nx, ny)
                        .into_iter()
                        .any(|(_, _, tile)| matches!(tile, Tile::Road | Tile::RoadPowerLine))
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }

    false
}

fn apply_growth_change(map: &mut Map, x: usize, y: usize, tile: Tile) {
    let density = map
        .effective_zone_spec(x, y)
        .map(|zone| zone.density)
        .or_else(|| tile.inferred_zone_density())
        .unwrap_or(ZoneDensity::Light);

    // Growth rewrites only the visible surface lot. The layered map keeps utilities and
    // underground infrastructure intact unless the specific transition is supposed to remove it.
    map.clear_surface_preserve_zone(x, y);

    if let Some(kind) = ZoneKind::from_tile(tile) {
        map.set_zone_spec(x, y, Some(ZoneSpec { kind, density }));
    } else {
        map.set_zone_spec(x, y, None);
    }

    if !tile.is_zone() {
        map.set_occupant(x, y, Some(tile));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::ViewLayer;
    use crate::core::sim::system::SimSystem;
    use crate::core::sim::systems::PowerSystem;
    use crate::core::sim::transport::TransportSystem;
    use crate::core::sim::PlantState;
    use crate::core::tool::Tool;

    #[test]
    fn light_zone_caps_at_low_density() {
        let mut map = Map::new(8, 5);
        map.set(0, 0, Tile::PowerPlantCoal);
        map.set(1, 0, Tile::RoadPowerLine);
        map.set_zone_spec(
            2,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set(3, 0, Tile::Road);
        map.set(4, 0, Tile::Road);
        map.set_zone_spec(
            5,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Industrial,
                density: ZoneDensity::Light,
            }),
        );

        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
            },
        );
        sim.demand_res = 1.0;
        sim.demand_ind = 1.0;

        let mut transport = TransportSystem::default();
        for _ in 0..200 {
            PowerSystem.tick(&mut map, &mut sim);
            transport.tick(&mut map, &mut sim);
            tick_growth(&mut map, &mut sim);
        }

        assert_eq!(map.get(2, 0), Tile::ResLow);
    }

    #[test]
    fn subway_tunnels_route_in_underground_view() {
        let mut map = Map::new(4, 1);
        map.set_subway_tunnel(1, 0, true);
        map.set_subway_tunnel(2, 0, true);

        let path = crate::app::line_drag::line_shortest_path(
            &map,
            Tool::Subway,
            ViewLayer::Underground,
            1,
            0,
            2,
            0,
        );
        assert_eq!(path, vec![(1, 0), (2, 0)]);
    }

    #[test]
    fn dense_growth_preserves_zone_density_and_underground() {
        let mut map = Map::new(1, 1);
        map.set_zone_spec(
            0,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
        );
        map.set_power_line(0, 0, true);
        map.set_water_pipe(0, 0, true);
        map.set_subway_tunnel(0, 0, true);

        apply_growth_change(&mut map, 0, 0, Tile::ResLow);

        assert_eq!(map.get(0, 0), Tile::ResLow);
        assert_eq!(map.zone_density(0, 0), Some(ZoneDensity::Dense));
        assert!(!map.has_power_line(0, 0));
        assert!(map.has_water_pipe(0, 0));
        assert!(map.has_subway_tunnel(0, 0));
    }

    #[test]
    fn zones_do_not_grow_without_transport_success() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();
        map.set_zone_spec(
            1,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_overlay(
            1,
            0,
            crate::core::map::TileOverlay {
                power_level: 255,
                ..crate::core::map::TileOverlay::default()
            },
        );
        sim.demand_res = 1.0;

        for _ in 0..200 {
            tick_growth(&mut map, &mut sim);
        }

        assert_eq!(map.get(1, 0), Tile::ZoneRes);
    }

    #[test]
    fn zone_under_powerline_can_grow_and_consumes_line() {
        let mut map = Map::new(1, 1);
        let mut sim = SimState::default();
        map.set_zone_spec(
            0,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_power_line(0, 0, true);
        map.set_overlay(
            0,
            0,
            crate::core::map::TileOverlay {
                power_level: 255,
                water_service: 255,
                trip_success: true,
                ..crate::core::map::TileOverlay::default()
            },
        );
        sim.demand_res = 1.0;

        for _ in 0..200 {
            tick_growth(&mut map, &mut sim);
            if map.get(0, 0) == Tile::ResLow {
                break;
            }
        }

        assert_eq!(map.get(0, 0), Tile::ResLow);
        assert!(!map.has_power_line(0, 0));
    }

    #[test]
    fn zoned_lot_with_powerline_grows_in_full_simulation_loop() {
        let mut map = Map::new(8, 1);
        map.set(0, 0, Tile::PowerPlantCoal);
        map.set(1, 0, Tile::PowerLine);
        map.set_zone_spec(
            2,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_power_line(2, 0, true);
        map.set(3, 0, Tile::Road);
        map.set(4, 0, Tile::Road);
        map.set_zone_spec(
            5,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Industrial,
                density: ZoneDensity::Light,
            }),
        );

        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
            },
        );
        sim.demand_res = 1.0;
        sim.demand_ind = 1.0;

        let mut transport = TransportSystem::default();
        for _ in 0..240 {
            PowerSystem.tick(&mut map, &mut sim);
            transport.tick(&mut map, &mut sim);
            tick_growth(&mut map, &mut sim);
            if map.get(2, 0) == Tile::ResLow {
                break;
            }
        }

        assert_eq!(map.get(2, 0), Tile::ResLow);
        assert!(!map.has_power_line(2, 0));
        assert!(map.get_overlay(2, 0).is_powered());
    }

    #[test]
    fn zone_under_powerline_can_bootstrap_from_road_and_power() {
        let mut map = Map::new(4, 1);
        map.set(0, 0, Tile::PowerPlantCoal);
        map.set(1, 0, Tile::PowerLine);
        map.set_zone_spec(
            2,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_power_line(2, 0, true);
        map.set(3, 0, Tile::Road);

        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
            },
        );
        sim.demand_res = 1.0;

        PowerSystem.tick(&mut map, &mut sim);
        tick_growth(&mut map, &mut sim);

        assert_eq!(map.get(2, 0), Tile::ResLow);
        assert!(!map.has_power_line(2, 0));
    }

    #[test]
    fn zone_under_powerline_directly_replaces_line_when_demand_and_road_exist() {
        let mut map = Map::new(4, 1);
        map.set_zone_spec(
            2,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_power_line(2, 0, true);
        map.set(3, 0, Tile::Road);

        let mut sim = SimState::default();
        sim.demand_res = 1.0;

        tick_growth(&mut map, &mut sim);

        assert_eq!(map.get(2, 0), Tile::ResLow);
        assert!(!map.has_power_line(2, 0));
    }
}
