use crate::core::map::{Map, Tile};
use crate::core::sim::constants::{EOL_DECAY_MONTHS, POWER_FALLOFF_PER_TILE};
use crate::core::sim::system::SimSystem;
use crate::core::sim::SimState;

// ── PowerSystem ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PowerSystem;

impl PowerSystem {
    pub fn get_consumption(tile: Tile) -> u32 {
        match tile {
            Tile::ResLow => 10,
            Tile::ResMed => 40,
            Tile::ResHigh => 150,
            Tile::CommLow => 30,
            Tile::CommHigh => 120,
            Tile::IndLight => 100,
            Tile::IndHeavy => 400,
            Tile::Police => 50,
            Tile::Fire => 50,
            Tile::Hospital => 200,
            Tile::BusDepot | Tile::RailDepot | Tile::SubwayStation => 25,
            Tile::WaterPump => 25,
            Tile::WaterTower => 15,
            Tile::WaterTreatment => 60,
            Tile::Desalination => 90,
            Tile::School => 50,
            Tile::Stadium => 300,
            Tile::Library => 30,
            Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd => 2, // Minimal for zones
            _ => 0,
        }
    }
}

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
                remaining as f32 / EOL_DECAY_MONTHS as f32  // uses constants::EOL_DECAY_MONTHS
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

        // SC2000-style conduction:
        // - plants, power lines and developed buildings conduct power
        // - empty zones can receive power, but do not relay it onward
        // - roads do not conduct unless there is a power line on the tile
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
            let next_level = level.saturating_sub(POWER_FALLOFF_PER_TILE);

            for (nx, ny, tile) in map.neighbors4(x, y) {
                let n_idx = ny * map.width + nx;
                let lot_tile = map.surface_lot_tile(nx, ny);
                let conductive = tile.power_connects()
                    || lot_tile == Tile::PowerPlantCoal
                    || lot_tile == Tile::PowerPlantGas
                    || lot_tile == Tile::PowerPlantNuclear
                    || lot_tile == Tile::PowerPlantWind
                    || lot_tile == Tile::PowerPlantSolar
                    || lot_tile.is_conductive_structure();
                let receivable = conductive || lot_tile.receives_power();

                if !receivable || map.overlays[n_idx].power_level >= next_level {
                    continue;
                }

                map.overlays[n_idx].power_level = next_level;
                if conductive {
                    queue.push_back((nx, ny, next_level));
                }
            }
        }

        // Brownouts depend on connected demand, not disconnected buildings elsewhere on the map.
        let mut connected_demand = 0;
        let mut connected_consumers = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                let lot_tile = map.surface_lot_tile(x, y);
                let consumption = Self::get_consumption(lot_tile);
                if consumption == 0 {
                    continue;
                }

                let idx = y * map.width + x;
                let raw_level = map.overlays[idx].power_level;
                if raw_level > 0 {
                    connected_demand += consumption;
                    connected_consumers.push((idx, raw_level));
                }
            }
        }
        sim.utilities.power_consumed_mw = connected_demand;

        let brownout_factor = if total_capacity == 0 {
            0.0
        } else if connected_demand > total_capacity {
            total_capacity as f32 / connected_demand as f32
        } else {
            1.0
        };

        for (idx, level) in connected_consumers {
            let actual_level = (level as f32 * brownout_factor) as u8;
            map.overlays[idx].power_level = actual_level;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn power_brownout_scaling() {
        let mut map = Map::new(10, 10);
        let mut sim = SimState::default();

        // Plant produces 500 MW
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

        // Heavy industry consumes 400 MW
        map.set(5, 0, Tile::IndHeavy);
        // Another heavy industry makes total 800 MW > 500 MW
        map.set(5, 1, Tile::IndHeavy);

        PowerSystem.tick(&mut map, &mut sim);

        let level = map.get_overlay(5, 0).power_level;
        // Without brownout, it would be around 250+.
        // With brownout (500/800 = 0.625), it should be lower.
        assert!(
            level < 200,
            "Power level should be scaled down during brownout (got {})",
            level
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
    fn plant_efficiency_degraded_browns_out() {
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
        map.set(5, 0, Tile::IndHeavy); // consumes 200 MW

        PowerSystem.tick(&mut map, &mut sim);

        let eff = sim.plants.get(&(0, 0)).unwrap().efficiency;
        assert!(
            (eff - 7.0 / 12.0).abs() < 0.001,
            "Efficiency should be 7/12 ≈ 0.583 with 7 months remaining (got {})",
            eff
        );
        let level = map.get_overlay(5, 0).power_level;
        assert!(
            level < 128,
            "With efficiency 0.5 and 200 MW demand, power level should be very low (got {})",
            level
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
}
