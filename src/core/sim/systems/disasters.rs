use crate::core::map::{Map, Tile};
use crate::core::sim::util::for_each_in_radius;
use crate::core::sim::constants::{
    FIRE_DAMAGE_CHANCE, FIRE_IGNITE_CHANCE_MAX, FIRE_SPREAD_CHANCE_MAX,
    FIRE_SPREAD_SUPPRESS_RADIUS, FIRE_SUPPRESS_CHANCE_BASE, FLOOD_TRIGGER_CHANCE,
    TORNADO_DRIFT_CHANCE, TORNADO_TRIGGER_CHANCE,
};
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;
use rand::{rngs::StdRng, Rng, SeedableRng};

// ── FireSpreadSystem ──────────────────────────────────────────────────────────
// Active fire disaster: spontaneous ignition, spreading, tile damage,
// and suppression by nearby fire stations.

/// Phase 1: Buildings with high fire_risk may spontaneously ignite (~0.02% at max risk).
fn ignite_spontaneous(map: &mut Map, rng: &mut StdRng) {
    let w = map.width;
    let h = map.height;
    for y in 0..h {
        for x in 0..w {
            if !map.tiles[y * w + x].is_building() {
                continue;
            }
            let o = &map.overlays[y * w + x];
            if o.on_fire {
                continue;
            }
            let chance = (o.fire_risk as f32 / 255.0) * FIRE_IGNITE_CHANCE_MAX;
            if rng.gen::<f32>() < chance {
                map.overlays[y * w + x].on_fire = true;
            }
        }
    }
}

/// Phase 2: Active fires spread to adjacent buildings based on their fire_risk.
fn spread_fires(map: &mut Map, rng: &mut StdRng) {
    let w = map.width;
    let h = map.height;
    let mut new_fires: Vec<(usize, usize)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if !map.overlays[y * w + x].on_fire {
                continue;
            }
            for (nx, ny, tile) in map.neighbors4(x, y) {
                if tile.is_building() && !map.overlays[ny * w + nx].on_fire {
                    let spread_chance =
                        map.overlays[ny * w + nx].fire_risk as f32 / 255.0 * FIRE_SPREAD_CHANCE_MAX;
                    if rng.gen::<f32>() < spread_chance {
                        new_fires.push((nx, ny));
                    }
                }
            }
        }
    }
    for (x, y) in new_fires {
        map.overlays[y * w + x].on_fire = true;
    }
}

/// Phase 3: Burning buildings have a 1% chance per tick to be destroyed (downgraded one tier).
fn apply_fire_damage(map: &mut Map, rng: &mut StdRng) {
    let w = map.width;
    let h = map.height;
    let mut damaged: Vec<(usize, usize)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if map.overlays[y * w + x].on_fire && rng.gen::<f32>() < FIRE_DAMAGE_CHANCE {
                damaged.push((x, y));
            }
        }
    }
    for (x, y) in damaged {
        let downgraded = match map.tiles[y * w + x] {
            Tile::ResHigh => Some(Tile::ResMed),
            Tile::ResMed => Some(Tile::ResLow),
            Tile::ResLow => Some(Tile::ZoneRes),
            Tile::CommHigh => Some(Tile::CommLow),
            Tile::CommLow => Some(Tile::ZoneComm),
            Tile::IndHeavy => Some(Tile::IndLight),
            Tile::IndLight => Some(Tile::ZoneInd),
            _ => Some(Tile::Grass),
        };
        if let Some(t) = downgraded {
            map.set(x, y, t);
            map.overlays[y * w + x].on_fire = false; // fire consumes the building
        }
    }
}

/// Phase 4: Fire stations suppress active fires within their service radius.
fn suppress_with_stations(map: &mut Map, rng: &mut StdRng) {
    let w = map.width;
    let h = map.height;
    // Collect suppression targets first; for_each_in_radius borrows map immutably
    // so we cannot also hold a mutable borrow on rng inside the closure.
    let mut suppress_targets: Vec<(usize, f32)> = Vec::new();
    let fire_stations: Vec<(usize, usize)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| map.get(x, y) == Tile::Fire)
        .collect();
    for (sx, sy) in fire_stations {
        for_each_in_radius(map, sx, sy, FIRE_SPREAD_SUPPRESS_RADIUS, |_nx, _ny, idx, falloff| {
            if map.overlays[idx].on_fire {
                suppress_targets.push((idx, FIRE_SUPPRESS_CHANCE_BASE * falloff));
            }
        });
    }
    for (idx, suppress_chance) in suppress_targets {
        if rng.gen::<f32>() < suppress_chance {
            map.overlays[idx].on_fire = false;
        }
    }
}

#[derive(Debug)]
pub struct FireSpreadSystem;
impl SimSystem for FireSpreadSystem {
    fn name(&self) -> &str {
        "FireSpread"
    }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        if !sim.disasters.fire_enabled {
            return;
        }
        let mut rng = StdRng::seed_from_u64(sim.rng.disaster);
        ignite_spontaneous(map, &mut rng);
        spread_fires(map, &mut rng);
        apply_fire_damage(map, &mut rng);
        suppress_with_stations(map, &mut rng);
        sim.rng.disaster = rng.gen();
    }
}

// ── FloodSystem ───────────────────────────────────────────────────────────────
// Once per year, small chance that water floods one adjacent tile per water body.

#[derive(Debug)]
pub struct FloodSystem;
impl SimSystem for FloodSystem {
    fn name(&self) -> &str {
        "Flood"
    }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        if !sim.disasters.flood_enabled {
            return;
        }
        let mut rng = StdRng::seed_from_u64(sim.rng.disaster);

        // Only trigger at month 6, with a ~10% annual chance
        if sim.month == 6 && rng.gen::<f32>() < FLOOD_TRIGGER_CHANCE {
            // Collect all water-adjacent non-water, non-road tiles
            let mut floodable: Vec<(usize, usize)> = Vec::new();
            for y in 0..map.height {
                for x in 0..map.width {
                    if map.get(x, y) == Tile::Water {
                        continue;
                    }
                    if matches!(
                        map.get(x, y),
                        Tile::Road
                            | Tile::Rail
                            | Tile::RoadPowerLine
                            | Tile::Highway
                            | Tile::Onramp
                    ) {
                        continue;
                    }
                    let near_water = map
                        .neighbors4(x, y)
                        .iter()
                        .any(|(_, _, t)| *t == Tile::Water);
                    if near_water {
                        floodable.push((x, y));
                    }
                }
            }

            // Flood a random selection (up to 5 tiles per event)
            let count = rng.gen_range(1..=5_usize.min(floodable.len().max(1)));
            for _ in 0..count {
                if floodable.is_empty() {
                    break;
                }
                let i = rng.gen_range(0..floodable.len());
                let (fx, fy) = floodable.swap_remove(i);
                map.set(fx, fy, Tile::Water);
                map.overlays[fy * map.width + fx].on_fire = false;
            }
        }

        sim.rng.disaster = rng.gen();
    }
}

// ── TornadoSystem ─────────────────────────────────────────────────────────────
// Very rare event: a tornado carves a random path across the map, bulldozing tiles.

#[derive(Debug)]
pub struct TornadoSystem;
impl SimSystem for TornadoSystem {
    fn name(&self) -> &str {
        "Tornado"
    }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        if !sim.disasters.tornado_enabled {
            return;
        }
        let mut rng = StdRng::seed_from_u64(sim.rng.disaster);

        // ~2% chance per year, checked on month 3
        if sim.month == 3 && rng.gen::<f32>() < TORNADO_TRIGGER_CHANCE {
            // Random starting edge
            let (mut x, mut y) = match rng.gen_range(0..4) {
                0 => (rng.gen_range(0..map.width), 0),
                1 => (rng.gen_range(0..map.width), map.height - 1),
                2 => (0, rng.gen_range(0..map.height)),
                _ => (map.width - 1, rng.gen_range(0..map.height)),
            };

            // Random direction (biased toward centre)
            let cx = map.width as i32 / 2;
            let cy = map.height as i32 / 2;
            let mut dx = ((cx - x as i32).signum() + rng.gen_range(-1..=1)).clamp(-1, 1);
            let dy = ((cy - y as i32).signum() + rng.gen_range(-1..=1)).clamp(-1, 1);
            if dx == 0 && dy == 0 {
                dx = 1;
            }

            let path_len = rng.gen_range(12..30_usize);
            for _ in 0..path_len {
                if !map.in_bounds(x as i32, y as i32) {
                    break;
                }
                // Destroy non-water tiles in a 3-tile-wide swath
                for wy in -1_i32..=1 {
                    for wx in -1_i32..=1 {
                        let nx = x as i32 + wx;
                        let ny = y as i32 + wy;
                        if map.in_bounds(nx, ny) {
                            let t = map.get(nx as usize, ny as usize);
                            if t != Tile::Water {
                                // Bulldoze to zone or grass
                                let rubble = match t {
                                    Tile::ResLow | Tile::ResMed | Tile::ResHigh => Tile::ZoneRes,
                                    Tile::CommLow | Tile::CommHigh => Tile::ZoneComm,
                                    Tile::IndLight | Tile::IndHeavy => Tile::ZoneInd,
                                    _ => Tile::Grass,
                                };
                                map.set(nx as usize, ny as usize, rubble);
                                map.overlays[ny as usize * map.width + nx as usize].on_fire = false;
                            }
                        }
                    }
                }
                // Step + slight random drift
                x = (x as i32 + dx).max(0) as usize;
                y = (y as i32 + dy).max(0) as usize;
                if rng.gen::<f32>() < TORNADO_DRIFT_CHANCE {
                    dx = (dx + rng.gen_range(-1..=1)).clamp(-1, 1);
                    if dx == 0 && dy == 0 {
                        dx = 1;
                    }
                }
            }
        }

        sim.rng.disaster = rng.gen();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::system::SimSystem;
    use crate::core::sim::SimState;

    #[test]
    fn disaster_rng_is_deterministic_with_fixed_seed() {
        let mut map = Map::new(30, 30);
        let mut sim = SimState::default();
        sim.disasters.fire_enabled = true;
        sim.disasters.flood_enabled = true;
        sim.disasters.tornado_enabled = true;
        sim.month = 6; // flood triggers in month 6, tornado in month 3

        // Place some buildings for fire spread
        map.set(15, 15, Tile::ResHigh);
        map.overlays[15 * 30 + 15].fire_risk = 200;

        let mut fire_results: Vec<Vec<(usize, usize)>> = Vec::new();

        for _run in 0..3 {
            let mut sim_run = SimState::default();
            sim_run.disasters.fire_enabled = true;
            sim_run.disasters.flood_enabled = true;
            sim_run.disasters.tornado_enabled = true;
            sim_run.month = 6;
            sim_run.rng.disaster = 0xDEADBEEF;

            let mut test_map = map.clone();
            FireSpreadSystem.tick(&mut test_map, &mut sim_run);

            let fires: Vec<_> = (0..test_map.height)
                .flat_map(|y| (0..test_map.width).map(move |x| (x, y)))
                .filter(|(x, y)| test_map.overlays[*y * 30 + *x].on_fire)
                .collect();
            fire_results.push(fires);
        }

        // All three runs with the same seed should produce identical fire results
        assert_eq!(
            fire_results[0], fire_results[1],
            "fire spread must be deterministic with fixed seed"
        );
        assert_eq!(
            fire_results[1], fire_results[2],
            "fire spread must be deterministic with fixed seed"
        );
    }

    #[test]
    fn disaster_rng_state_changes_after_tick() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();
        sim.disasters.fire_enabled = true;
        sim.rng.disaster = 0x12345678;

        let initial_state = sim.rng.disaster;
        FireSpreadSystem.tick(&mut map, &mut sim);

        assert_ne!(
            sim.rng.disaster, initial_state,
            "disaster_rng_state must be mutated after FireSpreadSystem tick"
        );
    }

    #[test]
    fn flood_system_uses_seeded_rng() {
        let mut map = Map::new(10, 10);
        map.set(5, 5, Tile::Water);
        // Surround water with buildable tiles
        for dy in -2isize..=2 {
            for dx in -2isize..=2 {
                let nx = (5isize + dx) as usize;
                let ny = (5isize + dy) as usize;
                if map.in_bounds(nx as i32, ny as i32) && map.get(nx, ny) != Tile::Water {
                    map.set(nx, ny, Tile::Grass);
                }
            }
        }

        let seed = 0xCAFEBABE;
        let mut results = Vec::new();
        for _ in 0..3 {
            let mut sim = SimState::default();
            sim.disasters.flood_enabled = true;
            sim.month = 6;
            sim.rng.disaster = seed;

            let mut test_map = map.clone();
            FloodSystem.tick(&mut test_map, &mut sim);
            results.push(test_map.get(4, 5));
        }

        assert_eq!(
            results[0], results[1],
            "flood must be deterministic with fixed seed"
        );
        assert_eq!(
            results[1], results[2],
            "flood must be deterministic with fixed seed"
        );
    }

    #[test]
    fn fire_suppression_at_exactly_12_tiles_has_zero_probability() {
        // At exactly distance 12: falloff = 1 - 144/144 = 0
        // suppress_chance = 0.08 * 0 = 0
        // Fire at exactly the radius boundary should NEVER be suppressed.
        let mut map = Map::new(30, 30);
        let mut sim = SimState::default();
        sim.disasters.fire_enabled = true;

        map.set(5, 5, Tile::Fire);
        map.set(17, 5, Tile::ResLow);
        map.overlays[17 * 30 + 5].on_fire = true;

        for seed in 0..50u64 {
            let mut test_sim = SimState::default();
            test_sim.disasters.fire_enabled = true;
            test_sim.rng.disaster = seed;
            let mut test_map = map.clone();
            FireSpreadSystem.tick(&mut test_map, &mut test_sim);
            assert!(
                test_map.overlays[17 * 30 + 5].on_fire,
                "fire at exactly distance 12 should NEVER be suppressed (falloff=0), but was extinguished with seed {seed}"
            );
        }
    }

    #[test]
    fn fire_at_distance_11_is_within_suppression_range() {
        // Fire station at (5,5), fire at (16,5) — distance 11
        // falloff = 1 - 121/144 = 0.16, suppress_chance = 0.08 * 0.16 = 0.0128
        // Run many times to confirm suppression CAN happen (probabilistic)
        let mut suppressed_count = 0;
        for seed in 0..200u64 {
            let mut test_sim = SimState::default();
            test_sim.disasters.fire_enabled = true;
            test_sim.rng.disaster = seed;
            let mut test_map = Map::new(30, 30);
            test_map.set(5, 5, Tile::Fire);
            test_map.set(16, 5, Tile::ResLow);
            test_map.overlays[16 * 30 + 5].on_fire = true;
            FireSpreadSystem.tick(&mut test_map, &mut test_sim);
            if !test_map.overlays[16 * 30 + 5].on_fire {
                suppressed_count += 1;
            }
        }
        assert!(
            suppressed_count > 0,
            "fire at distance 11 should be suppressible — suppressed {}/200 times",
            suppressed_count
        );
    }

    #[test]
    fn fire_beyond_radius_12_not_suppressed_but_can_burn_out() {
        // Fire station at (5,5), fire at (18,5) — distance 13
        // Station range: x ∈ [-7,17], y ∈ [-7,17] — (18,5) is outside
        // Fire can still be extinguished by fire damage (1% per tick), not suppression
        let mut fire_still_burning = 0;
        for seed in 0..100u64 {
            let mut test_sim = SimState::default();
            test_sim.disasters.fire_enabled = true;
            test_sim.rng.disaster = seed;
            let mut test_map = Map::new(30, 30);
            test_map.set(5, 5, Tile::Fire);
            test_map.set(18, 5, Tile::ResLow);
            test_map.overlays[18 * 30 + 5].on_fire = true;
            FireSpreadSystem.tick(&mut test_map, &mut test_sim);
            if test_map.overlays[18 * 30 + 5].on_fire {
                fire_still_burning += 1;
            }
        }
        assert!(
            fire_still_burning < 100,
            "fire at distance 13 should sometimes burn out (via fire damage, not suppression) — still burning {}/100",
            fire_still_burning
        );
        assert!(
            fire_still_burning > 0,
            "fire at distance 13 is NOT in suppression range, so should persist at least sometimes"
        );
    }

    #[test]
    fn fire_station_on_map_corner_suppresses_nearby_fires() {
        // Fire station at (0,0) on map corner
        // Fire at (5,0) — distance 5, well within range
        // Falloff = 1 - 25/144 = 0.83, suppress_chance = 0.066
        let mut suppressed_count = 0;
        for seed in 0..200u64 {
            let mut test_sim = SimState::default();
            test_sim.disasters.fire_enabled = true;
            test_sim.rng.disaster = seed;
            let mut test_map = Map::new(20, 20);
            test_map.set(0, 0, Tile::Fire);
            test_map.set(5, 0, Tile::ResLow);
            test_map.overlays[5 * 20 + 0].on_fire = true;
            FireSpreadSystem.tick(&mut test_map, &mut test_sim);
            if !test_map.overlays[5 * 20 + 0].on_fire {
                suppressed_count += 1;
            }
        }
        assert!(
            suppressed_count > 0,
            "corner fire station should suppress nearby fires — suppressed {}/200",
            suppressed_count
        );
    }
}
