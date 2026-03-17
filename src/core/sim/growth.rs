use super::SimState;
use crate::core::map::{Map, Tile};
use rand::Rng;

pub fn tick_growth(map: &mut Map, sim: &mut SimState) {
    let mut rng = rand::thread_rng();
    let w = map.width;
    let h = map.height;
    let mut new_pop: u64 = 0;

    // Collect changes to apply after iteration (avoid borrow issues)
    let mut changes: Vec<(usize, usize, Tile)> = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let tile = map.get(x, y);
            let powered = map.get_overlay(x, y).powered;
            let road_access = has_road_access(map, x, y, 3);
            
            // Zones can only develop into low density if they have roads and power, OR if demand is very high and they have roads (like early game)
            // But they upgrade to Med/High ONLY if powered.
            
            match tile {
                Tile::ZoneRes => {
                    if road_access && powered && rng.gen::<f32>() < sim.demand_res * 0.15 {
                        changes.push((x, y, Tile::ResLow));
                    }
                }
                Tile::ZoneComm => {
                    if road_access && powered && rng.gen::<f32>() < sim.demand_comm * 0.08 {
                        changes.push((x, y, Tile::CommLow));
                    }
                }
                Tile::ZoneInd => {
                    if road_access && powered && rng.gen::<f32>() < sim.demand_ind * 0.08 {
                        changes.push((x, y, Tile::IndLight));
                    }
                }
                Tile::ResLow => {
                    new_pop += 10;
                    if road_access && powered && rng.gen::<f32>() < sim.demand_res * 0.03 {
                        changes.push((x, y, Tile::ResMed));
                    } else if !powered && rng.gen::<f32>() < 0.01 {
                         // Small chance to decay if no power
                         changes.push((x, y, Tile::ZoneRes));
                    }
                }
                Tile::ResMed => {
                    new_pop += 50;
                    if road_access && powered && rng.gen::<f32>() < sim.demand_res * 0.015 {
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
                    if road_access && powered && rng.gen::<f32>() < sim.demand_comm * 0.02 {
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
            // Manhattan distance
            if dx.abs() + dy.abs() <= max_dist {
                let nx = ix + dx;
                let ny = iy + dy;
                if map.in_bounds(nx, ny)
                    && map.get(nx as usize, ny as usize).is_road() {
                        return true;
                    }
            }
        }
    }
    false
}
