use crate::core::map::{Map, ResourceRole, Tile};
use crate::core::sim::constants::EOL_DECAY_MONTHS;
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── PowerSystem ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PowerSystem;

impl SimSystem for PowerSystem {
    fn name(&self) -> &str {
        "Power"
    }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        // 1. Age plants, compute efficiency decay, handle explosion
        let mut to_remove = Vec::new();
        // (x, y, footprint) tuples for explosion processing
        let mut exploded: Vec<(usize, usize, usize)> = Vec::new();

        for (&(x, y), state) in sim.plants.iter_mut() {
            state.age_months += 1;
            let remaining = state.max_life_months.saturating_sub(state.age_months);
            state.efficiency = if remaining < EOL_DECAY_MONTHS {
                remaining as f32 / EOL_DECAY_MONTHS as f32 // uses constants::EOL_DECAY_MONTHS
            } else {
                1.0
            };
            if state.age_months >= state.max_life_months {
                exploded.push((x, y, state.footprint as usize));
                to_remove.push((x, y));
            }
        }

        for (x, y, fp) in exploded {
            // Explode: replace footprint area with Rubble
            for dy in 0..fp {
                for dx in 0..fp {
                    if map.in_bounds(x as i32 + dx as i32, y as i32 + dy as i32) {
                        map.set(x + dx, y + dy, Tile::Rubble);
                    }
                }
            }
        }

        for pos in to_remove {
            sim.plants.remove(&pos);
        }

        // 2. Reset service overlays (power, water, trip state) in one pass
        map.reset_service_overlays();

        // 3. Calculate total effective production and distribute
        let mut total_capacity = 0;
        // (x, y, footprint) for BFS seeding
        let mut plant_positions: Vec<(usize, usize, usize)> = Vec::new();
        for (&(x, y), state) in sim.plants.iter() {
            let effective = (state.capacity_mw as f32 * state.efficiency) as u32;
            total_capacity += effective;
            let fp = state.footprint as usize;
            plant_positions.push((x, y, fp));
            // Mark all footprint tiles with current efficiency for the renderer
            let eff_u8 = (state.efficiency * 255.0) as u8;
            for dy in 0..fp {
                for dx in 0..fp {
                    if map.in_bounds(x as i32 + dx as i32, y as i32 + dy as i32) {
                        let idx = (y + dy) * map.width + (x + dx);
                        map.overlays[idx].plant_efficiency = eff_u8;
                    }
                }
            }
        }
        sim.utilities.power_produced_mw = total_capacity;

        // SC2000-style conduction using ResourceRole:
        // - Producer: seeds BFS at 255 (handled above)
        // - Conductor { falloff }: relays with per-type level drop
        // - Consumer: receives but does NOT relay onward
        // - None: not part of power grid
        let mut queue = std::collections::VecDeque::new();
        for (px, py, fp) in plant_positions {
            for dy in 0..fp {
                for dx in 0..fp {
                    let sx = px + dx;
                    let sy = py + dy;
                    if map.in_bounds(sx as i32, sy as i32) {
                        let idx = sy * map.width + sx;
                        map.overlays[idx].power_level = 255;
                        queue.push_back((sx, sy, 255u8));
                    }
                }
            }
        }

        while let Some((x, y, level)) = queue.pop_front() {
            if level <= 1 {
                continue;
            }

            for (nx, ny, tile) in map.neighbors4(x, y) {
                let n_idx = ny * map.width + nx;
                let lot_tile = map.surface_lot_tile(nx, ny);

                // Determine role: rendered tile (for PowerLine on roads) or
                // logical surface tile (for buildings/zones).
                let role = if tile.power_connects() {
                    tile.power_role()
                } else {
                    lot_tile.power_role()
                };

                let falloff = match role {
                    ResourceRole::None => continue,
                    ResourceRole::Producer => continue,
                    ResourceRole::Consumer => {
                        // Receives but does not relay — use the smallest
                        // conductor falloff as a proxy for the reception
                        // step (the tile still loses level crossing the
                        // boundary).
                        let next_level = level.saturating_sub(1);
                        if next_level > map.overlays[n_idx].power_level {
                            map.overlays[n_idx].power_level = next_level;
                        }
                        continue;
                    }
                    ResourceRole::Conductor { falloff } => falloff,
                };

                let next_level = level.saturating_sub(falloff);
                if next_level == 0 || next_level <= map.overlays[n_idx].power_level {
                    continue;
                }

                map.overlays[n_idx].power_level = next_level;
                queue.push_back((nx, ny, next_level));
            }
        }

        // Brownouts depend on connected demand, not disconnected buildings elsewhere on the map.
        let mut connected_demand = 0u32;
        for y in 0..map.height {
            for x in 0..map.width {
                let lot_tile = map.surface_lot_tile(x, y);
                let consumption = lot_tile.power_demand();
                if consumption == 0 {
                    continue;
                }

                let idx = y * map.width + x;
                if map.overlays[idx].power_level > 0 {
                    connected_demand += consumption;
                }
            }
        }
        sim.utilities.power_consumed_mw = connected_demand;

        // NOTE: power_level overlay stores raw BFS signal strength (reach).
        // Brownout (supply < demand) is communicated via sim.utilities and
        // consumed by the growth system separately — it is NOT baked into
        // the per-tile overlay.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{ZoneDensity, ZoneKind, ZoneSpec};
    use crate::core::sim::constants::{POWER_FALLOFF_BUILDING, POWER_FALLOFF_LINE, POWER_FALLOFF_ZONE};
    use crate::core::sim::system::SimSystem;
    use crate::core::sim::{PlantState, SimState};

    #[test]
    fn power_decay_over_distance() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();

        // Place a Coal plant at (0,0)
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }

        // Place a long line of power lines
        for x in 4..15 {
            map.set(x, 0, Tile::PowerLine);
        }

        PowerSystem.tick(&mut map, &mut sim);

        let level_near = map.get_overlay(4, 0).power_level;
        let level_far = map.get_overlay(14, 0).power_level;

        assert!(
            level_near > level_far,
            "Power level should decay over distance ({} > {})",
            level_near,
            level_far
        );
        assert!(
            level_far > 0,
            "Power should still reach the end of the line"
        );
    }

    #[test]
    fn power_plant_expiration_and_rubble() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();

        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 599,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }

        PowerSystem.tick(&mut map, &mut sim);

        // After one tick, it should have reached 600 and exploded
        assert!(sim.plants.is_empty(), "Plant should be removed from state");
        assert_eq!(
            map.get(0, 0),
            Tile::Rubble,
            "Tile should be replaced with Rubble"
        );
    }

    #[test]
    fn power_brownout_reported_in_utilities() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();

        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }
        map.set(4, 0, Tile::PowerLine);

        // Heavy industry consumes 400 MW each → 800 MW > 500 MW supply
        map.set(5, 0, Tile::IndHeavy);
        map.set(5, 1, Tile::IndHeavy);

        PowerSystem.tick(&mut map, &mut sim);

        // Overlay shows raw BFS signal (not brownout-scaled)
        let level = map.get_overlay(5, 0).power_level;
        assert!(
            level > 200,
            "Raw signal at adjacent consumer should be high (got {})",
            level
        );
        // Brownout is reported via supply/demand in utilities
        assert!(
            sim.utilities.power_consumed_mw > sim.utilities.power_produced_mw,
            "Utilities should report brownout: consumed={} > produced={}",
            sim.utilities.power_consumed_mw,
            sim.utilities.power_produced_mw
        );
    }

    #[test]
    fn power_buildings_relay_to_adjacent_zones_but_empty_zones_do_not_chain() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();

        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ResLow);
        map.set(6, 0, Tile::ZoneRes);
        map.set(7, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).is_powered(),
            "Building should receive power"
        );
        assert!(
            map.get_overlay(6, 0).is_powered(),
            "Adjacent empty zone should receive power from a powered building"
        );
        assert!(
            map.get_overlay(7, 0).is_powered(),
            "Empty zones now relay power, so a zone adjacent to a powered zone should also be powered"
        );
    }

    #[test]
    fn disconnected_load_does_not_brown_out_connected_grid() {
        let mut map = Map::new(12, 12);
        let mut sim = SimState::default();

        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::IndHeavy);
        map.set(10, 10, Tile::IndHeavy);

        PowerSystem.tick(&mut map, &mut sim);

        assert_eq!(
            sim.utilities.power_consumed_mw, 400,
            "Only connected demand should count toward load"
        );
        assert!(
            map.get_overlay(5, 0).power_level > 200,
            "Connected load should stay at full strength when capacity exceeds connected demand"
        );
        assert_eq!(
            map.get_overlay(10, 10).power_level,
            0,
            "Disconnected load should remain unpowered"
        );
    }

    // ── Plant efficiency decay ───────────────────────────────────────────────────

    #[test]
    fn plant_efficiency_full_life() {
        let mut map = Map::new(4, 4);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 100,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }
        PowerSystem.tick(&mut map, &mut sim);
        assert_eq!(
            sim.plants.get(&(0, 0)).unwrap().efficiency,
            1.0,
            "New plant has full efficiency"
        );
    }

    #[test]
    fn plant_efficiency_eol_boundary() {
        let mut map = Map::new(4, 4);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 587, // → remaining=12 after tick (age incremented first)
                max_life_months: 600,
                capacity_mw: 100,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }
        PowerSystem.tick(&mut map, &mut sim);
        let eff = sim.plants.get(&(0, 0)).unwrap().efficiency;
        assert_eq!(
            eff, 1.0,
            "At exactly 12 remaining months, efficiency is 1.0"
        );

        PowerSystem.tick(&mut map, &mut sim);
        let eff = sim.plants.get(&(0, 0)).unwrap().efficiency;
        assert!(
            (eff - 11.0 / 12.0).abs() < 0.001,
            "At 11 remaining months, efficiency should be 11/12 (got {})",
            eff
        );
    }

    #[test]
    fn plant_efficiency_one_month() {
        let mut map = Map::new(4, 4);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 598, // → remaining=1 after tick (age incremented first)
                max_life_months: 600,
                capacity_mw: 100,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }
        PowerSystem.tick(&mut map, &mut sim);
        let eff = sim.plants.get(&(0, 0)).unwrap().efficiency;
        assert!(
            (eff - 1.0 / 12.0).abs() < 0.001,
            "At 1 remaining month, efficiency should be 1/12 (got {})",
            eff
        );
    }

    #[test]
    fn plant_efficiency_map_overlay_all_16_tiles() {
        let mut map = Map::new(8, 8);
        let mut sim = SimState::default();
        sim.plants.insert(
            (1, 1),
            PlantState {
                age_months: 594, // → remaining=5 after tick → efficiency = 5/12, plant not exploded
                max_life_months: 600,
                capacity_mw: 120,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 1..5 {
            for dx in 1..5 {
                map.set(dx, dy, Tile::PowerPlantGas);
            }
        }
        PowerSystem.tick(&mut map, &mut sim);

        for dy in 1..5 {
            for dx in 1..5 {
                let eff = map.get_overlay(dx, dy).plant_efficiency;
                assert!(
                    eff < 255 && eff > 0,
                    "All footprint tiles should have degraded efficiency (tile ({},{}) has {})",
                    dx,
                    dy,
                    eff
                );
            }
        }
        assert_eq!(
            map.get_overlay(0, 1).plant_efficiency,
            0,
            "Non-footprint tile outside x-range retains default 0"
        );
        assert_eq!(
            map.get_overlay(5, 1).plant_efficiency,
            0,
            "Non-footprint tile outside x-range retains default 0"
        );
    }

    #[test]
    fn plant_efficiency_degraded_reports_brownout() {
        let mut map = Map::new(6, 6);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 592, // → remaining=7 after tick → efficiency = 7/12 ≈ 0.583
                max_life_months: 600,
                capacity_mw: 100,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }
        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::IndHeavy); // consumes 400 MW

        PowerSystem.tick(&mut map, &mut sim);

        let eff = sim.plants.get(&(0, 0)).unwrap().efficiency;
        assert!(
            (eff - 7.0 / 12.0).abs() < 0.001,
            "Efficiency should be 7/12 ≈ 0.583 with 7 months remaining (got {})",
            eff
        );
        // Consumer should still have raw BFS signal
        let level = map.get_overlay(5, 0).power_level;
        assert!(
            level > 200,
            "Raw signal should be high regardless of brownout (got {})",
            level
        );
        // But supply < demand in utilities
        assert!(
            sim.utilities.power_consumed_mw > sim.utilities.power_produced_mw,
            "Degraded plant should cause brownout: consumed={} > produced={}",
            sim.utilities.power_consumed_mw,
            sim.utilities.power_produced_mw
        );
    }

    #[test]
    fn plant_efficiency_normal_plant_preserves_default() {
        let mut map = Map::new(4, 4);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 100,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(dx, dy, Tile::PowerPlantCoal);
            }
        }
        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ResLow);

        PowerSystem.tick(&mut map, &mut sim);

        for dy in 0..4 {
            for dx in 0..4 {
                assert_eq!(
                    map.get_overlay(dx, dy).plant_efficiency,
                    255,
                    "Normal plant should set overlay to 255"
                );
            }
        }
    }

    // ── Helper: place a 4×4 coal plant at (px,py) ────────────────────────────
    fn place_coal_plant(map: &mut Map, sim: &mut SimState, px: usize, py: usize) {
        sim.plants.insert(
            (px, py),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
                footprint: 4,
            },
        );
        for dy in 0..4 {
            for dx in 0..4 {
                map.set(px + dx, py + dy, Tile::PowerPlantCoal);
            }
        }
    }

    // ── Road-square scenario tests ───────────────────────────────────────────

    /// Plant → PowerLine → Zone: zone should receive power.
    #[test]
    fn power_line_to_adjacent_zone() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "Zone adjacent to power line must receive power (got {})",
            map.get_overlay(5, 0).power_level
        );
    }

    /// Plant → PowerLine → Building: building should receive power.
    #[test]
    fn power_line_to_adjacent_building() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ResLow);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "Building adjacent to power line must receive power (got {})",
            map.get_overlay(5, 0).power_level
        );
    }

    /// Plant → RoadPowerLine → Zone: zone adjacent to road+powerline should get power.
    #[test]
    fn road_powerline_to_adjacent_zone() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::RoadPowerLine);
        map.set(5, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "Zone adjacent to RoadPowerLine must receive power (got {})",
            map.get_overlay(5, 0).power_level
        );
    }

    /// Plain road does NOT conduct power.
    #[test]
    fn plain_road_does_not_conduct() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::Road);
        map.set(5, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert_eq!(
            map.get_overlay(4, 0).power_level, 0,
            "Plain road must not receive power"
        );
        assert_eq!(
            map.get_overlay(5, 0).power_level, 0,
            "Zone behind a plain road must not receive power"
        );
    }

    /// Road square with zones inside: power must propagate through RoadPowerLine
    /// to reach interior zones.
    ///
    /// Layout (20×20 map):
    /// ```
    ///   Plant(0..4, 0..4)
    ///   PowerLine at (4,2)
    ///   RoadPowerLine at (5,2)  -- bridge
    ///   Road ring: (5,1)..(8,1) top, (5,4)..(8,4) bottom, (5,1)..(5,4) left, (8,1)..(8,4) right
    ///   Zones: (6,2), (7,2), (6,3), (7,3) inside the ring
    /// ```
    #[test]
    fn road_square_with_powerline_bridge_powers_interior_zones() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        // Power line connecting plant to the road bridge
        map.set(4, 2, Tile::PowerLine);

        // Road ring (no power lines on roads except the bridge)
        // Top row
        for x in 5..=8 {
            map.set(x, 1, Tile::Road);
        }
        // Bottom row
        for x in 5..=8 {
            map.set(x, 4, Tile::Road);
        }
        // Left column
        for y in 1..=4 {
            map.set(5, y, Tile::Road);
        }
        // Right column
        for y in 1..=4 {
            map.set(8, y, Tile::Road);
        }

        // Bridge: power line on the left-side road entry
        map.set(5, 2, Tile::RoadPowerLine);

        // Interior zones
        map.set(6, 2, Tile::ZoneRes);
        map.set(7, 2, Tile::ZoneRes);
        map.set(6, 3, Tile::ZoneRes);
        map.set(7, 3, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        // The bridge should be powered
        assert!(
            map.get_overlay(5, 2).power_level > 0,
            "RoadPowerLine bridge must be powered (got {})",
            map.get_overlay(5, 2).power_level
        );

        // Interior zones adjacent to the bridge
        assert!(
            map.get_overlay(6, 2).power_level > 0,
            "Zone at (6,2) adjacent to bridge must be powered (got {})",
            map.get_overlay(6, 2).power_level
        );

        // Interior zones further from the bridge (connected via zone chain)
        assert!(
            map.get_overlay(7, 2).power_level > 0,
            "Zone at (7,2) must be powered via zone chain (got {})",
            map.get_overlay(7, 2).power_level
        );
        assert!(
            map.get_overlay(6, 3).power_level > 0,
            "Zone at (6,3) must be powered via zone chain (got {})",
            map.get_overlay(6, 3).power_level
        );
        assert!(
            map.get_overlay(7, 3).power_level > 0,
            "Zone at (7,3) must be powered via zone chain (got {})",
            map.get_overlay(7, 3).power_level
        );
    }

    /// Same as above but with buildings instead of empty zones.
    #[test]
    fn road_square_with_powerline_bridge_powers_interior_buildings() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 2, Tile::PowerLine);

        // Road ring
        for x in 5..=8 {
            map.set(x, 1, Tile::Road);
        }
        for x in 5..=8 {
            map.set(x, 4, Tile::Road);
        }
        for y in 1..=4 {
            map.set(5, y, Tile::Road);
        }
        for y in 1..=4 {
            map.set(8, y, Tile::Road);
        }

        // Bridge
        map.set(5, 2, Tile::RoadPowerLine);

        // Interior buildings
        map.set(6, 2, Tile::ResLow);
        map.set(7, 2, Tile::CommLow);
        map.set(6, 3, Tile::IndLight);
        map.set(7, 3, Tile::ResLow);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(6, 2).power_level > 0,
            "ResLow at (6,2) must be powered (got {})",
            map.get_overlay(6, 2).power_level
        );
        assert!(
            map.get_overlay(7, 2).power_level > 0,
            "CommLow at (7,2) must be powered (got {})",
            map.get_overlay(7, 2).power_level
        );
        assert!(
            map.get_overlay(6, 3).power_level > 0,
            "IndLight at (6,3) must be powered (got {})",
            map.get_overlay(6, 3).power_level
        );
        assert!(
            map.get_overlay(7, 3).power_level > 0,
            "ResLow at (7,3) must be powered (got {})",
            map.get_overlay(7, 3).power_level
        );
    }

    /// Zones should chain-conduct power to each other.
    #[test]
    fn zone_chain_conducts_power() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        // Chain of 5 zones
        for x in 5..10 {
            map.set(x, 0, Tile::ZoneRes);
        }

        PowerSystem.tick(&mut map, &mut sim);

        for x in 5..10 {
            assert!(
                map.get_overlay(x, 0).power_level > 0,
                "Zone at ({},0) in chain must be powered (got {})",
                x,
                map.get_overlay(x, 0).power_level
            );
        }

        // Verify decay
        assert!(
            map.get_overlay(5, 0).power_level > map.get_overlay(9, 0).power_level,
            "Power should decay along the zone chain (near={}, far={})",
            map.get_overlay(5, 0).power_level,
            map.get_overlay(9, 0).power_level
        );
    }

    /// Building chain should conduct power.
    #[test]
    fn building_chain_conducts_power() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ResLow);
        map.set(6, 0, Tile::CommLow);
        map.set(7, 0, Tile::IndLight);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(7, 0).power_level > 0,
            "IndLight at end of building chain must be powered (got {})",
            map.get_overlay(7, 0).power_level
        );
    }

    /// Mixed chain: building → zone → building should all conduct.
    #[test]
    fn mixed_building_zone_chain_conducts() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ResLow);
        map.set(6, 0, Tile::ZoneRes);
        map.set(7, 0, Tile::CommLow);

        PowerSystem.tick(&mut map, &mut sim);

        for x in 5..=7 {
            assert!(
                map.get_overlay(x, 0).power_level > 0,
                "Tile at ({},0) in mixed chain must be powered (got {})",
                x,
                map.get_overlay(x, 0).power_level
            );
        }
    }

    /// Road in the middle of a conductive chain breaks propagation.
    #[test]
    fn road_gap_breaks_power_chain() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ZoneRes);
        map.set(6, 0, Tile::Road); // gap
        map.set(7, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "Zone before road gap must be powered"
        );
        assert_eq!(
            map.get_overlay(6, 0).power_level, 0,
            "Road gap must not receive power"
        );
        assert_eq!(
            map.get_overlay(7, 0).power_level, 0,
            "Zone after road gap must not receive power"
        );
    }

    /// Grass tile breaks the power chain.
    #[test]
    fn grass_gap_breaks_power_chain() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::ZoneRes);
        // (6,0) is Grass by default
        map.set(7, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "Zone before grass gap must be powered"
        );
        assert_eq!(
            map.get_overlay(7, 0).power_level, 0,
            "Zone after grass gap must not receive power"
        );
    }

    /// Power line placed on top of a zone (layered model) should conduct
    /// AND the zone should still be recognized as powered.
    #[test]
    fn powerline_over_zone_conducts_and_powers_zone() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);

        // Place zone then power line on top (layered)
        map.set_zone_spec(
            5,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_power_line(5, 0, true);

        // Zone after the powerline-over-zone
        map.set(6, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "PowerLine-over-zone must be powered (got {})",
            map.get_overlay(5, 0).power_level
        );
        assert!(
            map.get_overlay(6, 0).power_level > 0,
            "Zone after powerline-over-zone must be powered (got {})",
            map.get_overlay(6, 0).power_level
        );
    }

    /// Realistic city layout: plant → powerlines → road square → zones inside.
    /// Only one road edge has power lines; zones should still get power
    /// through the RoadPowerLine entry point.
    #[test]
    fn realistic_road_square_single_entry_point() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        // Power line corridor from plant to the road area
        for x in 4..=9 {
            map.set(x, 2, Tile::PowerLine);
        }

        // Road square: 10..=14 x, 1..=5 y
        for x in 10..=14 {
            map.set(x, 1, Tile::Road); // top
            map.set(x, 5, Tile::Road); // bottom
        }
        for y in 1..=5 {
            map.set(10, y, Tile::Road); // left
            map.set(14, y, Tile::Road); // right
        }

        // Single entry: power line on the left-side road at y=2
        map.set(10, 2, Tile::RoadPowerLine);

        // Zones inside the square
        for x in 11..=13 {
            for y in 2..=4 {
                map.set(x, y, Tile::ZoneRes);
            }
        }

        PowerSystem.tick(&mut map, &mut sim);

        // The bridge must be powered
        assert!(
            map.get_overlay(10, 2).power_level > 0,
            "RoadPowerLine entry must be powered (got {})",
            map.get_overlay(10, 2).power_level
        );

        // All interior zones must be powered
        for x in 11..=13 {
            for y in 2..=4 {
                assert!(
                    map.get_overlay(x, y).power_level > 0,
                    "Zone at ({},{}) must be powered (got {})",
                    x,
                    y,
                    map.get_overlay(x, y).power_level
                );
            }
        }
    }

    /// Realistic layout with developed buildings inside the road square.
    #[test]
    fn realistic_road_square_with_buildings_inside() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        // Power line corridor
        for x in 4..=9 {
            map.set(x, 2, Tile::PowerLine);
        }

        // Road square
        for x in 10..=14 {
            map.set(x, 1, Tile::Road);
            map.set(x, 5, Tile::Road);
        }
        for y in 1..=5 {
            map.set(10, y, Tile::Road);
            map.set(14, y, Tile::Road);
        }

        // Single entry
        map.set(10, 2, Tile::RoadPowerLine);

        // Mix of buildings and zones inside
        map.set(11, 2, Tile::ResLow);
        map.set(12, 2, Tile::ZoneRes);
        map.set(13, 2, Tile::CommLow);
        map.set(11, 3, Tile::ZoneComm);
        map.set(12, 3, Tile::IndLight);
        map.set(13, 3, Tile::ZoneInd);
        map.set(11, 4, Tile::ResLow);
        map.set(12, 4, Tile::ResLow);
        map.set(13, 4, Tile::ResLow);

        PowerSystem.tick(&mut map, &mut sim);

        let tiles = [
            (11, 2),
            (12, 2),
            (13, 2),
            (11, 3),
            (12, 3),
            (13, 3),
            (11, 4),
            (12, 4),
            (13, 4),
        ];
        for (x, y) in tiles {
            assert!(
                map.get_overlay(x, y).power_level > 0,
                "Tile at ({},{}) [{}] must be powered (got {})",
                x,
                y,
                map.surface_lot_tile(x, y).name(),
                map.get_overlay(x, y).power_level
            );
        }
    }

    /// Zones adjacent to a plain road (no power line) should NOT receive power
    /// through the road.
    #[test]
    fn zones_adjacent_to_plain_road_not_powered_through_road() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::Road); // no power line
        map.set(6, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert_eq!(
            map.get_overlay(6, 0).power_level, 0,
            "Zone behind plain road must NOT be powered"
        );
    }

    /// Power should reach zone diagonally-adjacent to a power line via an
    /// intermediate zone (no diagonal conduction, only 4-connected).
    #[test]
    fn no_diagonal_conduction() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        // Zone at (5, 1) — diagonally adjacent to power line at (4,0)
        map.set(5, 1, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert_eq!(
            map.get_overlay(5, 1).power_level, 0,
            "Zone diagonally adjacent to power line must NOT be powered via diagonal"
        );
    }

    /// Service buildings (police, fire, hospital) should receive and conduct power.
    #[test]
    fn service_buildings_conduct_power() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::Police);
        map.set(6, 0, Tile::Fire);
        map.set(7, 0, Tile::Hospital);
        map.set(8, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "Police should be powered"
        );
        assert!(
            map.get_overlay(6, 0).power_level > 0,
            "Fire dept should be powered"
        );
        assert!(
            map.get_overlay(7, 0).power_level > 0,
            "Hospital should be powered"
        );
        assert!(
            map.get_overlay(8, 0).power_level > 0,
            "Zone after service buildings should be powered"
        );
    }

    /// Multiple road squares with separate RoadPowerLine bridges should
    /// independently receive power.
    #[test]
    fn multiple_road_squares_powered_independently() {
        let mut map = Map::new(30, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        // Power line backbone
        for x in 4..20 {
            map.set(x, 0, Tile::PowerLine);
        }

        // First block: roads at x=5..8, y=1..4
        for x in 5..=8 {
            map.set(x, 1, Tile::Road);
            map.set(x, 4, Tile::Road);
        }
        for y in 1..=4 {
            map.set(5, y, Tile::Road);
            map.set(8, y, Tile::Road);
        }
        map.set(6, 1, Tile::RoadPowerLine); // bridge from backbone
        map.set(6, 2, Tile::ZoneRes);
        map.set(7, 2, Tile::ZoneRes);
        map.set(6, 3, Tile::ZoneRes);
        map.set(7, 3, Tile::ZoneRes);

        // Second block: roads at x=12..15, y=1..4
        for x in 12..=15 {
            map.set(x, 1, Tile::Road);
            map.set(x, 4, Tile::Road);
        }
        for y in 1..=4 {
            map.set(12, y, Tile::Road);
            map.set(15, y, Tile::Road);
        }
        map.set(13, 1, Tile::RoadPowerLine); // bridge from backbone
        map.set(13, 2, Tile::ResLow);
        map.set(14, 2, Tile::ResLow);
        map.set(13, 3, Tile::CommLow);
        map.set(14, 3, Tile::IndLight);

        PowerSystem.tick(&mut map, &mut sim);

        // First block zones
        for (x, y) in [(6, 2), (7, 2), (6, 3), (7, 3)] {
            assert!(
                map.get_overlay(x, y).power_level > 0,
                "Block 1 zone at ({},{}) must be powered (got {})",
                x,
                y,
                map.get_overlay(x, y).power_level
            );
        }

        // Second block buildings
        for (x, y) in [(13, 2), (14, 2), (13, 3), (14, 3)] {
            assert!(
                map.get_overlay(x, y).power_level > 0,
                "Block 2 building at ({},{}) must be powered (got {})",
                x,
                y,
                map.get_overlay(x, y).power_level
            );
        }
    }

    /// Zone placed using layered API (set_zone_spec) should conduct power
    /// the same way as zone placed via set().
    #[test]
    fn layered_zone_placement_conducts_power() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);

        // Place zone using the layered API
        map.set_zone_spec(
            5,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_zone_spec(
            6,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Commercial,
                density: ZoneDensity::Dense,
            }),
        );

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "Layered ZoneRes must be powered (got {})",
            map.get_overlay(5, 0).power_level
        );
        assert!(
            map.get_overlay(6, 0).power_level > 0,
            "Layered ZoneComm must be powered (got {})",
            map.get_overlay(6, 0).power_level
        );
    }

    /// Power level at the zone entry point should be meaningfully high,
    /// not just barely above zero.
    #[test]
    fn power_level_at_zone_entry_is_substantial() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::RoadPowerLine);
        map.set(6, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        let bridge_level = map.get_overlay(5, 0).power_level;
        let zone_level = map.get_overlay(6, 0).power_level;

        assert!(
            bridge_level > 200,
            "Bridge power level should be high (got {})",
            bridge_level
        );
        assert!(
            zone_level > 200,
            "Zone adjacent to bridge should have substantial power (got {}), not nearly zero",
            zone_level
        );
        assert!(
            zone_level >= bridge_level.saturating_sub(POWER_FALLOFF_ZONE),
            "Zone level should be bridge level minus zone falloff"
        );
    }

    /// Zones inside a road square where ALL roads have power lines should
    /// be well-powered from multiple directions.
    #[test]
    fn road_square_all_edges_powered() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        // Power line corridor
        for x in 4..10 {
            map.set(x, 2, Tile::PowerLine);
        }

        // Road square with ALL edges as RoadPowerLine
        for x in 10..=14 {
            map.set(x, 1, Tile::RoadPowerLine);
            map.set(x, 5, Tile::RoadPowerLine);
        }
        for y in 1..=5 {
            map.set(10, y, Tile::RoadPowerLine);
            map.set(14, y, Tile::RoadPowerLine);
        }

        // Zones inside
        for x in 11..=13 {
            for y in 2..=4 {
                map.set(x, y, Tile::ZoneRes);
            }
        }

        PowerSystem.tick(&mut map, &mut sim);

        for x in 11..=13 {
            for y in 2..=4 {
                let level = map.get_overlay(x, y).power_level;
                assert!(
                    level > 100,
                    "Zone at ({},{}) should be well powered from all sides (got {})",
                    x,
                    y,
                    level
                );
            }
        }
    }

    /// Water utility buildings (pump, tower, treatment, desalination) should
    /// conduct and receive power.
    #[test]
    fn water_facilities_conduct_power() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::WaterPump);
        map.set(6, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert!(
            map.get_overlay(5, 0).power_level > 0,
            "WaterPump must be powered"
        );
        assert!(
            map.get_overlay(6, 0).power_level > 0,
            "Zone after WaterPump must be powered (facility should conduct)"
        );
    }

    /// Highway does NOT conduct power (same as plain road).
    #[test]
    fn highway_does_not_conduct_power() {
        let mut map = Map::new(20, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::Highway);
        map.set(6, 0, Tile::ZoneRes);

        PowerSystem.tick(&mut map, &mut sim);

        assert_eq!(
            map.get_overlay(5, 0).power_level, 0,
            "Highway must not receive power"
        );
        assert_eq!(
            map.get_overlay(6, 0).power_level, 0,
            "Zone behind highway must not be powered"
        );
    }

    /// BUG REPRO: power line shows ~80% but adjacent zone shows ~6%.
    /// This happens because brownout scaling only applies to consumers
    /// (consumption > 0) but NOT to power lines (consumption = 0).
    /// The visual result is that power lines look fine but connected
    /// zones/buildings appear nearly unpowered.
    #[test]
    fn brownout_scaling_consistent_between_powerlines_and_zones() {
        let mut map = Map::new(30, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        // Power line corridor from plant
        for x in 4..15 {
            map.set(x, 0, Tile::PowerLine);
        }

        // Many zones connected — enough to cause brownout
        // Each ZoneRes consumes 2 MW, each ResLow consumes 10 MW
        // Plant capacity = 500 MW
        // Place enough buildings to exceed capacity
        for x in 15..30 {
            for y in 0..10 {
                if (x + y) % 2 == 0 {
                    map.set(x, y, Tile::ResLow); // 10 MW each
                } else {
                    map.set(x, y, Tile::ZoneRes); // 2 MW each
                }
            }
        }
        // Connect them to the grid
        map.set(15, 0, Tile::PowerLine);

        PowerSystem.tick(&mut map, &mut sim);

        let powerline_level = map.get_overlay(14, 0).power_level;
        let first_zone_level = map.get_overlay(16, 0).power_level;

        // The key assertion: if a zone is only 2 tiles from a power line,
        // their displayed power levels should not differ by much.
        // A huge gap (e.g., 80% vs 6%) indicates the brownout factor is
        // only applied to consumers and not to power lines.
        let gap = powerline_level.saturating_sub(first_zone_level);
        assert!(
            gap <= 20,
            "Power line ({}) and adjacent zone ({}) levels should be close, \
             but gap is {}. This indicates brownout scaling is not applied \
             consistently — power lines keep raw BFS level while consumers \
             get scaled down.",
            powerline_level,
            first_zone_level,
            gap
        );
    }

    /// Overlay shows raw BFS signal even during brownout. Plant footprint and
    /// adjacent powerline should both show high raw signal — no cliff.
    /// Brownout is reported separately in sim.utilities.
    #[test]
    fn overlay_shows_raw_signal_during_brownout() {
        let mut map = Map::new(30, 20);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        // Lots of heavy industry to force severe brownout
        map.set(4, 0, Tile::PowerLine);
        map.set(5, 0, Tile::IndHeavy);
        map.set(5, 1, Tile::IndHeavy);
        map.set(5, 2, Tile::IndHeavy);
        map.set(5, 3, Tile::IndHeavy);

        PowerSystem.tick(&mut map, &mut sim);

        // Verify brownout IS happening (in utilities, not overlay)
        assert!(
            sim.utilities.power_consumed_mw > sim.utilities.power_produced_mw,
            "Test setup should cause brownout (consumed={} > produced={})",
            sim.utilities.power_consumed_mw,
            sim.utilities.power_produced_mw
        );

        // Raw BFS signal: plant at 255, adjacent powerline close to 255
        let plant_level = map.get_overlay(0, 0).power_level;
        let powerline_level = map.get_overlay(4, 0).power_level;
        assert_eq!(plant_level, 255, "Plant tiles should keep raw BFS level 255");
        assert!(
            powerline_level > 250,
            "Powerline adjacent to plant should have high raw signal (got {})",
            powerline_level
        );
    }

    /// Power propagation must stop when level decays to 0.
    #[test]
    fn power_stops_after_max_distance() {
        // With POWER_FALLOFF_LINE=1, power lines can reach ~255 tiles from
        // the plant edge. Use a corridor long enough to exceed that.
        let corridor_len = 300;
        let mut map = Map::new(corridor_len, 5);
        let mut sim = SimState::default();
        place_coal_plant(&mut map, &mut sim, 0, 0);

        for x in 4..corridor_len {
            map.set(x, 0, Tile::PowerLine);
        }

        PowerSystem.tick(&mut map, &mut sim);

        // Find the last tile with power
        let mut last_powered = 0;
        for x in 0..corridor_len {
            if map.get_overlay(x, 0).power_level > 0 {
                last_powered = x;
            }
        }

        // Power should stop before the end of the corridor
        assert_eq!(
            map.get_overlay(corridor_len - 1, 0).power_level, 0,
            "Power should not reach the far end of a long corridor",
        );
        // Power should reach far beyond the plant
        assert!(
            last_powered > 100,
            "Power lines should conduct power over long distances (last powered: {})",
            last_powered
        );
    }

    // ── ResourceRole classification tests ─────────────────────────────────────

    #[test]
    fn power_role_producers() {
        use crate::core::map::ResourceRole;
        for tile in [
            Tile::PowerPlantCoal,
            Tile::PowerPlantGas,
            Tile::PowerPlantNuclear,
            Tile::PowerPlantWind,
            Tile::PowerPlantSolar,
        ] {
            assert_eq!(tile.power_role(), ResourceRole::Producer, "{:?}", tile);
        }
    }

    #[test]
    fn power_role_conductors() {
        use crate::core::map::ResourceRole;
        // Power lines should be the best conductors
        assert!(
            matches!(Tile::PowerLine.power_role(), ResourceRole::Conductor { falloff } if falloff == POWER_FALLOFF_LINE)
        );
        assert!(
            matches!(Tile::RoadPowerLine.power_role(), ResourceRole::Conductor { falloff } if falloff == POWER_FALLOFF_LINE)
        );
        // Buildings should conduct with higher falloff
        for tile in [Tile::ResLow, Tile::CommHigh, Tile::IndHeavy, Tile::Hospital, Tile::School] {
            assert!(
                matches!(tile.power_role(), ResourceRole::Conductor { falloff } if falloff == POWER_FALLOFF_BUILDING),
                "{:?}",
                tile
            );
        }
        // Zones should have the highest falloff
        for tile in [Tile::ZoneRes, Tile::ZoneComm, Tile::ZoneInd] {
            assert!(
                matches!(tile.power_role(), ResourceRole::Conductor { falloff } if falloff == POWER_FALLOFF_ZONE),
                "{:?}",
                tile
            );
        }
    }

    #[test]
    fn power_role_none_for_roads_and_terrain() {
        use crate::core::map::ResourceRole;
        for tile in [Tile::Road, Tile::Highway, Tile::Rail, Tile::Grass, Tile::Water, Tile::Trees] {
            assert_eq!(tile.power_role(), ResourceRole::None, "{:?}", tile);
        }
    }

    #[test]
    fn power_demand_values() {
        assert_eq!(Tile::ResLow.power_demand(), 10);
        assert_eq!(Tile::ResHigh.power_demand(), 150);
        assert_eq!(Tile::IndHeavy.power_demand(), 400);
        assert_eq!(Tile::Hospital.power_demand(), 200);
        assert_eq!(Tile::Stadium.power_demand(), 300);
        assert_eq!(Tile::ZoneRes.power_demand(), 2);
        assert_eq!(Tile::Road.power_demand(), 0);
        assert_eq!(Tile::PowerLine.power_demand(), 0);
        assert_eq!(Tile::PowerPlantCoal.power_demand(), 0);
    }

    #[test]
    fn receives_power_matches_role() {
        use crate::core::map::ResourceRole;
        for tile in [
            Tile::PowerPlantCoal, Tile::PowerLine, Tile::RoadPowerLine,
            Tile::ResLow, Tile::CommHigh, Tile::Hospital, Tile::ZoneRes,
        ] {
            assert!(tile.receives_power(), "{:?} should receive power", tile);
            assert_ne!(tile.power_role(), ResourceRole::None, "{:?}", tile);
        }
        for tile in [Tile::Road, Tile::Highway, Tile::Grass, Tile::Water] {
            assert!(!tile.receives_power(), "{:?} should NOT receive power", tile);
        }
    }

    // ── Per-conductor falloff tests ───────────────────────────────────────────

    #[test]
    fn powerline_chain_decays_slower_than_zone_chain() {
        let mut map_line = Map::new(30, 1);
        let mut sim_line = SimState::default();
        sim_line.plants.insert(
            (0, 0),
            PlantState { age_months: 0, max_life_months: 600, capacity_mw: 5000, efficiency: 1.0, footprint: 1 },
        );
        map_line.set(0, 0, Tile::PowerPlantCoal);
        for x in 1..30 {
            map_line.set(x, 0, Tile::PowerLine);
        }
        PowerSystem.tick(&mut map_line, &mut sim_line);

        let mut map_zone = Map::new(30, 1);
        let mut sim_zone = SimState::default();
        sim_zone.plants.insert(
            (0, 0),
            PlantState { age_months: 0, max_life_months: 600, capacity_mw: 5000, efficiency: 1.0, footprint: 1 },
        );
        map_zone.set(0, 0, Tile::PowerPlantCoal);
        for x in 1..30 {
            map_zone.set(x, 0, Tile::ZoneRes);
        }
        PowerSystem.tick(&mut map_zone, &mut sim_zone);

        let line_level = map_line.get_overlay(20, 0).power_level;
        let zone_level = map_zone.get_overlay(20, 0).power_level;
        assert!(
            line_level > zone_level + 50,
            "PowerLine chain ({}) should retain much more power than zone chain ({}) at distance 20",
            line_level, zone_level
        );
    }

    #[test]
    fn building_chain_decays_between_line_and_zone() {
        let len = 25;
        let mut map = Map::new(len, 1);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState { age_months: 0, max_life_months: 600, capacity_mw: 5000, efficiency: 1.0, footprint: 1 },
        );
        map.set(0, 0, Tile::PowerPlantCoal);
        for x in 1..len {
            map.set(x, 0, Tile::ResLow);
        }
        PowerSystem.tick(&mut map, &mut sim);

        let bldg_level_at_10 = map.get_overlay(10, 0).power_level;
        assert!(
            bldg_level_at_10 > 200 && bldg_level_at_10 < 240,
            "Building chain at distance 10 should be around 225 (got {})",
            bldg_level_at_10
        );
    }

    #[test]
    fn zones_conduct_power_with_high_falloff() {
        let mut map = Map::new(10, 1);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState { age_months: 0, max_life_months: 600, capacity_mw: 5000, efficiency: 1.0, footprint: 1 },
        );
        map.set(0, 0, Tile::PowerPlantCoal);
        for x in 1..10 {
            map.set(x, 0, Tile::ZoneRes);
        }
        PowerSystem.tick(&mut map, &mut sim);

        let z1 = map.get_overlay(1, 0).power_level;
        let z5 = map.get_overlay(5, 0).power_level;
        assert!(z1 > 240, "First zone should have high power (got {})", z1);
        assert!(z5 > 0, "Zone at distance 5 should still have some power (got {})", z5);
        assert!(z5 < z1, "Power should decrease along zone chain");
    }

    // ── Producer immunity tests ───────────────────────────────────────────────

    #[test]
    fn producer_keeps_raw_signal_during_massive_overload() {
        let mut map = Map::new(50, 1);
        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState { age_months: 0, max_life_months: 600, capacity_mw: 10, efficiency: 1.0, footprint: 1 },
        );
        map.set(0, 0, Tile::PowerPlantCoal);
        for x in 1..50 {
            map.set(x, 0, Tile::IndHeavy);
        }
        PowerSystem.tick(&mut map, &mut sim);

        // Overlay keeps raw BFS signal; brownout is in utilities
        let producer_level = map.get_overlay(0, 0).power_level;
        assert_eq!(producer_level, 255, "Producer keeps raw signal level 255");
        let consumer_level = map.get_overlay(1, 0).power_level;
        assert!(consumer_level > 250, "Adjacent consumer raw signal should be high (got {})", consumer_level);
        // Massive brownout reported in utilities
        assert!(
            sim.utilities.power_consumed_mw > sim.utilities.power_produced_mw * 10,
            "Should report massive overload: consumed={} vs produced={}",
            sim.utilities.power_consumed_mw, sim.utilities.power_produced_mw
        );
    }
}
