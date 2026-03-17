use rand::Rng;
use crate::core::map::{Map, Tile};
use super::SimState;

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
            match tile {
                Tile::ZoneRes => {
                    if has_road_access(map, x, y)
                        && rng.gen::<f32>() < sim.demand_res * 0.15
                    {
                        changes.push((x, y, Tile::ResLow));
                    }
                }
                Tile::ZoneComm => {
                    if has_road_access(map, x, y)
                        && rng.gen::<f32>() < sim.demand_comm * 0.08
                    {
                        changes.push((x, y, Tile::CommLow));
                    }
                }
                Tile::ZoneInd => {
                    if has_road_access(map, x, y)
                        && rng.gen::<f32>() < sim.demand_ind * 0.08
                    {
                        changes.push((x, y, Tile::IndLight));
                    }
                }
                Tile::ResLow => {
                    new_pop += 10;
                    if has_road_access(map, x, y)
                        && rng.gen::<f32>() < sim.demand_res * 0.03
                    {
                        changes.push((x, y, Tile::ResMed));
                    }
                }
                Tile::ResMed => {
                    new_pop += 50;
                    if has_road_access(map, x, y)
                        && rng.gen::<f32>() < sim.demand_res * 0.015
                    {
                        changes.push((x, y, Tile::ResHigh));
                    }
                }
                Tile::ResHigh => {
                    new_pop += 200;
                }
                Tile::CommLow => {
                    new_pop += 5;
                    if has_road_access(map, x, y)
                        && rng.gen::<f32>() < sim.demand_comm * 0.02
                    {
                        changes.push((x, y, Tile::CommHigh));
                    }
                }
                Tile::CommHigh => {
                    new_pop += 20;
                }
                Tile::IndLight => {
                    if has_road_access(map, x, y)
                        && rng.gen::<f32>() < sim.demand_ind * 0.02
                    {
                        changes.push((x, y, Tile::IndHeavy));
                    }
                }
                Tile::IndHeavy => {}
                _ => {}
            }
        }
    }

    for (x, y, tile) in changes {
        map.set(x, y, tile);
    }

    sim.population = new_pop;
}

fn has_road_access(map: &Map, x: usize, y: usize) -> bool {
    map.neighbors4(x, y).iter().any(|(_, _, t)| t.is_road())
}
