mod network;
mod pathfinding;

use network::{transfer_entry_nodes, LotAccess, NetworkCache};
use pathfinding::{search_path, RouteSuccess};

use super::{economy::tile_sector_capacity, system::SimSystem, DepotState, SimState};
use crate::core::map::{Map, Tile, TripFailure, TripMode, ZoneKind};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

// Re-export transport constants from the central constants module so existing callers
// (tests, other modules) that import from here continue to compile unchanged.
pub use crate::core::sim::constants::{
    BUS_DEPOT_CAPACITY, MAX_TRIP_COST, ROAD_TRAFFIC_FACTOR, TRANSFER_PENALTY, WALK_DIST,
};
const BUS_TRAFFIC_FACTOR: u16 = 1;

// ── Per-lot trip state ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default)]
struct LotTripState {
    bus_cooldown: u8,
    rail_cooldown: u8,
    subway_cooldown: u8,
}

impl LotTripState {
    fn advance_month(&mut self) {
        self.bus_cooldown = self.bus_cooldown.saturating_sub(1);
        self.rail_cooldown = self.rail_cooldown.saturating_sub(1);
        self.subway_cooldown = self.subway_cooldown.saturating_sub(1);
    }

    fn cooldown(self, mode: TripMode) -> u8 {
        match mode {
            TripMode::Road => 0,
            TripMode::Bus => self.bus_cooldown,
            TripMode::Rail => self.rail_cooldown,
            TripMode::Subway => self.subway_cooldown,
        }
    }

    fn trigger_cooldown(&mut self, mode: TripMode) {
        match mode {
            TripMode::Road => {}
            // Cooldowns are written as 2 because the monthly tick decrements them before
            // evaluating trips. A value of 2 therefore blocks the next month and clears
            // on the month after that.
            TripMode::Bus => self.bus_cooldown = 2,
            TripMode::Rail => self.rail_cooldown = 2,
            TripMode::Subway => self.subway_cooldown = 2,
        }
    }
}

// ── Trip result types ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default)]
struct ModeEligibility {
    has_local: bool,
    has_destination: bool,
    eligible: bool,
}

#[derive(Clone, Debug)]
enum TargetTripResult {
    Success {
        mode: TripMode,
        path: Vec<usize>,
        cost: usize,
    },
    Failure(TripFailure),
}

#[derive(Clone, Debug, Default)]
struct LotSimulation {
    success: bool,
    failure: Option<TripFailure>,
    mode_weights: [u32; 4],
    total_cost: u32,
    successful_attempts: u32,
}

// ── TransportSystem ───────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct TransportSystem {
    lot_state: Vec<LotTripState>,
    node_to_depot: HashMap<usize, (usize, usize)>,
}

impl SimSystem for TransportSystem {
    fn name(&self) -> &str {
        "Transport"
    }

    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        let len = map.width * map.height;
        if self.lot_state.len() != len {
            self.lot_state = vec![LotTripState::default(); len];
        }
        for state in &mut self.lot_state {
            state.advance_month();
        }

        let cache = NetworkCache::build(map);

        // Build node-to-depot mapping and reset depot trip counts
        self.node_to_depot.clear();
        for state in sim.depots.values_mut() {
            state.trips_used = 0;
        }
        for y in 0..map.height {
            for x in 0..map.width {
                if map.occupant_at(x, y) == Some(Tile::BusDepot) {
                    sim.depots
                        .entry((x, y))
                        .or_insert(DepotState { trips_used: 0 });
                    for entry_node in transfer_entry_nodes(map, x, y, Tile::BusDepot) {
                        self.node_to_depot.insert(entry_node, (x, y));
                    }
                }
            }
        }

        // Persisted RNG state keeps SC2000-style transit randomness reproducible across
        // save/load and deterministic in tests.
        let mut rng = StdRng::seed_from_u64(sim.rng.transport);
        let mut raw_traffic: Vec<u16> = map
            .overlays
            .iter()
            .map(|overlay| overlay.traffic.saturating_sub(16) as u16)
            .collect();

        sim.trips.attempts = 0;
        sim.trips.successes = 0;
        sim.trips.failures = 0;
        sim.trips.road_share = 0;
        sim.trips.bus_share = 0;
        sim.trips.rail_share = 0;
        sim.trips.subway_share = 0;

        for y in 0..map.height {
            for x in 0..map.width {
                let idx = map.idx(x, y);
                let tile = trip_lot_tile(map, x, y);
                if trip_targets(tile).is_none() {
                    continue;
                }

                let developed = tile.is_building();
                let weight = trip_weight(tile);
                let simulation = simulate_lot(
                    map,
                    &cache,
                    x,
                    y,
                    tile,
                    developed,
                    weight,
                    &mut self.lot_state[idx],
                    &mut rng,
                    &mut raw_traffic,
                    sim,
                    &self.node_to_depot,
                );
                let overlay = &mut map.overlays[idx];
                overlay.trip_success = simulation.success;
                if simulation.success {
                    overlay.trip_mode = dominant_mode(&simulation.mode_weights);
                    overlay.trip_cost = (simulation.total_cost
                        / simulation.successful_attempts.max(1))
                    .min(255) as u8;
                } else {
                    overlay.trip_failure = simulation.failure;
                }
            }
        }

        for (idx, traffic) in raw_traffic.into_iter().enumerate() {
            map.overlays[idx].traffic = traffic.min(255) as u8;
        }

        sim.rng.transport = rng.gen();
    }
}

// ── Lot simulation ────────────────────────────────────────────────────────────

fn simulate_lot(
    map: &Map,
    cache: &NetworkCache,
    x: usize,
    y: usize,
    tile: Tile,
    developed: bool,
    weight: u16,
    lot_state: &mut LotTripState,
    rng: &mut StdRng,
    raw_traffic: &mut [u16],
    sim: &mut SimState,
    node_to_depot: &HashMap<usize, (usize, usize)>,
) -> LotSimulation {
    let mut simulation = LotSimulation::default();
    let idx = map.idx(x, y);
    let access = &cache.lot_access[idx];

    for &target_kind in trip_targets(tile).unwrap_or(&[]) {
        if developed {
            sim.trips.attempts = sim.trips.attempts.saturating_add(weight as u32);
        }

        match attempt_target_trip(map, cache, access, target_kind, lot_state, rng) {
            TargetTripResult::Success { mode, path, cost } => {
                let diagnostic_weight = if developed { weight.max(1) as u32 } else { 1 };
                simulation.success = true;
                simulation.successful_attempts += 1;
                simulation.total_cost = simulation.total_cost.saturating_add(cost as u32);
                simulation.mode_weights[mode_index(mode)] =
                    simulation.mode_weights[mode_index(mode)].saturating_add(diagnostic_weight);

                if developed {
                    sim.trips.successes = sim.trips.successes.saturating_add(weight as u32);
                    match mode {
                        TripMode::Road => {
                            sim.trips.road_share = sim.trips.road_share.saturating_add(weight as u32);
                            apply_path_traffic(raw_traffic, &path, ROAD_TRAFFIC_FACTOR, weight);
                        }
                        TripMode::Bus => {
                            let depot_full = if let Some(&depot_pos) =
                                path.first().and_then(|&n| node_to_depot.get(&n))
                            {
                                if let Some(state) = sim.depots.get_mut(&depot_pos) {
                                    if state.trips_used >= BUS_DEPOT_CAPACITY {
                                        true
                                    } else {
                                        state.trips_used =
                                            state.trips_used.saturating_add(weight as u32);
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if depot_full {
                                sim.trips.road_share = sim.trips.road_share.saturating_add(weight as u32);
                                apply_path_traffic(raw_traffic, &path, ROAD_TRAFFIC_FACTOR, weight);
                            } else {
                                sim.trips.bus_share = sim.trips.bus_share.saturating_add(weight as u32);
                                apply_path_traffic(raw_traffic, &path, BUS_TRAFFIC_FACTOR, weight);
                            }
                        }
                        TripMode::Rail => {
                            sim.trips.rail_share = sim.trips.rail_share.saturating_add(weight as u32);
                        }
                        TripMode::Subway => {
                            sim.trips.subway_share = sim.trips.subway_share.saturating_add(weight as u32);
                        }
                    }
                }
            }
            TargetTripResult::Failure(failure) => {
                simulation.failure = choose_failure(simulation.failure, failure);
                if developed {
                    sim.trips.failures = sim.trips.failures.saturating_add(weight as u32);
                }
            }
        }
    }

    simulation
}

fn attempt_target_trip(
    map: &Map,
    cache: &NetworkCache,
    access: &LotAccess,
    target_kind: ZoneKind,
    lot_state: &mut LotTripState,
    rng: &mut StdRng,
) -> TargetTripResult {
    let target = &cache.targets_by_kind[zone_index(target_kind)];
    // Transit modes are checked independently because each one has different local access,
    // connected components, and temporary cooldown state.
    let bus_eligibility = mode_eligibility(
        &access.bus_nodes,
        &cache.road_components,
        &target.bus_components,
    );
    let rail_eligibility = mode_eligibility(
        &access.rail_nodes,
        &cache.rail_components,
        &target.rail_components,
    );
    let subway_eligibility = mode_eligibility(
        &access.subway_nodes,
        &cache.subway_components,
        &target.subway_components,
    );
    let road_eligibility = mode_eligibility(
        &access.road_nodes,
        &cache.road_components,
        &target.road_components,
    );

    let any_local = [
        bus_eligibility.has_local,
        rail_eligibility.has_local,
        subway_eligibility.has_local,
        road_eligibility.has_local,
    ]
    .into_iter()
    .any(|value| value);
    if !any_local {
        return TargetTripResult::Failure(TripFailure::NoLocalAccess);
    }

    let any_destination = [
        bus_eligibility.has_destination,
        rail_eligibility.has_destination,
        subway_eligibility.has_destination,
        road_eligibility.has_destination,
    ]
    .into_iter()
    .any(|value| value);
    if !any_destination {
        return TargetTripResult::Failure(TripFailure::NoDestination);
    }

    let mut fallback_failure = None;

    for (mode, eligibility, starts, targets) in [
        (
            TripMode::Bus,
            bus_eligibility,
            access.bus_nodes.as_slice(),
            target.bus_nodes.as_slice(),
        ),
        (
            TripMode::Rail,
            rail_eligibility,
            access.rail_nodes.as_slice(),
            target.rail_nodes.as_slice(),
        ),
        (
            TripMode::Subway,
            subway_eligibility,
            access.subway_nodes.as_slice(),
            target.subway_nodes.as_slice(),
        ),
    ] {
        if !eligibility.eligible || lot_state.cooldown(mode) > 0 {
            continue;
        }
        // This preserves the SC2000 feel where transit is opportunistic rather than a
        // guaranteed preference whenever a station exists.
        if !rng.gen_bool(0.5) {
            continue;
        }

        match search_path(map, starts, targets, mode, MAX_TRIP_COST) {
            Ok(RouteSuccess { path, cost }) => {
                return TargetTripResult::Success { mode, path, cost };
            }
            Err(failure) => {
                lot_state.trigger_cooldown(mode);
                fallback_failure = choose_failure(fallback_failure, failure);
            }
        }
    }

    if road_eligibility.eligible {
        match search_path(
            map,
            &access.road_nodes,
            &target.road_nodes,
            TripMode::Road,
            MAX_TRIP_COST,
        ) {
            Ok(RouteSuccess { path, cost }) => {
                return TargetTripResult::Success {
                    mode: TripMode::Road,
                    path,
                    cost,
                };
            }
            Err(failure) => {
                fallback_failure = choose_failure(fallback_failure, failure);
            }
        }
    }

    if let Some(failure) = fallback_failure {
        return TargetTripResult::Failure(failure);
    }

    TargetTripResult::Failure(TripFailure::NoRoute)
}

// ── Mode eligibility ──────────────────────────────────────────────────────────

fn mode_eligibility(
    starts: &[usize],
    components: &[Option<u32>],
    target_components: &std::collections::HashSet<u32>,
) -> ModeEligibility {
    let has_local = !starts.is_empty();
    let has_destination = !target_components.is_empty();
    // Eligibility means "same connected network somewhere", not "specific route proven".
    // Pathfinding still runs afterward to catch detours that exceed the cost cap.
    let eligible = has_local
        && has_destination
        && starts.iter().any(|&node| {
            components[node]
                .map(|component| target_components.contains(&component))
                .unwrap_or(false)
        });

    ModeEligibility {
        has_local,
        has_destination,
        eligible,
    }
}

// ── Trip helpers ──────────────────────────────────────────────────────────────

pub(super) fn trip_lot_tile(map: &Map, x: usize, y: usize) -> Tile {
    map.surface_lot_tile(x, y)
}

fn trip_targets(tile: Tile) -> Option<&'static [ZoneKind]> {
    match tile {
        Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh => {
            Some(&[ZoneKind::Commercial, ZoneKind::Industrial])
        }
        Tile::ZoneComm | Tile::CommLow | Tile::CommHigh => Some(&[ZoneKind::Residential]),
        Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy => Some(&[ZoneKind::Residential]),
        _ => None,
    }
}

pub(super) fn is_trip_lot(tile: Tile) -> bool {
    tile.is_building() || tile.is_zone()
}

fn trip_weight(tile: Tile) -> u16 {
    // Weight is intentionally coarse. It gives larger buildings more influence without
    // simulating individual agents.
    tile_sector_capacity(tile)
        .map(|(_, amount)| (amount / 10).max(1) as u16)
        .unwrap_or(0)
}

fn apply_path_traffic(raw_traffic: &mut [u16], path: &[usize], factor: u16, weight: u16) {
    let load = factor.saturating_mul(weight.max(1));
    for &idx in path {
        raw_traffic[idx] = raw_traffic[idx].saturating_add(load);
    }
}

fn choose_failure(current: Option<TripFailure>, next: TripFailure) -> Option<TripFailure> {
    match current {
        // Keep the most structural failure so the UI shows the clearest reason growth failed.
        Some(existing) if failure_priority(existing) >= failure_priority(next) => Some(existing),
        _ => Some(next),
    }
}

fn failure_priority(failure: TripFailure) -> u8 {
    match failure {
        TripFailure::NoLocalAccess => 4,
        TripFailure::NoDestination => 3,
        TripFailure::TooLong => 2,
        TripFailure::NoRoute => 1,
    }
}

fn dominant_mode(mode_weights: &[u32; 4]) -> Option<TripMode> {
    let mut best = None;
    let mut best_weight = 0;
    for mode in [
        TripMode::Road,
        TripMode::Bus,
        TripMode::Rail,
        TripMode::Subway,
    ] {
        let weight = mode_weights[mode_index(mode)];
        if weight > best_weight {
            best = Some(mode);
            best_weight = weight;
        }
    }
    best
}

fn mode_index(mode: TripMode) -> usize {
    match mode {
        TripMode::Road => 0,
        TripMode::Bus => 1,
        TripMode::Rail => 2,
        TripMode::Subway => 3,
    }
}

pub(super) fn zone_index(kind: ZoneKind) -> usize {
    match kind {
        ZoneKind::Residential => 0,
        ZoneKind::Commercial => 1,
        ZoneKind::Industrial => 2,
    }
}

pub(super) fn xy(map: &Map, idx: usize) -> (usize, usize) {
    (idx % map.width, idx / map.width)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{TransportTile, ZoneSpec};
    use crate::core::sim::SimState;
    use rand::{rngs::StdRng, SeedableRng};

    fn run_transport(map: &mut Map, sim: &mut SimState) {
        let mut system = TransportSystem::default();
        system.tick(map, sim);
    }

    fn seed_for_mode(
        map: &Map,
        x: usize,
        y: usize,
        target_kind: ZoneKind,
        desired: TripMode,
    ) -> u64 {
        let cache = NetworkCache::build(map);
        let access = &cache.lot_access[map.idx(x, y)];
        for seed in 0..2048 {
            let mut state = LotTripState::default();
            let mut rng = StdRng::seed_from_u64(seed);
            if let TargetTripResult::Success { mode, .. } =
                attempt_target_trip(map, &cache, access, target_kind, &mut state, &mut rng)
            {
                if mode == desired {
                    return seed;
                }
            }
        }
        panic!(
            "no seed produced {:?} for target {:?}",
            desired, target_kind
        );
    }

    #[test]
    fn zoned_lots_gain_trip_success_before_first_growth() {
        let mut map = Map::new(6, 1);
        let mut sim = SimState::default();
        map.set_zone_spec(
            0,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: crate::core::map::ZoneDensity::Light,
            }),
        );
        map.set_transport(1, 0, Some(TransportTile::Road));
        map.set_transport(2, 0, Some(TransportTile::Road));
        map.set_zone_spec(
            4,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Industrial,
                density: crate::core::map::ZoneDensity::Light,
            }),
        );
        map.set_transport(3, 0, Some(TransportTile::Road));

        run_transport(&mut map, &mut sim);

        assert!(map.get_overlay(0, 0).trip_success);
    }

    #[test]
    fn isolated_highway_does_not_count_as_local_access() {
        let mut map = Map::new(8, 1);
        let mut sim = SimState::default();
        map.set_zone(0, 0, Some(ZoneKind::Residential));
        for x in 1..=5 {
            map.set_transport(x, 0, Some(TransportTile::Highway));
        }
        map.set_zone(7, 0, Some(ZoneKind::Commercial));

        run_transport(&mut map, &mut sim);

        let overlay = map.get_overlay(0, 0);
        assert!(!overlay.trip_success);
        assert_eq!(overlay.trip_failure, Some(TripFailure::NoLocalAccess));
    }

    #[test]
    fn connected_rail_networks_allow_transit_success() {
        let mut map = Map::new(10, 1);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::CommLow);
        map.set_occupant(1, 0, Some(Tile::RailDepot));
        for x in 2..=7 {
            map.set_transport(x, 0, Some(TransportTile::Rail));
        }
        map.set_occupant(8, 0, Some(Tile::RailDepot));
        map.set_zone(9, 0, Some(ZoneKind::Residential));
        sim.rng.transport = seed_for_mode(&map, 0, 0, ZoneKind::Residential, TripMode::Rail);

        run_transport(&mut map, &mut sim);

        assert!(map.get_overlay(0, 0).trip_success);
        assert_eq!(map.get_overlay(0, 0).trip_mode, Some(TripMode::Rail));
    }

    #[test]
    fn disconnected_rail_networks_fail_without_false_positive() {
        let mut map = Map::new(10, 2);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::CommLow);
        map.set_occupant(1, 0, Some(Tile::RailDepot));
        map.set_transport(2, 0, Some(TransportTile::Rail));
        map.set_zone(9, 0, Some(ZoneKind::Residential));
        map.set_occupant(8, 0, Some(Tile::RailDepot));
        map.set_transport(7, 0, Some(TransportTile::Rail));

        run_transport(&mut map, &mut sim);

        assert!(!map.get_overlay(0, 0).trip_success);
        assert_eq!(
            map.get_overlay(0, 0).trip_failure,
            Some(TripFailure::NoRoute)
        );
    }

    #[test]
    fn bus_trips_add_less_traffic_than_road_trips() {
        let mut road_map = Map::new(9, 1);
        let mut bus_map = Map::new(9, 1);
        let mut road_sim = SimState::default();
        let mut bus_sim = SimState::default();

        for map in [&mut road_map, &mut bus_map] {
            map.set(0, 0, Tile::CommHigh);
            for x in 1..=7 {
                map.set_transport(x, 0, Some(TransportTile::Road));
            }
            map.set(8, 0, Tile::ResHigh);
        }

        bus_map.set_occupant(1, 0, Some(Tile::BusDepot));
        bus_map.set_occupant(7, 0, Some(Tile::BusDepot));
        road_sim.rng.transport =
            seed_for_mode(&road_map, 0, 0, ZoneKind::Residential, TripMode::Road);
        bus_sim.rng.transport =
            seed_for_mode(&bus_map, 0, 0, ZoneKind::Residential, TripMode::Bus);

        run_transport(&mut road_map, &mut road_sim);
        run_transport(&mut bus_map, &mut bus_sim);

        assert_eq!(bus_map.get_overlay(0, 0).trip_mode, Some(TripMode::Bus));
        assert!(bus_map.get_overlay(3, 0).traffic < road_map.get_overlay(3, 0).traffic);
    }

    #[test]
    fn failed_transit_mode_enters_one_month_cooldown() {
        let mut map = Map::new(60, 1);
        map.set(0, 0, Tile::CommLow);
        for x in 1..=58 {
            map.set_transport(x, 0, Some(TransportTile::Road));
        }
        map.set_occupant(1, 0, Some(Tile::BusDepot));
        map.set_occupant(58, 0, Some(Tile::BusDepot));
        map.set_zone(59, 0, Some(ZoneKind::Residential));

        let cache = NetworkCache::build(&map);
        let access = &cache.lot_access[map.idx(0, 0)];
        let mut state = LotTripState::default();
        let mut seed = None;
        for candidate in 0..2048 {
            let mut candidate_state = LotTripState::default();
            let mut rng = StdRng::seed_from_u64(candidate);
            let result = attempt_target_trip(
                &map,
                &cache,
                access,
                ZoneKind::Residential,
                &mut candidate_state,
                &mut rng,
            );
            if matches!(result, TargetTripResult::Failure(TripFailure::TooLong))
                && candidate_state.bus_cooldown == 2
            {
                seed = Some(candidate);
                break;
            }
        }

        let mut rng = StdRng::seed_from_u64(seed.expect("expected a bus failure seed"));
        let first = attempt_target_trip(
            &map,
            &cache,
            access,
            ZoneKind::Residential,
            &mut state,
            &mut rng,
        );
        assert!(matches!(
            first,
            TargetTripResult::Failure(TripFailure::TooLong)
        ));
        assert_eq!(state.bus_cooldown, 2);

        state.advance_month();
        let mut next_rng = StdRng::seed_from_u64(0);
        let second = attempt_target_trip(
            &map,
            &cache,
            access,
            ZoneKind::Residential,
            &mut state,
            &mut next_rng,
        );
        assert_eq!(state.bus_cooldown, 1);
        assert!(matches!(
            second,
            TargetTripResult::Failure(TripFailure::TooLong)
        ));
    }

    #[test]
    fn bus_depot_at_capacity_trips_fall_back_to_roads() {
        let mut map = Map::new(15, 1);
        map.set(0, 0, Tile::CommHigh);
        for x in 1..=8 {
            map.set_transport(x, 0, Some(TransportTile::Road));
        }
        map.set(2, 0, Tile::BusDepot);
        map.set(9, 0, Tile::ResHigh);

        let mut sim = SimState::default();
        sim.depots.insert(
            (2, 0),
            crate::core::sim::DepotState {
                trips_used: BUS_DEPOT_CAPACITY,
            },
        );
        sim.rng.transport = 0xABCD;

        run_transport(&mut map, &mut sim);

        assert_eq!(sim.trips.bus_share, 0);
        assert!(
            sim.trips.road_share > 0,
            "trips should fall back to roads when depot is at capacity"
        );
    }

    #[test]
    fn bus_depot_trips_used_resets_each_month() {
        let mut map = Map::new(20, 1);
        map.set(0, 0, Tile::CommHigh);
        for x in 1..=18 {
            map.set_transport(x, 0, Some(TransportTile::Road));
        }
        map.set(19, 0, Tile::ResHigh);
        map.set_occupant(1, 0, Some(Tile::BusDepot));

        let mut sim = SimState::default();
        sim.depots.insert(
            (1, 0),
            crate::core::sim::DepotState {
                trips_used: BUS_DEPOT_CAPACITY - 1,
            },
        );

        let mut system = TransportSystem::default();
        system.tick(&mut map, &mut sim);

        let state = sim.depots.get(&(1, 0)).unwrap();
        assert_eq!(
            state.trips_used, 0,
            "depot trips_used should reset to 0 at the start of each month"
        );
    }

    #[test]
    fn depot_not_in_sim_depots_map_does_not_crash() {
        let mut map = Map::new(15, 1);
        map.set(0, 0, Tile::CommHigh);
        for x in 1..=10 {
            map.set_transport(x, 0, Some(TransportTile::Road));
        }
        map.set(3, 0, Tile::BusDepot);
        map.set(9, 0, Tile::ResHigh);

        let mut sim = SimState::default();
        sim.rng.transport = 0x1234;

        let mut system = TransportSystem::default();
        system.tick(&mut map, &mut sim);

        assert!(
            sim.depots.contains_key(&(3, 0)),
            "depot should be auto-registered in sim.depots even if not placed via engine"
        );
    }

    #[test]
    fn depot_with_capacity_accepts_bus_trips() {
        let mut map = Map::new(7, 7);
        map.set(0, 3, Tile::CommHigh);
        for y in 0..7 {
            for x in 1..=5 {
                map.set_transport(x, y, Some(TransportTile::Road));
            }
        }
        map.set(3, 3, Tile::BusDepot);
        map.set(6, 3, Tile::ResHigh);

        let mut sim = SimState::default();
        sim.depots
            .insert((3, 3), crate::core::sim::DepotState { trips_used: 0 });
        sim.rng.transport = 0xABCD;

        let mut system = TransportSystem::default();
        system.tick(&mut map, &mut sim);

        assert!(
            sim.trips.bus_share > 0,
            "depot with available capacity should accept bus trips"
        );
        assert!(
            sim.depots.get(&(3, 3)).map(|s| s.trips_used).unwrap_or(0) > 0,
            "depot trips_used should be incremented when bus trips succeed"
        );
    }

    #[test]
    fn second_depot_takes_over_when_first_is_full() {
        let mut map = Map::new(10, 10);
        map.set(0, 5, Tile::CommHigh);
        for y in 0..10 {
            for x in 1..=8 {
                map.set_transport(x, y, Some(TransportTile::Road));
            }
        }
        map.set(3, 5, Tile::BusDepot);
        map.set(7, 5, Tile::BusDepot);
        map.set(9, 5, Tile::ResHigh);

        let mut sim = SimState::default();
        sim.depots.insert(
            (3, 5),
            crate::core::sim::DepotState {
                trips_used: BUS_DEPOT_CAPACITY,
            },
        );
        sim.depots
            .insert((7, 5), crate::core::sim::DepotState { trips_used: 0 });
        sim.rng.transport = 0xABCD;

        run_transport(&mut map, &mut sim);

        assert!(
            sim.trips.bus_share > 0,
            "second depot should accept trips when first is at capacity"
        );
    }

    #[test]
    fn rail_depot_trips_used_not_incremented_by_bus_logic() {
        let mut map = Map::new(7, 7);
        map.set(0, 3, Tile::CommHigh);
        for y in 0..7 {
            for x in 1..=5 {
                map.set_transport(x, y, Some(TransportTile::Road));
            }
        }
        map.set(3, 3, Tile::RailDepot);
        map.set(6, 3, Tile::ResHigh);

        let mut sim = SimState::default();
        sim.rng.transport = 0x5678;

        run_transport(&mut map, &mut sim);

        assert_eq!(
            sim.trips.rail_share, 0,
            "rail trips require rail network, not road"
        );
    }

    #[test]
    fn depot_at_capacity_boundary() {
        let mut map = Map::new(7, 7);
        map.set(0, 3, Tile::CommLow);
        for y in 0..7 {
            for x in 1..=5 {
                map.set_transport(x, y, Some(TransportTile::Road));
            }
        }
        map.set(3, 3, Tile::BusDepot);
        map.set(6, 3, Tile::ResLow);

        let mut sim = SimState::default();
        sim.depots.insert(
            (3, 3),
            crate::core::sim::DepotState {
                trips_used: BUS_DEPOT_CAPACITY - 1,
            },
        );
        sim.rng.transport = 0xDEAD;

        run_transport(&mut map, &mut sim);

        let state = sim.depots.get(&(3, 3)).unwrap();
        assert_eq!(
            state.trips_used, 1,
            "after one tick, trips_used should be 1 (depot resets each tick)"
        );
        assert!(
            sim.trips.bus_share > 0 || sim.trips.road_share > 0,
            "at least some trips should succeed"
        );
    }
}
