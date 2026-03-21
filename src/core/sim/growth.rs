use super::{economy::compute_sector_stats, SimState};
use crate::core::map::TransportTile;
use crate::core::map::{Map, Tile, ZoneDensity, ZoneKind, ZoneSpec};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub fn tick_growth(map: &mut Map, sim: &mut SimState) {
    let mut rng = StdRng::seed_from_u64(sim.growth_rng_state);
    let w = map.width;
    let h = map.height;

    let mut changes: Vec<(usize, usize, Tile)> = Vec::new();
    let mut neglect_degrades: Vec<(usize, usize)> = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let zone_spec = map.effective_zone_spec(x, y);
            let tile = current_growth_tile(map, x, y);
            let overlay_data = map.overlays[idx];
            let powered = overlay_data.is_powered();
            let functional = powered && overlay_data.trip_success;
            let bootstrap_ready = powered && has_local_road_access(map, x, y, 3);
            let watered = overlay_data.has_water();
            let covered_by_power_line = map.has_power_line(x, y);
            let power_level = overlay_data.power_level;
            let polluted = overlay_data.pollution;
            let crime = overlay_data.crime;
            let traffic = overlay_data.traffic;
            let trip_success = overlay_data.trip_success;
            let neglected = overlay_data.neglected_months;

            if tile.is_building() {
                let is_underserved = !powered || !watered || !trip_success;
                if is_underserved {
                    map.overlays[idx].neglected_months = neglected.saturating_add(1);
                    if map.overlays[idx].neglected_months >= 6 {
                        neglect_degrades.push((x, y));
                    }
                } else {
                    map.overlays[idx].neglected_months = 0;
                }
            }

            let power_ratio = power_level as f32 / 255.0;
            let severely_brownout = power_ratio < 0.30 && powered;
            let fully_unpowered = power_level == 0;

            let pollution_penalty = 1.0 - (polluted as f32 / 255.0) * 0.7;
            let lv_bonus = overlay_data.land_value as f32 / 255.0 * 0.1;
            let crime_penalty = 1.0 - (crime as f32 / 255.0) * 0.7;
            let traffic_penalty = 1.0 - (traffic as f32 / 255.0) * 0.5;

            match tile {
                Tile::ZoneRes => {
                    if covered_by_power_line
                        && has_local_road_access(map, x, y, 3)
                        && sim.demand_res > 0.0
                    {
                        changes.push((x, y, Tile::ResLow));
                    } else {
                        let chance = (sim.demand_res * 0.15 + lv_bonus)
                            * pollution_penalty
                            * crime_penalty
                            * traffic_penalty;
                        if (functional || bootstrap_ready) && rng.gen::<f32>() < chance {
                            changes.push((x, y, Tile::ResLow));
                        }
                    }
                }
                Tile::ZoneComm => {
                    if covered_by_power_line
                        && has_local_road_access(map, x, y, 3)
                        && sim.demand_comm > 0.0
                    {
                        changes.push((x, y, Tile::CommLow));
                    } else {
                        let chance = (sim.demand_comm * 0.08 + lv_bonus * 0.5)
                            * crime_penalty
                            * traffic_penalty;
                        if (functional || bootstrap_ready) && rng.gen::<f32>() < chance {
                            changes.push((x, y, Tile::CommLow));
                        }
                    }
                }
                Tile::ZoneInd => {
                    if covered_by_power_line
                        && has_local_road_access(map, x, y, 3)
                        && sim.demand_ind > 0.0
                    {
                        changes.push((x, y, Tile::IndLight));
                    } else {
                        let chance = sim.demand_ind * 0.08 * traffic_penalty;
                        if (functional || bootstrap_ready) && rng.gen::<f32>() < chance {
                            changes.push((x, y, Tile::IndLight));
                        }
                    }
                }
                Tile::ResLow => {
                    let upgrade_chance = (sim.demand_res * 0.03 + lv_bonus)
                        * pollution_penalty
                        * crime_penalty
                        * traffic_penalty;
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < upgrade_chance
                    {
                        changes.push((x, y, Tile::ResMed));
                    } else if !functional && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneRes));
                    } else if severely_brownout || fully_unpowered {
                        if rng.gen::<f32>() < 0.01 {
                            changes.push((x, y, Tile::ZoneRes));
                        }
                    }
                }
                Tile::ResMed => {
                    let upgrade_chance = (sim.demand_res * 0.015 + lv_bonus * 0.5)
                        * pollution_penalty
                        * crime_penalty
                        * traffic_penalty;
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < upgrade_chance
                    {
                        changes.push((x, y, Tile::ResHigh));
                    } else if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.05 {
                            changes.push((x, y, Tile::ResLow));
                        }
                    } else if severely_brownout || fully_unpowered {
                        if rng.gen::<f32>() < 0.01 {
                            changes.push((x, y, Tile::ResLow));
                        }
                    }
                }
                Tile::ResHigh => {
                    if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.10 {
                            changes.push((x, y, Tile::ResMed));
                        }
                    } else if severely_brownout || fully_unpowered {
                        if rng.gen::<f32>() < 0.02 {
                            changes.push((x, y, Tile::ResMed));
                        }
                    }
                }
                Tile::CommLow => {
                    let upgrade_chance =
                        (sim.demand_comm * 0.02 + lv_bonus * 0.5) * crime_penalty * traffic_penalty;
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < upgrade_chance
                    {
                        changes.push((x, y, Tile::CommHigh));
                    } else if !functional && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneComm));
                    } else if severely_brownout || fully_unpowered {
                        if rng.gen::<f32>() < 0.01 {
                            changes.push((x, y, Tile::ZoneComm));
                        }
                    }
                }
                Tile::CommHigh => {
                    if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.05 {
                            changes.push((x, y, Tile::CommLow));
                        }
                    } else if severely_brownout || fully_unpowered {
                        if rng.gen::<f32>() < 0.02 {
                            changes.push((x, y, Tile::CommLow));
                        }
                    }
                }
                Tile::IndLight => {
                    let upgrade_chance = sim.demand_ind * 0.02 * traffic_penalty;
                    if functional
                        && zone_allows_dense_upgrade(zone_spec, watered)
                        && rng.gen::<f32>() < upgrade_chance
                    {
                        changes.push((x, y, Tile::IndHeavy));
                    } else if !functional && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneInd));
                    } else if severely_brownout || fully_unpowered {
                        if rng.gen::<f32>() < 0.01 {
                            changes.push((x, y, Tile::ZoneInd));
                        }
                    }
                }
                Tile::IndHeavy => {
                    if !functional || !zone_allows_dense_upgrade(zone_spec, watered) {
                        if rng.gen::<f32>() < 0.05 {
                            changes.push((x, y, Tile::IndLight));
                        }
                    } else if severely_brownout || fully_unpowered {
                        if rng.gen::<f32>() < 0.02 {
                            changes.push((x, y, Tile::IndLight));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for (x, y) in neglect_degrades {
        let tile = map.surface_lot_tile(x, y);
        let downgrade = match tile {
            Tile::ResHigh => Some(Tile::ResMed),
            Tile::ResMed => Some(Tile::ResLow),
            Tile::ResLow => Some(Tile::ZoneRes),
            Tile::CommHigh => Some(Tile::CommLow),
            Tile::CommLow => Some(Tile::ZoneComm),
            Tile::IndHeavy => Some(Tile::IndLight),
            Tile::IndLight => Some(Tile::ZoneInd),
            _ => None,
        };
        if let Some(t) = downgrade {
            changes.push((x, y, t));
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
    sim.growth_rng_state = rng.gen();
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

    // Reset neglect counter when a tile is rebuilt
    let idx = y * map.width + x;
    map.overlays[idx].neglected_months = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{TileOverlay, ViewLayer};
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
                efficiency: 1.0,
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
                efficiency: 1.0,
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
                efficiency: 1.0,
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

    #[test]
    fn growth_uses_seeded_rng_deterministic() {
        // Growth must be deterministic with fixed seed — no thread_rng interference
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

        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
            },
        );
        sim.demand_res = 1.0;
        sim.growth_rng_state = 0xFACEFEED;

        let results = [
            run_growth_with_seed(&map, &sim, 0xFACEFEED),
            run_growth_with_seed(&map, &sim, 0xFACEFEED),
            run_growth_with_seed(&map, &sim, 0xFACEFEED),
        ];

        assert_eq!(
            results[0], results[1],
            "growth must be deterministic with fixed seed"
        );
        assert_eq!(
            results[1], results[2],
            "growth must be deterministic with fixed seed"
        );
    }

    fn run_growth_with_seed(map: &Map, base_sim: &SimState, seed: u64) -> Tile {
        let mut test_map = map.clone();
        let mut test_sim = base_sim.clone();
        test_sim.growth_rng_state = seed;

        for _ in 0..50 {
            tick_growth(&mut test_map, &mut test_sim);
            if test_map.get(2, 0) != Tile::ZoneRes {
                return test_map.get(2, 0);
            }
        }
        test_map.get(2, 0)
    }

    #[test]
    fn neglect_degrades_at_6_months_not_before() {
        let mut map = Map::new(3, 1);
        map.set(0, 0, Tile::ResLow);
        map.set(1, 0, Tile::PowerPlantCoal);
        map.set(2, 0, Tile::Road);

        let mut sim = SimState::default();
        sim.demand_res = 1.0;

        // Start at 4 months neglect — after tick 1: 5 (still < 6, no degrade)
        map.set_overlay(
            0,
            0,
            TileOverlay {
                power_level: 255,
                trip_success: true,
                neglected_months: 4,
                ..TileOverlay::default()
            },
        );
        sim.growth_rng_state = 0xAAAA; // seed where 5% functional decay doesn't trigger
        tick_growth(&mut map, &mut sim);
        assert_eq!(
            map.get(0, 0),
            Tile::ResLow,
            "4->5 neglect should NOT degrade (still < 6)"
        );
        assert_eq!(
            map.get_overlay(0, 0).neglected_months,
            5,
            "neglect should be 5"
        );

        // Second tick: 5->6, now >= 6, degradation triggers
        tick_growth(&mut map, &mut sim);
        assert_eq!(
            map.get(0, 0),
            Tile::ZoneRes,
            "5->6 (>= 6) neglect SHOULD degrade"
        );
        assert_eq!(
            map.get_overlay(0, 0).neglected_months,
            0,
            "neglect resets after degradation"
        );
    }

    #[test]
    fn neglect_saturating_add_means_max_neglect_degrades_immediately() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::ResLow);
        map.set(1, 0, Tile::PowerPlantCoal);
        map.set(2, 0, Tile::Road);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                power_level: 255,
                trip_success: true,
                neglected_months: u8::MAX,
                ..TileOverlay::default()
            },
        );
        sim.demand_res = 1.0;
        sim.growth_rng_state = 0xDEAD;

        // u8::MAX.saturating_add(1) = u8::MAX (saturates, does not wrap).
        // 255 >= 6, so degradation triggers immediately and resets to 0.
        tick_growth(&mut map, &mut sim);
        assert_eq!(
            map.get_overlay(0, 0).neglected_months,
            0,
            "max neglect saturates and degrades in one tick, resetting to 0"
        );
        assert_eq!(
            map.get(0, 0),
            Tile::ZoneRes,
            "max neglect should trigger immediate degradation"
        );
    }

    #[test]
    fn brownout_at_30_percent_power_is_not_severe() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::ResLow);
        map.set(1, 0, Tile::PowerPlantCoal);
        map.set(2, 0, Tile::Road);
        // 30% of 255 = 76.5, so power_level=76 is just below 30%
        // power_level=77 is just above 30%
        map.set_overlay(
            0,
            0,
            TileOverlay {
                power_level: 77,
                trip_success: true,
                neglected_months: 0,
                ..TileOverlay::default()
            },
        );
        sim.demand_res = 1.0;
        sim.growth_rng_state = 0xBAD;

        // power_ratio = 77/255 = 0.3019... > 0.30 — NOT severely brownout
        // Degradation is probabilistic (1%), so run many times to check it doesn't ALWAYS degrade
        let mut degraded_count = 0;
        for seed in 0..100u64 {
            let mut test_map = map.clone();
            let mut test_sim = sim.clone();
            test_sim.growth_rng_state = seed;
            tick_growth(&mut test_map, &mut test_sim);
            if test_map.get(0, 0) == Tile::ZoneRes {
                degraded_count += 1;
            }
        }
        assert!(
            degraded_count < 50,
            "power at 30.2% (77/255) should NOT be severely brownout — degraded {}/100 times",
            degraded_count
        );
    }

    #[test]
    fn brownout_at_29_percent_power_is_severe() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::ResLow);
        map.set(1, 0, Tile::PowerPlantCoal);
        map.set(2, 0, Tile::Road);
        // 29% of 255 = 73.95, so power_level=74 is below 30%
        map.set_overlay(
            0,
            0,
            TileOverlay {
                power_level: 74,
                trip_success: true,
                neglected_months: 0,
                ..TileOverlay::default()
            },
        );
        sim.demand_res = 1.0;
        sim.growth_rng_state = 0xBAD;

        // power_ratio = 74/255 = 0.290... < 0.30 — IS severely brownout
        // Degradation is probabilistic (1%), so we check that it CAN degrade
        let mut degraded_count = 0;
        for seed in 0..200u64 {
            let mut test_map = map.clone();
            let mut test_sim = sim.clone();
            test_sim.growth_rng_state = seed;
            tick_growth(&mut test_map, &mut test_sim);
            if test_map.get(0, 0) == Tile::ZoneRes {
                degraded_count += 1;
            }
        }
        assert!(
            degraded_count > 0,
            "power at 29% (74/255) IS severely brownout — should degrade at least once in 200 runs, got {}",
            degraded_count
        );
    }

    #[test]
    fn fully_unpowered_tile_can_brownout_degrade() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::ResLow);
        map.set(1, 0, Tile::PowerPlantCoal);
        map.set(2, 0, Tile::Road);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                power_level: 0,
                trip_success: true,
                neglected_months: 0,
                ..TileOverlay::default()
            },
        );
        sim.demand_res = 1.0;

        // power_level=0 is fully unpowered — should have brownout degradation path
        let mut degraded_count = 0;
        for seed in 0..200u64 {
            let mut test_map = map.clone();
            let mut test_sim = sim.clone();
            test_sim.growth_rng_state = seed;
            tick_growth(&mut test_map, &mut test_sim);
            if test_map.get(0, 0) == Tile::ZoneRes {
                degraded_count += 1;
            }
        }
        assert!(
            degraded_count > 0,
            "fully unpowered (power_level=0) should degrade — degraded {}/200",
            degraded_count
        );
    }
}
