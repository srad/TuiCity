use super::{
    constants::{BROWNOUT_THRESHOLD, NEGLECT_THRESHOLD_MONTHS, WALK_DIST},
    economy::compute_sector_stats,
    SimState,
};
use crate::core::map::TransportTile;
use crate::core::map::{Map, Tile, ZoneDensity, ZoneKind, ZoneSpec};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// All pre-computed values for a single tile evaluation pass.
struct TileCtx {
    zone_spec: Option<ZoneSpec>,
    functional: bool,
    bootstrap_ready: bool,
    watered: bool,
    severely_brownout: bool,
    fully_unpowered: bool,
    pollution_penalty: f32,
    lv_bonus: f32,
    crime_penalty: f32,
    traffic_penalty: f32,
}

/// Evaluate residential zone/building growth. Returns the new tile if a change should occur.
fn evaluate_res(tile: Tile, ctx: &TileCtx, demand: f32, rng: &mut StdRng) -> Option<Tile> {
    let dense = zone_allows_dense_upgrade(ctx.zone_spec, ctx.watered);
    match tile {
        Tile::ZoneRes => {
            let chance = (demand * 0.15 + ctx.lv_bonus)
                * ctx.pollution_penalty
                * ctx.crime_penalty
                * ctx.traffic_penalty;
            if (ctx.functional || ctx.bootstrap_ready) && rng.gen::<f32>() < chance {
                Some(Tile::ResLow)
            } else {
                None
            }
        }
        Tile::ResLow => {
            let upgrade_chance = (demand * 0.03 + ctx.lv_bonus)
                * ctx.pollution_penalty
                * ctx.crime_penalty
                * ctx.traffic_penalty;
            if ctx.functional && dense && rng.gen::<f32>() < upgrade_chance {
                Some(Tile::ResMed)
            } else if !ctx.functional && rng.gen::<f32>() < 0.01 {
                Some(Tile::ZoneRes)
            } else if ctx.severely_brownout || ctx.fully_unpowered {
                if rng.gen::<f32>() < 0.01 { Some(Tile::ZoneRes) } else { None }
            } else {
                None
            }
        }
        Tile::ResMed => {
            let upgrade_chance = (demand * 0.015 + ctx.lv_bonus * 0.5)
                * ctx.pollution_penalty
                * ctx.crime_penalty
                * ctx.traffic_penalty;
            if ctx.functional && dense && rng.gen::<f32>() < upgrade_chance {
                Some(Tile::ResHigh)
            } else if !ctx.functional || !dense {
                if rng.gen::<f32>() < 0.05 { Some(Tile::ResLow) } else { None }
            } else if ctx.severely_brownout || ctx.fully_unpowered {
                if rng.gen::<f32>() < 0.01 { Some(Tile::ResLow) } else { None }
            } else {
                None
            }
        }
        Tile::ResHigh => {
            if !ctx.functional || !dense {
                if rng.gen::<f32>() < 0.10 { Some(Tile::ResMed) } else { None }
            } else if ctx.severely_brownout || ctx.fully_unpowered {
                if rng.gen::<f32>() < 0.02 { Some(Tile::ResMed) } else { None }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Evaluate commercial zone/building growth. Returns the new tile if a change should occur.
fn evaluate_comm(tile: Tile, ctx: &TileCtx, demand: f32, rng: &mut StdRng) -> Option<Tile> {
    let dense = zone_allows_dense_upgrade(ctx.zone_spec, ctx.watered);
    match tile {
        Tile::ZoneComm => {
            let chance = (demand * 0.08 + ctx.lv_bonus * 0.5)
                * ctx.crime_penalty
                * ctx.traffic_penalty;
            if (ctx.functional || ctx.bootstrap_ready) && rng.gen::<f32>() < chance {
                Some(Tile::CommLow)
            } else {
                None
            }
        }
        Tile::CommLow => {
            let upgrade_chance =
                (demand * 0.02 + ctx.lv_bonus * 0.5) * ctx.crime_penalty * ctx.traffic_penalty;
            if ctx.functional && dense && rng.gen::<f32>() < upgrade_chance {
                Some(Tile::CommHigh)
            } else if !ctx.functional && rng.gen::<f32>() < 0.01 {
                Some(Tile::ZoneComm)
            } else if ctx.severely_brownout || ctx.fully_unpowered {
                if rng.gen::<f32>() < 0.01 { Some(Tile::ZoneComm) } else { None }
            } else {
                None
            }
        }
        Tile::CommHigh => {
            if !ctx.functional || !dense {
                if rng.gen::<f32>() < 0.05 { Some(Tile::CommLow) } else { None }
            } else if ctx.severely_brownout || ctx.fully_unpowered {
                if rng.gen::<f32>() < 0.02 { Some(Tile::CommLow) } else { None }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Evaluate industrial zone/building growth. Returns the new tile if a change should occur.
fn evaluate_ind(tile: Tile, ctx: &TileCtx, demand: f32, rng: &mut StdRng) -> Option<Tile> {
    let dense = zone_allows_dense_upgrade(ctx.zone_spec, ctx.watered);
    match tile {
        Tile::ZoneInd => {
            let chance = demand * 0.08 * ctx.traffic_penalty;
            if (ctx.functional || ctx.bootstrap_ready) && rng.gen::<f32>() < chance {
                Some(Tile::IndLight)
            } else {
                None
            }
        }
        Tile::IndLight => {
            let upgrade_chance = demand * 0.02 * ctx.traffic_penalty;
            if ctx.functional && dense && rng.gen::<f32>() < upgrade_chance {
                Some(Tile::IndHeavy)
            } else if !ctx.functional && rng.gen::<f32>() < 0.01 {
                Some(Tile::ZoneInd)
            } else if ctx.severely_brownout || ctx.fully_unpowered {
                if rng.gen::<f32>() < 0.01 { Some(Tile::ZoneInd) } else { None }
            } else {
                None
            }
        }
        Tile::IndHeavy => {
            if !ctx.functional || !dense {
                if rng.gen::<f32>() < 0.05 { Some(Tile::IndLight) } else { None }
            } else if ctx.severely_brownout || ctx.fully_unpowered {
                if rng.gen::<f32>() < 0.02 { Some(Tile::IndLight) } else { None }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn tick_growth(map: &mut Map, sim: &mut SimState) {
    let mut rng = StdRng::seed_from_u64(sim.rng.growth);
    let w = map.width;
    let h = map.height;

    let mut changes: Vec<(usize, usize, Tile)> = Vec::new();
    let mut neglect_degrades: Vec<(usize, usize)> = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let tile = current_growth_tile(map, x, y);
            let overlay_data = map.overlays[idx];
            let powered = overlay_data.is_powered();
            let watered = overlay_data.has_water();
            let trip_success = overlay_data.trip_success;
            let neglected = overlay_data.neglected_months;

            if tile.is_building() {
                let is_underserved = !powered || !watered || !trip_success;
                if is_underserved {
                    map.overlays[idx].neglected_months = neglected.saturating_add(1);
                    if map.overlays[idx].neglected_months >= NEGLECT_THRESHOLD_MONTHS {
                        neglect_degrades.push((x, y));
                    }
                } else {
                    map.overlays[idx].neglected_months = 0;
                }
            }

            let power_ratio = overlay_data.power_level as f32 / 255.0;
            let ctx = TileCtx {
                zone_spec: map.effective_zone_spec(x, y),
                functional: powered && trip_success,
                bootstrap_ready: powered && has_local_road_access(map, x, y, WALK_DIST),
                watered,
                severely_brownout: power_ratio < BROWNOUT_THRESHOLD && powered,
                fully_unpowered: overlay_data.power_level == 0,
                pollution_penalty: 1.0 - (overlay_data.pollution as f32 / 255.0) * 0.7,
                lv_bonus: overlay_data.land_value as f32 / 255.0 * 0.1,
                crime_penalty: 1.0 - (overlay_data.crime as f32 / 255.0) * 0.7,
                traffic_penalty: 1.0 - (overlay_data.traffic as f32 / 255.0) * 0.5,
            };

            let new_tile = match tile {
                Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh => {
                    evaluate_res(tile, &ctx, sim.demand.res, &mut rng)
                }
                Tile::ZoneComm | Tile::CommLow | Tile::CommHigh => {
                    evaluate_comm(tile, &ctx, sim.demand.comm, &mut rng)
                }
                Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy => {
                    evaluate_ind(tile, &ctx, sim.demand.ind, &mut rng)
                }
                _ => None,
            };

            if let Some(t) = new_tile {
                changes.push((x, y, t));
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
    sim.pop.residential_population = stats.residential_population;
    sim.pop.commercial_jobs = stats.commercial_jobs;
    sim.pop.industrial_jobs = stats.industrial_jobs;
    sim.pop.population = stats.residential_population;
    sim.rng.growth = rng.gen();
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
        sim.demand.res = 1.0;
        sim.demand.ind = 1.0;

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
        sim.demand.res = 1.0;

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
        sim.demand.res = 1.0;

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
        sim.demand.res = 1.0;
        sim.demand.ind = 1.0;

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
        // Zone with power + road access develops within 50 ticks (probabilistic timing)
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
        sim.demand.res = 1.0;

        for _ in 0..50 {
            PowerSystem.tick(&mut map, &mut sim);
            tick_growth(&mut map, &mut sim);
            if map.get(2, 0) == Tile::ResLow {
                break;
            }
        }

        assert_eq!(map.get(2, 0), Tile::ResLow, "Zone should develop within 50 ticks");
        assert!(!map.has_power_line(2, 0));
    }

    #[test]
    fn zone_with_power_and_road_develops_probabilistically() {
        // Zone with power plant, power line, and road develops within 50 ticks
        let mut map = Map::new(5, 1);
        map.set(0, 0, Tile::PowerPlantCoal);
        map.set(1, 0, Tile::PowerLine);
        map.set(2, 0, Tile::PowerLine);
        map.set_zone_spec(
            3,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set(4, 0, Tile::Road);

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
        sim.demand.res = 1.0;

        for _ in 0..50 {
            PowerSystem.tick(&mut map, &mut sim);
            tick_growth(&mut map, &mut sim);
            if map.get(3, 0) == Tile::ResLow {
                break;
            }
        }

        assert_eq!(map.get(3, 0), Tile::ResLow, "Zone should develop within 50 ticks");
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
        sim.demand.res = 1.0;
        sim.rng.growth = 0xFACEFEED;

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
        test_sim.rng.growth = seed;

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
        sim.demand.res = 1.0;

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
        sim.rng.growth = 0xAAAA; // seed where 5% functional decay doesn't trigger
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
        sim.demand.res = 1.0;
        sim.rng.growth = 0xDEAD;

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
        sim.demand.res = 1.0;
        sim.rng.growth = 0xBAD;

        // power_ratio = 77/255 = 0.3019... > 0.30 — NOT severely brownout
        // Degradation is probabilistic (1%), so run many times to check it doesn't ALWAYS degrade
        let mut degraded_count = 0;
        for seed in 0..100u64 {
            let mut test_map = map.clone();
            let mut test_sim = sim.clone();
            test_sim.rng.growth = seed;
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
        sim.demand.res = 1.0;
        sim.rng.growth = 0xBAD;

        // power_ratio = 74/255 = 0.290... < 0.30 — IS severely brownout
        // Degradation is probabilistic (1%), so we check that it CAN degrade
        let mut degraded_count = 0;
        for seed in 0..200u64 {
            let mut test_map = map.clone();
            let mut test_sim = sim.clone();
            test_sim.rng.growth = seed;
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
        sim.demand.res = 1.0;

        // power_level=0 is fully unpowered — should have brownout degradation path
        let mut degraded_count = 0;
        for seed in 0..200u64 {
            let mut test_map = map.clone();
            let mut test_sim = sim.clone();
            test_sim.rng.growth = seed;
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

    // ── evaluate_res / evaluate_comm / evaluate_ind unit tests ───────────────

    fn make_ctx(functional: bool, bootstrap_ready: bool) -> TileCtx {
        TileCtx {
            zone_spec: Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
            functional,
            bootstrap_ready,
            watered: true,
            severely_brownout: false,
            fully_unpowered: false,
            pollution_penalty: 1.0,
            lv_bonus: 0.0,
            crime_penalty: 1.0,
            traffic_penalty: 1.0,
        }
    }

    #[test]
    fn evaluate_res_zone_upgrades_to_res_low_when_conditions_met() {
        // ZoneRes with bootstrap_ready + positive demand → develops within reasonable seeds
        let ctx = make_ctx(false, true);
        let developed = (0..200u64)
            .filter(|&seed| {
                let mut rng = StdRng::seed_from_u64(seed);
                evaluate_res(Tile::ZoneRes, &ctx, 0.8, &mut rng) == Some(Tile::ResLow)
            })
            .count();
        assert!(developed > 0, "ZoneRes should develop in at least some seeds (got 0/200)");
    }

    #[test]
    fn evaluate_res_zone_no_upgrade_when_demand_zero() {
        // Demand ≤ 0 means the fast-path and the probabilistic path both fail to upgrade.
        let ctx = make_ctx(false, false);
        let mut rng = StdRng::seed_from_u64(0);
        // Demand = 0.0 → fast-path blocked; probabilistic chance = 0 → None
        let result = evaluate_res(Tile::ZoneRes, &ctx, 0.0, &mut rng);
        assert_eq!(result, None);
    }

    #[test]
    fn evaluate_res_low_downgrades_when_not_functional() {
        // ResLow without functional service should degrade in at least some seeds.
        let ctx = TileCtx {
            functional: false,
            bootstrap_ready: false,
            severely_brownout: false,
            fully_unpowered: false,
            watered: true,
            zone_spec: Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
            pollution_penalty: 1.0,
            lv_bonus: 0.0,
            crime_penalty: 1.0,
            traffic_penalty: 1.0,
        };
        let mut degraded = 0usize;
        for seed in 0..500u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            if evaluate_res(Tile::ResLow, &ctx, 0.5, &mut rng) == Some(Tile::ZoneRes) {
                degraded += 1;
            }
        }
        assert!(degraded > 0, "ResLow should degrade occasionally when not functional");
    }

    #[test]
    fn evaluate_comm_zone_upgrades_to_comm_low_when_conditions_met() {
        let ctx = make_ctx(false, true);
        let developed = (0..200u64)
            .filter(|&seed| {
                let mut rng = StdRng::seed_from_u64(seed);
                evaluate_comm(Tile::ZoneComm, &ctx, 0.8, &mut rng) == Some(Tile::CommLow)
            })
            .count();
        assert!(developed > 0, "ZoneComm should develop in at least some seeds (got 0/200)");
    }

    #[test]
    fn evaluate_ind_zone_upgrades_to_ind_light_when_conditions_met() {
        let ctx = make_ctx(false, true);
        let developed = (0..200u64)
            .filter(|&seed| {
                let mut rng = StdRng::seed_from_u64(seed);
                evaluate_ind(Tile::ZoneInd, &ctx, 0.8, &mut rng) == Some(Tile::IndLight)
            })
            .count();
        assert!(developed > 0, "ZoneInd should develop in at least some seeds (got 0/200)");
    }

    #[test]
    fn evaluate_res_high_downgrades_when_not_functional() {
        let ctx = TileCtx {
            functional: false,
            zone_spec: Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
            bootstrap_ready: false,
            watered: true,
            severely_brownout: false,
            fully_unpowered: false,
            pollution_penalty: 1.0,
            lv_bonus: 0.0,
            crime_penalty: 1.0,
            traffic_penalty: 1.0,
        };
        let mut degraded = 0usize;
        for seed in 0..200u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            if evaluate_res(Tile::ResHigh, &ctx, 0.5, &mut rng) == Some(Tile::ResMed) {
                degraded += 1;
            }
        }
        assert!(degraded > 0, "ResHigh should degrade when not functional");
    }

    #[test]
    fn evaluate_functions_return_none_for_unrelated_tiles() {
        let ctx = make_ctx(true, true);
        let mut rng = StdRng::seed_from_u64(0);
        // evaluate_res should not touch commercial or industrial tiles
        assert_eq!(evaluate_res(Tile::CommLow, &ctx, 1.0, &mut rng), None);
        assert_eq!(evaluate_res(Tile::IndHeavy, &ctx, 1.0, &mut rng), None);
        // evaluate_comm should not touch residential or industrial tiles
        assert_eq!(evaluate_comm(Tile::ResHigh, &ctx, 1.0, &mut rng), None);
        assert_eq!(evaluate_comm(Tile::IndLight, &ctx, 1.0, &mut rng), None);
        // evaluate_ind should not touch residential or commercial tiles
        assert_eq!(evaluate_ind(Tile::ResLow, &ctx, 1.0, &mut rng), None);
        assert_eq!(evaluate_ind(Tile::CommHigh, &ctx, 1.0, &mut rng), None);
    }
}
