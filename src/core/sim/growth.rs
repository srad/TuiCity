use super::SimState;
use crate::core::map::{Map, Tile};
use rand::Rng;

pub fn tick_growth(map: &mut Map, sim: &mut SimState) {
    let mut rng = rand::thread_rng();
    let w = map.width;
    let h = map.height;
    let mut new_pop: u64 = 0;

    let mut changes: Vec<(usize, usize, Tile)> = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let tile    = map.get(x, y);
            let overlay = map.get_overlay(x, y);
            let powered      = overlay.powered;
            let road_access  = has_road_access(map, x, y, 3);

            // Modifier: pollution hurts residential growth (0.3..1.0)
            let pollution_penalty = 1.0 - (overlay.pollution as f32 / 255.0) * 0.7;
            // Modifier: high land value gives a small growth bonus
            let lv_bonus = overlay.land_value as f32 / 255.0 * 0.1;
            // Modifier: crime reduces residential and commercial upgrades (0.3..1.0)
            let crime_penalty = 1.0 - (overlay.crime as f32 / 255.0) * 0.7;

            match tile {
                Tile::ZoneRes => {
                    let chance = (sim.demand_res * 0.15 + lv_bonus) * pollution_penalty * crime_penalty;
                    if road_access && powered && rng.gen::<f32>() < chance {
                        changes.push((x, y, Tile::ResLow));
                    }
                }
                Tile::ZoneComm => {
                    let chance = (sim.demand_comm * 0.08 + lv_bonus * 0.5) * crime_penalty;
                    if road_access && powered && rng.gen::<f32>() < chance {
                        changes.push((x, y, Tile::CommLow));
                    }
                }
                Tile::ZoneInd => {
                    // Industry is indifferent to crime and benefits less from land value
                    if road_access && powered && rng.gen::<f32>() < sim.demand_ind * 0.08 {
                        changes.push((x, y, Tile::IndLight));
                    }
                }
                Tile::ResLow => {
                    new_pop += 10;
                    let chance = (sim.demand_res * 0.03 + lv_bonus) * pollution_penalty * crime_penalty;
                    if road_access && powered && rng.gen::<f32>() < chance {
                        changes.push((x, y, Tile::ResMed));
                    } else if !powered && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneRes));
                    }
                }
                Tile::ResMed => {
                    new_pop += 50;
                    let chance = (sim.demand_res * 0.015 + lv_bonus * 0.5) * pollution_penalty * crime_penalty;
                    if road_access && powered && rng.gen::<f32>() < chance {
                        changes.push((x, y, Tile::ResHigh));
                    } else if !powered && rng.gen::<f32>() < 0.05 {
                        changes.push((x, y, Tile::ResLow));
                    }
                }
                Tile::ResHigh => {
                    new_pop += 200;
                    if !powered && rng.gen::<f32>() < 0.1 {
                        changes.push((x, y, Tile::ResMed));
                    }
                }
                Tile::CommLow => {
                    new_pop += 5;
                    let chance = (sim.demand_comm * 0.02 + lv_bonus * 0.5) * crime_penalty;
                    if road_access && powered && rng.gen::<f32>() < chance {
                        changes.push((x, y, Tile::CommHigh));
                    } else if !powered && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneComm));
                    }
                }
                Tile::CommHigh => {
                    new_pop += 20;
                    if !powered && rng.gen::<f32>() < 0.05 {
                        changes.push((x, y, Tile::CommLow));
                    }
                }
                Tile::IndLight => {
                    if road_access && powered && rng.gen::<f32>() < sim.demand_ind * 0.02 {
                        changes.push((x, y, Tile::IndHeavy));
                    } else if !powered && rng.gen::<f32>() < 0.01 {
                        changes.push((x, y, Tile::ZoneInd));
                    }
                }
                Tile::IndHeavy => {
                    if !powered && rng.gen::<f32>() < 0.05 {
                        changes.push((x, y, Tile::IndLight));
                    }
                }
                _ => {}
            }
        }
    }

    for (x, y, tile) in changes {
        map.set(x, y, tile);
    }

    sim.population = new_pop;
}

fn has_road_access(map: &Map, start_x: usize, start_y: usize, max_dist: i32) -> bool {
    let ix = start_x as i32;
    let iy = start_y as i32;

    for dy in -max_dist..=max_dist {
        for dx in -max_dist..=max_dist {
            if dx.abs() + dy.abs() <= max_dist {
                let nx = ix + dx;
                let ny = iy + dy;
                if map.in_bounds(nx, ny) && map.get(nx as usize, ny as usize).is_road() {
                    return true;
                }
            }
        }
    }
    false
}
