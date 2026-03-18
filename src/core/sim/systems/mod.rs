use crate::core::map::{Map, Tile};
use crate::core::sim::{SimState, MaintenanceBreakdown, growth};
use crate::core::sim::system::SimSystem;
use rand::Rng;

// ── PowerSystem ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PowerSystem;
impl SimSystem for PowerSystem {
    fn name(&self) -> &str { "Power" }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        map.update_power_grid();
    }
}

// ── PollutionSystem ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PollutionSystem;
impl SimSystem for PollutionSystem {
    fn name(&self) -> &str { "Pollution" }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        // Reset
        for o in map.overlays.iter_mut() {
            o.pollution = 0;
        }

        // Collect industrial sources
        let mut sources: Vec<(usize, usize, u8)> = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                let strength: u8 = match map.get(x, y) {
                    Tile::IndHeavy => 200,
                    Tile::IndLight => 120,
                    Tile::PowerPlant => 80,
                    _ => 0,
                };
                if strength > 0 {
                    sources.push((x, y, strength));
                }
            }
        }

        // Radial diffusion with distance falloff (radius 10)
        const RADIUS: i32 = 10;
        const RADIUS_SQ: f32 = (RADIUS * RADIUS) as f32;
        for (sx, sy, strength) in sources {
            for dy in -RADIUS..=RADIUS {
                for dx in -RADIUS..=RADIUS {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    if dist_sq > RADIUS_SQ { continue; }
                    let nx = sx as i32 + dx;
                    let ny = sy as i32 + dy;
                    if !map.in_bounds(nx, ny) { continue; }
                    let falloff = 1.0 - (dist_sq / RADIUS_SQ);
                    let amount = (strength as f32 * falloff) as u8;
                    let idx = ny as usize * map.width + nx as usize;
                    map.overlays[idx].pollution = map.overlays[idx].pollution.saturating_add(amount);
                }
            }
        }

        // Parks scrub nearby pollution (radius 3, -20 per tile)
        let mut park_scrubs: Vec<(usize, usize)> = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) == Tile::Park {
                    park_scrubs.push((x, y));
                }
            }
        }
        for (px, py) in park_scrubs {
            for dy in -3_i32..=3 {
                for dx in -3_i32..=3 {
                    let nx = px as i32 + dx;
                    let ny = py as i32 + dy;
                    if !map.in_bounds(nx, ny) { continue; }
                    let idx = ny as usize * map.width + nx as usize;
                    map.overlays[idx].pollution = map.overlays[idx].pollution.saturating_sub(20);
                }
            }
        }
    }
}

// ── LandValueSystem ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct LandValueSystem;
impl SimSystem for LandValueSystem {
    fn name(&self) -> &str { "LandValue" }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        let n = map.width * map.height;
        let mut lv: Vec<u16> = vec![80; n]; // baseline

        // Water proximity bonus (radius 5, up to +40)
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) != Tile::Water { continue; }
                for dy in -5_i32..=5 {
                    for dx in -5_i32..=5 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if !map.in_bounds(nx, ny) { continue; }
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        let bonus = ((1.0 - dist / 5.0).max(0.0) * 40.0) as u16;
                        let idx = ny as usize * map.width + nx as usize;
                        lv[idx] = lv[idx].saturating_add(bonus);
                    }
                }
            }
        }

        // Park proximity bonus (radius 4, up to +30)
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) != Tile::Park { continue; }
                for dy in -4_i32..=4 {
                    for dx in -4_i32..=4 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if !map.in_bounds(nx, ny) { continue; }
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        let bonus = ((1.0 - dist / 4.0).max(0.0) * 30.0) as u16;
                        let idx = ny as usize * map.width + nx as usize;
                        lv[idx] = lv[idx].saturating_add(bonus);
                    }
                }
            }
        }

        // Hospital proximity bonus (radius 4, up to +20)
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) != Tile::Hospital { continue; }
                for dy in -4_i32..=4 {
                    for dx in -4_i32..=4 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if !map.in_bounds(nx, ny) { continue; }
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        let bonus = ((1.0 - dist / 4.0).max(0.0) * 20.0) as u16;
                        let idx = ny as usize * map.width + nx as usize;
                        lv[idx] = lv[idx].saturating_add(bonus);
                    }
                }
            }
        }

        // Pollution penalty (each point of pollution reduces land value)
        for i in 0..n {
            let penalty = map.overlays[i].pollution as u16 / 3;
            lv[i] = lv[i].saturating_sub(penalty);
        }

        // Write back (clamped to u8)
        for i in 0..n {
            map.overlays[i].land_value = lv[i].min(255) as u8;
        }
    }
}

// ── PoliceSystem ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PoliceSystem;
impl SimSystem for PoliceSystem {
    fn name(&self) -> &str { "Police" }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        // Baseline crime (higher in dense zones)
        for i in 0..map.tiles.len() {
            map.overlays[i].crime = match map.tiles[i] {
                Tile::ResHigh | Tile::CommHigh | Tile::IndHeavy => 90,
                Tile::ResMed | Tile::CommLow | Tile::IndLight => 60,
                Tile::ResLow => 40,
                _ => 20,
            };
        }

        // Police stations reduce crime in radius 12 (up to -70)
        const RADIUS: i32 = 12;
        const RADIUS_SQ: f32 = (RADIUS * RADIUS) as f32;
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) != Tile::Police { continue; }
                for dy in -RADIUS..=RADIUS {
                    for dx in -RADIUS..=RADIUS {
                        let dist_sq = (dx * dx + dy * dy) as f32;
                        if dist_sq > RADIUS_SQ { continue; }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if !map.in_bounds(nx, ny) { continue; }
                        let falloff = 1.0 - (dist_sq / RADIUS_SQ);
                        let reduction = (70.0 * falloff) as u8;
                        let idx = ny as usize * map.width + nx as usize;
                        map.overlays[idx].crime = map.overlays[idx].crime.saturating_sub(reduction);
                    }
                }
            }
        }
    }
}

// ── FireSystem ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct FireSystem;
impl SimSystem for FireSystem {
    fn name(&self) -> &str { "Fire" }
    fn tick(&mut self, map: &mut Map, _sim: &mut SimState) {
        // Baseline fire risk (higher in dense/industrial areas)
        for i in 0..map.tiles.len() {
            map.overlays[i].fire_risk = match map.tiles[i] {
                Tile::IndHeavy => 110,
                Tile::IndLight | Tile::PowerPlant => 80,
                Tile::ResHigh | Tile::CommHigh => 60,
                Tile::ResMed | Tile::CommLow => 40,
                Tile::ResLow => 25,
                _ => 10,
            };
        }

        // Fire stations reduce fire risk in radius 12 (up to -80)
        const RADIUS: i32 = 12;
        const RADIUS_SQ: f32 = (RADIUS * RADIUS) as f32;
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) != Tile::Fire { continue; }
                for dy in -RADIUS..=RADIUS {
                    for dx in -RADIUS..=RADIUS {
                        let dist_sq = (dx * dx + dy * dy) as f32;
                        if dist_sq > RADIUS_SQ { continue; }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if !map.in_bounds(nx, ny) { continue; }
                        let falloff = 1.0 - (dist_sq / RADIUS_SQ);
                        let reduction = (80.0 * falloff) as u8;
                        let idx = ny as usize * map.width + nx as usize;
                        map.overlays[idx].fire_risk = map.overlays[idx].fire_risk.saturating_sub(reduction);
                    }
                }
            }
        }
    }
}

// ── GrowthSystem ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct GrowthSystem;
impl SimSystem for GrowthSystem {
    fn name(&self) -> &str { "Growth" }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        growth::tick_growth(map, sim);
    }
}

// ── FinanceSystem ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct FinanceSystem;
impl SimSystem for FinanceSystem {
    fn name(&self) -> &str { "Finance" }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        // Monthly maintenance costs
        let road_tiles        = count_tiles(map, |t| matches!(t, Tile::Road | Tile::RoadPowerLine)) as i64;
        let power_line_tiles  = count_tiles(map, |t| matches!(t, Tile::PowerLine | Tile::RoadPowerLine)) as i64;
        let power_plant_tiles = count_tiles(map, |t| t == Tile::PowerPlant) as i64;
        let police_tiles      = count_tiles(map, |t| t == Tile::Police) as i64;
        let fire_tiles        = count_tiles(map, |t| t == Tile::Fire) as i64;
        let park_tiles        = count_tiles(map, |t| t == Tile::Park) as i64;

        let road_monthly        = road_tiles        * 1;
        let power_line_monthly  = power_line_tiles  * 1;
        let power_plant_monthly = power_plant_tiles * 5;
        let police_monthly      = police_tiles      * 10;
        let fire_monthly        = fire_tiles        * 10;
        let park_monthly        = park_tiles        * 2;

        let maintenance = road_monthly + power_line_monthly + power_plant_monthly
                        + police_monthly + fire_monthly + park_monthly;
        sim.treasury -= maintenance;

        // Annual tax collection (month 1 = start of new year)
        let annual_tax = {
            let tax_per_capita = (sim.tax_rate as f32 / 9.0) * 5.0;
            (sim.population as f32 * tax_per_capita) as i64
        };
        if sim.month == 1 {
            sim.treasury += annual_tax;
        }

        // Annualised net income: taxes collected per year minus yearly maintenance cost
        sim.last_income = annual_tax - maintenance * 12;

        // Populate per-category breakdown for the budget popup
        sim.last_breakdown = MaintenanceBreakdown {
            roads:        road_monthly        * 12,
            power_lines:  power_line_monthly  * 12,
            power_plants: power_plant_monthly * 12,
            police:       police_monthly      * 12,
            fire:         fire_monthly        * 12,
            parks:        park_monthly        * 12,
            total:        maintenance         * 12,
            annual_tax,
        };

        // Recalculate demand
        let res  = count_tiles(map, |t| matches!(t, Tile::ZoneRes  | Tile::ResLow  | Tile::ResMed  | Tile::ResHigh)) as f32;
        let comm = count_tiles(map, |t| matches!(t, Tile::ZoneComm | Tile::CommLow | Tile::CommHigh)) as f32;
        let ind  = count_tiles(map, |t| matches!(t, Tile::ZoneInd  | Tile::IndLight | Tile::IndHeavy)) as f32;

        let total = (res + comm + ind).max(1.0);
        let current_res_ratio  = res  / total;
        let current_comm_ratio = comm / total;
        let current_ind_ratio  = ind  / total;

        let ideal_res  = 0.50;
        let ideal_comm = 0.125;
        let ideal_ind  = 0.375;

        let tax_modifier = (9.0 - sim.tax_rate as f32) * 0.05;
        let growth_boost = if sim.population < 1000 { 0.5 } else { 0.0 };

        sim.demand_res  = (ideal_res  - current_res_ratio  + tax_modifier + growth_boost).clamp(-1.0, 1.0);
        sim.demand_comm = (ideal_comm - current_comm_ratio + tax_modifier + growth_boost).clamp(-1.0, 1.0);
        sim.demand_ind  = (ideal_ind  - current_ind_ratio  + tax_modifier + growth_boost).clamp(-1.0, 1.0);
    }
}

// ── HistorySystem ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct HistorySystem;
impl SimSystem for HistorySystem {
    fn name(&self) -> &str { "History" }
    fn tick(&mut self, _map: &mut Map, sim: &mut SimState) {
        sim.demand_history_res.push(sim.demand_res);
        sim.demand_history_comm.push(sim.demand_comm);
        sim.demand_history_ind.push(sim.demand_ind);
        sim.treasury_history.push(sim.treasury);

        if sim.demand_history_res.len() > 24 {
            sim.demand_history_res.remove(0);
            sim.demand_history_comm.remove(0);
            sim.demand_history_ind.remove(0);
            sim.treasury_history.remove(0);
        }
    }
}

// ── FireSpreadSystem ──────────────────────────────────────────────────────────
// Active fire disaster: spontaneous ignition, spreading, tile damage,
// and suppression by nearby fire stations.

#[derive(Debug)]
pub struct FireSpreadSystem;
impl SimSystem for FireSpreadSystem {
    fn name(&self) -> &str { "FireSpread" }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        if !sim.disasters.fire_enabled { return; }

        let mut rng = rand::thread_rng();
        let w = map.width;
        let h = map.height;

        // 1. Spontaneous ignition: buildings with high fire_risk may catch fire
        for y in 0..h {
            for x in 0..w {
                if !map.tiles[y * w + x].is_building() { continue; }
                let o = &map.overlays[y * w + x];
                if o.on_fire { continue; }
                // Base rate ~0.02% per tick at max risk; scaled down by fire coverage
                let chance = (o.fire_risk as f32 / 255.0) * 0.0002;
                if rng.gen::<f32>() < chance {
                    map.overlays[y * w + x].on_fire = true;
                }
            }
        }

        // 2. Fire spreads to adjacent buildings
        let mut new_fires: Vec<(usize, usize)> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if !map.overlays[y * w + x].on_fire { continue; }
                for (nx, ny, tile) in map.neighbors4(x, y) {
                    if tile.is_building() && !map.overlays[ny * w + nx].on_fire {
                        let spread_chance = map.overlays[ny * w + nx].fire_risk as f32 / 255.0 * 0.04;
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

        // 3. Fire damage: burning buildings have a 1% chance per tick to be destroyed
        let mut damaged: Vec<(usize, usize)> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if map.overlays[y * w + x].on_fire && rng.gen::<f32>() < 0.01 {
                    damaged.push((x, y));
                }
            }
        }
        for (x, y) in damaged {
            // Downgrade: ResHigh→ResMed→ResLow→ZoneRes, etc.
            let downgraded = match map.tiles[y * w + x] {
                Tile::ResHigh  => Some(Tile::ResMed),
                Tile::ResMed   => Some(Tile::ResLow),
                Tile::ResLow   => Some(Tile::ZoneRes),
                Tile::CommHigh => Some(Tile::CommLow),
                Tile::CommLow  => Some(Tile::ZoneComm),
                Tile::IndHeavy => Some(Tile::IndLight),
                Tile::IndLight => Some(Tile::ZoneInd),
                _              => Some(Tile::Grass),
            };
            if let Some(t) = downgraded {
                map.tiles[y * w + x] = t;
                map.overlays[y * w + x].on_fire = false; // fire consumes the building
            }
        }

        // 4. Fire stations suppress fires within radius 8
        const RADIUS: i32 = 8;
        const RADIUS_SQ: f32 = (RADIUS * RADIUS) as f32;
        for y in 0..h {
            for x in 0..w {
                if map.get(x, y) != Tile::Fire { continue; }
                for dy in -RADIUS..=RADIUS {
                    for dx in -RADIUS..=RADIUS {
                        let dist_sq = (dx * dx + dy * dy) as f32;
                        if dist_sq > RADIUS_SQ { continue; }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if !map.in_bounds(nx, ny) { continue; }
                        let idx = ny as usize * w + nx as usize;
                        if map.overlays[idx].on_fire {
                            let falloff = 1.0 - (dist_sq / RADIUS_SQ);
                            let suppress_chance = 0.08 * falloff;
                            if rng.gen::<f32>() < suppress_chance {
                                map.overlays[idx].on_fire = false;
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── FloodSystem ───────────────────────────────────────────────────────────────
// Once per year, small chance that water floods one adjacent tile per water body.

#[derive(Debug)]
pub struct FloodSystem;
impl SimSystem for FloodSystem {
    fn name(&self) -> &str { "Flood" }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        if !sim.disasters.flood_enabled { return; }
        // Only trigger at the start of the year, with a ~10% annual chance
        if sim.month != 6 { return; }
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() > 0.10 { return; }

        // Collect all water-adjacent non-water, non-road tiles
        let mut floodable: Vec<(usize, usize)> = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) == Tile::Water { continue; }
                if matches!(map.get(x, y), Tile::Road | Tile::Rail | Tile::RoadPowerLine) { continue; }
                let near_water = map.neighbors4(x, y).iter().any(|(_, _, t)| *t == Tile::Water);
                if near_water {
                    floodable.push((x, y));
                }
            }
        }

        // Flood a random selection (up to 5 tiles per event)
        let count = rng.gen_range(1..=5_usize.min(floodable.len().max(1)));
        for _ in 0..count {
            if floodable.is_empty() { break; }
            let i = rng.gen_range(0..floodable.len());
            let (fx, fy) = floodable.swap_remove(i);
            map.set(fx, fy, Tile::Water);
            map.overlays[fy * map.width + fx].on_fire = false;
        }
    }
}

// ── TornadoSystem ─────────────────────────────────────────────────────────────
// Very rare event: a tornado carves a random path across the map, bulldozing tiles.

#[derive(Debug)]
pub struct TornadoSystem;
impl SimSystem for TornadoSystem {
    fn name(&self) -> &str { "Tornado" }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        if !sim.disasters.tornado_enabled { return; }
        // ~2% chance per year, checked on month 3
        if sim.month != 3 { return; }
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() > 0.02 { return; }

        // Random starting edge
        let (mut x, mut y) = match rng.gen_range(0..4) {
            0 => (rng.gen_range(0..map.width),  0),
            1 => (rng.gen_range(0..map.width),  map.height - 1),
            2 => (0,                             rng.gen_range(0..map.height)),
            _ => (map.width - 1,                 rng.gen_range(0..map.height)),
        };

        // Random direction (biased toward centre)
        let cx = map.width  as i32 / 2;
        let cy = map.height as i32 / 2;
        let mut dx = ((cx - x as i32).signum() + rng.gen_range(-1..=1)).clamp(-1, 1);
        let dy = ((cy - y as i32).signum() + rng.gen_range(-1..=1)).clamp(-1, 1);
        if dx == 0 && dy == 0 { dx = 1; }

        let path_len = rng.gen_range(12..30_usize);
        for _ in 0..path_len {
            if !map.in_bounds(x as i32, y as i32) { break; }
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
            if rng.gen::<f32>() < 0.3 {
                dx = (dx + rng.gen_range(-1..=1)).clamp(-1, 1);
                if dx == 0 && dy == 0 { dx = 1; }
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn count_tiles(map: &Map, pred: impl Fn(Tile) -> bool) -> usize {
    map.tiles.iter().filter(|&&t| pred(t)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::system::SimSystem;
    use crate::core::map::Map;
    use crate::core::sim::SimState;

    fn run_finance(map: &mut Map, sim: &mut SimState) {
        FinanceSystem.tick(map, sim);
    }

    #[test]
    fn finance_breakdown_roads_annual_is_12x_monthly() {
        let mut map = Map::new(5, 5);
        map.set(2, 2, Tile::Road);
        map.set(2, 3, Tile::Road);
        let mut sim = SimState::default();
        run_finance(&mut map, &mut sim);
        // 2 road tiles × $1/month × 12 months = $24
        assert_eq!(sim.last_breakdown.roads, 24);
    }

    #[test]
    fn finance_breakdown_total_equals_sum_of_parts() {
        let mut map = Map::new(5, 5);
        map.set(0, 0, Tile::Road);
        map.set(1, 0, Tile::PowerLine);
        let mut sim = SimState::default();
        run_finance(&mut map, &mut sim);
        let b = &sim.last_breakdown;
        assert_eq!(b.total, b.roads + b.power_lines + b.power_plants + b.police + b.fire + b.parks);
    }

    #[test]
    fn finance_breakdown_annual_tax_matches_last_income_at_zero_maintenance() {
        let mut map = Map::new(5, 5); // empty map → no maintenance
        let mut sim = SimState::default();
        sim.population = 100;
        sim.tax_rate = 9;
        run_finance(&mut map, &mut sim);
        // annual_tax should equal last_income when maintenance is 0
        assert_eq!(sim.last_breakdown.annual_tax, sim.last_income);
    }
}
