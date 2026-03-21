use crate::core::map::{Map, Tile};
use crate::core::sim::constants::{
    GROWTH_BOOST_AMOUNT, GROWTH_BOOST_THRESHOLD, IDEAL_COMM_RATIO, IDEAL_IND_RATIO,
    IDEAL_RES_RATIO, TAX_BASELINE_RATE,
};
use crate::core::sim::economy::annual_tax_from_base;
use crate::core::sim::system::SimSystem;
use crate::core::sim::{MaintenanceBreakdown, SimState};

// ── FinanceSystem ─────────────────────────────────────────────────────────────

#[derive(Debug)]
struct TileCounts {
    road_tiles: i64,
    highway_tiles: i64,
    power_line_tiles: i64,
    rail_tiles: i64,
    bus_depot_tiles: i64,
    rail_depot_tiles: i64,
    subway_station_tiles: i64,
    water_structure_tiles: i64,
    coal_plant_tiles: i64,
    gas_plant_tiles: i64,
    police_tiles: i64,
    fire_tiles: i64,
    park_tiles: i64,
    res_tiles: i64,
    comm_tiles: i64,
    ind_tiles: i64,
}

impl TileCounts {
    fn zero() -> Self {
        Self {
            road_tiles: 0,
            highway_tiles: 0,
            power_line_tiles: 0,
            rail_tiles: 0,
            bus_depot_tiles: 0,
            rail_depot_tiles: 0,
            subway_station_tiles: 0,
            water_structure_tiles: 0,
            coal_plant_tiles: 0,
            gas_plant_tiles: 0,
            police_tiles: 0,
            fire_tiles: 0,
            park_tiles: 0,
            res_tiles: 0,
            comm_tiles: 0,
            ind_tiles: 0,
        }
    }

    fn count(map: &Map) -> Self {
        let mut c = Self::zero();
        for &tile in &map.tiles {
            let is_road = matches!(tile, Tile::Road | Tile::RoadPowerLine | Tile::Onramp);
            let is_power_line = matches!(tile, Tile::PowerLine | Tile::RoadPowerLine);
            if is_road {
                c.road_tiles += 1;
            }
            if is_power_line {
                c.power_line_tiles += 1;
            }
            match tile {
                Tile::Highway => c.highway_tiles += 1,
                Tile::Rail => c.rail_tiles += 1,
                Tile::BusDepot => c.bus_depot_tiles += 1,
                Tile::RailDepot => c.rail_depot_tiles += 1,
                Tile::SubwayStation => c.subway_station_tiles += 1,
                Tile::WaterPump | Tile::WaterTower | Tile::WaterTreatment | Tile::Desalination => {
                    c.water_structure_tiles += 1
                }
                Tile::PowerPlantCoal => c.coal_plant_tiles += 1,
                Tile::PowerPlantGas => c.gas_plant_tiles += 1,
                Tile::Police => c.police_tiles += 1,
                Tile::Fire => c.fire_tiles += 1,
                Tile::Park => c.park_tiles += 1,
                Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh => c.res_tiles += 1,
                Tile::ZoneComm | Tile::CommLow | Tile::CommHigh => c.comm_tiles += 1,
                Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy => c.ind_tiles += 1,
                _ => {}
            }
        }
        c
    }
}

struct UndergroundCounts {
    water_pipe_tiles: i64,
    subway_tiles: i64,
}

impl UndergroundCounts {
    fn count(map: &Map) -> Self {
        let mut water_pipe_tiles = 0i64;
        let mut subway_tiles = 0i64;
        for tile in &map.underground {
            let t = tile.unwrap_or_default();
            if t.water_pipe {
                water_pipe_tiles += 1;
            }
            if t.subway {
                subway_tiles += 1;
            }
        }
        Self {
            water_pipe_tiles,
            subway_tiles,
        }
    }
}

#[derive(Debug)]
pub struct FinanceSystem;
impl SimSystem for FinanceSystem {
    fn name(&self) -> &str {
        "Finance"
    }
    fn tick(&mut self, map: &mut Map, sim: &mut SimState) {
        // sim.pop was already updated by GrowthSystem earlier this tick; read directly.
        let c = TileCounts::count(map);
        let u = UndergroundCounts::count(map);

        let road_monthly = c.road_tiles;
        let highway_monthly = c.highway_tiles * 3;
        let rail_monthly = c.rail_tiles * 2;
        let power_line_monthly = c.power_line_tiles;
        let water_monthly = u.water_pipe_tiles + c.water_structure_tiles * 6;
        let transit_monthly = u.subway_tiles * 3
            + c.bus_depot_tiles * 8
            + c.rail_depot_tiles * 12
            + c.subway_station_tiles * 10;
        let power_plant_monthly = (c.coal_plant_tiles / 16) * 100 + (c.gas_plant_tiles / 16) * 150;
        let police_monthly = c.police_tiles * 10;
        let fire_monthly = c.fire_tiles * 10;
        let park_monthly = c.park_tiles * 2;

        let maintenance = road_monthly
            + highway_monthly
            + rail_monthly
            + power_line_monthly
            + water_monthly
            + transit_monthly
            + power_plant_monthly
            + police_monthly
            + fire_monthly
            + park_monthly;
        sim.economy.treasury -= maintenance;

        let residential_tax =
            annual_tax_from_base(sim.pop.residential_population, sim.economy.tax_rates.residential);
        let commercial_tax = annual_tax_from_base(sim.pop.commercial_jobs, sim.economy.tax_rates.commercial);
        let industrial_tax = annual_tax_from_base(sim.pop.industrial_jobs, sim.economy.tax_rates.industrial);
        let annual_tax = residential_tax + commercial_tax + industrial_tax;
        sim.economy.treasury += annual_tax / 12;

        sim.economy.last_income = annual_tax - maintenance * 12;

        sim.economy.last_breakdown = MaintenanceBreakdown {
            roads: road_monthly * 12,
            power_lines: power_line_monthly * 12,
            power_plants: power_plant_monthly * 12,
            police: police_monthly * 12,
            fire: fire_monthly * 12,
            parks: park_monthly * 12,
            residential_tax,
            commercial_tax,
            industrial_tax,
            total: maintenance * 12,
            annual_tax,
        };

        let res = c.res_tiles as f32;
        let comm = c.comm_tiles as f32;
        let ind = c.ind_tiles as f32;

        let total = (res + comm + ind).max(1.0);
        let current_res_ratio = res / total;
        let current_comm_ratio = comm / total;
        let current_ind_ratio = ind / total;

        let res_tax_modifier =
            (TAX_BASELINE_RATE - sim.economy.tax_rates.residential as f32) * 0.05;
        let comm_tax_modifier =
            (TAX_BASELINE_RATE - sim.economy.tax_rates.commercial as f32) * 0.05;
        let ind_tax_modifier =
            (TAX_BASELINE_RATE - sim.economy.tax_rates.industrial as f32) * 0.05;
        let growth_boost = if sim.pop.population < GROWTH_BOOST_THRESHOLD {
            GROWTH_BOOST_AMOUNT
        } else {
            0.0
        };

        sim.demand.res =
            (IDEAL_RES_RATIO - current_res_ratio + res_tax_modifier + growth_boost).clamp(-1.0, 1.0);
        sim.demand.comm =
            (IDEAL_COMM_RATIO - current_comm_ratio + comm_tax_modifier + growth_boost).clamp(-1.0, 1.0);
        sim.demand.ind =
            (IDEAL_IND_RATIO - current_ind_ratio + ind_tax_modifier + growth_boost).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::economy::annual_tax_from_base;
    use crate::core::sim::system::SimSystem;
    use crate::core::sim::SimState;

    fn run_finance(map: &mut Map, sim: &mut SimState) {
        // Seed sim.pop as GrowthSystem would before FinanceSystem runs in a real tick.
        use crate::core::sim::economy::compute_sector_stats;
        let stats = compute_sector_stats(map);
        sim.pop.residential_population = stats.residential_population;
        sim.pop.commercial_jobs = stats.commercial_jobs;
        sim.pop.industrial_jobs = stats.industrial_jobs;
        sim.pop.population = stats.residential_population;
        FinanceSystem.tick(map, sim);
    }

    #[test]
    fn finance_tile_counts_road_powerline_both_increment() {
        let mut map = Map::new(3, 1);
        map.set(0, 0, Tile::Road);
        map.set(1, 0, Tile::RoadPowerLine);
        map.set(2, 0, Tile::PowerLine);

        let c = TileCounts::count(&map);
        assert_eq!(
            c.road_tiles, 2,
            "RoadPowerLine and Road both count as roads"
        );
        assert_eq!(
            c.power_line_tiles, 2,
            "RoadPowerLine and PowerLine both count as power lines"
        );
    }

    #[test]
    fn finance_tile_counts_empty_map_all_zero() {
        let map = Map::new(10, 10);
        let c = TileCounts::count(&map);
        assert_eq!(c.road_tiles, 0);
        assert_eq!(c.highway_tiles, 0);
        assert_eq!(c.power_line_tiles, 0);
        assert_eq!(c.rail_tiles, 0);
        assert_eq!(c.res_tiles, 0);
        assert_eq!(c.comm_tiles, 0);
        assert_eq!(c.ind_tiles, 0);
    }

    #[test]
    fn finance_tile_counts_matches_expected_categories() {
        let mut map = Map::new(10, 10);
        map.set(0, 0, Tile::Road);
        map.set(1, 0, Tile::Highway);
        map.set(2, 0, Tile::Rail);
        map.set(3, 0, Tile::BusDepot);
        map.set(4, 0, Tile::RailDepot);
        map.set(5, 0, Tile::SubwayStation);
        map.set(6, 0, Tile::WaterPump);
        map.set(7, 0, Tile::PowerPlantCoal);
        map.set(8, 0, Tile::Police);
        map.set(9, 0, Tile::Park);

        let c = TileCounts::count(&map);
        assert_eq!(c.road_tiles, 1);
        assert_eq!(c.highway_tiles, 1);
        assert_eq!(c.rail_tiles, 1);
        assert_eq!(c.bus_depot_tiles, 1);
        assert_eq!(c.rail_depot_tiles, 1);
        assert_eq!(c.subway_station_tiles, 1);
        assert_eq!(c.water_structure_tiles, 1);
        assert_eq!(c.coal_plant_tiles, 1);
        assert_eq!(c.police_tiles, 1);
        assert_eq!(c.park_tiles, 1);
    }

    #[test]
    fn finance_tile_counts_zones() {
        let mut map = Map::new(10, 10);
        map.set(0, 0, Tile::ZoneRes);
        map.set(1, 0, Tile::ResLow);
        map.set(2, 0, Tile::ZoneComm);
        map.set(3, 0, Tile::CommHigh);
        map.set(4, 0, Tile::ZoneInd);
        map.set(5, 0, Tile::IndLight);

        let c = TileCounts::count(&map);
        assert_eq!(c.res_tiles, 2);
        assert_eq!(c.comm_tiles, 2);
        assert_eq!(c.ind_tiles, 2);
    }

    #[test]
    fn finance_underground_counts_both_types() {
        let mut map = Map::new(3, 1);
        map.set_water_pipe(0, 0, true);
        map.set_water_pipe(1, 0, true);
        map.set_subway_tunnel(2, 0, true);

        let u = UndergroundCounts::count(&map);
        assert_eq!(u.water_pipe_tiles, 2);
        assert_eq!(u.subway_tiles, 1);
    }

    #[test]
    fn finance_underground_counts_empty_map() {
        let map = Map::new(5, 5);
        let u = UndergroundCounts::count(&map);
        assert_eq!(u.water_pipe_tiles, 0);
        assert_eq!(u.subway_tiles, 0);
    }

    #[test]
    fn finance_breakdown_roads_annual_is_12x_monthly() {
        let mut map = Map::new(5, 5);
        map.set(2, 2, Tile::Road);
        map.set(2, 3, Tile::Road);
        let mut sim = SimState::default();
        run_finance(&mut map, &mut sim);
        // 2 road tiles × $1/month × 12 months = $24
        assert_eq!(sim.economy.last_breakdown.roads, 24);
    }

    #[test]
    fn finance_breakdown_total_equals_sum_of_parts() {
        let mut map = Map::new(5, 5);
        map.set(0, 0, Tile::Road);
        map.set(1, 0, Tile::PowerLine);
        let mut sim = SimState::default();
        run_finance(&mut map, &mut sim);
        let b = &sim.economy.last_breakdown;
        assert_eq!(
            b.total,
            b.roads + b.power_lines + b.power_plants + b.police + b.fire + b.parks
        );
    }

    #[test]
    fn finance_sector_taxes_sum_to_total_tax() {
        let mut map = Map::new(5, 5);
        map.set(0, 0, Tile::ResLow);
        map.set(1, 0, Tile::CommHigh);
        map.set(2, 0, Tile::IndLight);

        let mut sim = SimState::default();
        sim.economy.tax_rates.residential = 10;
        sim.economy.tax_rates.commercial = 12;
        sim.economy.tax_rates.industrial = 8;

        run_finance(&mut map, &mut sim);

        let b = &sim.economy.last_breakdown;
        assert_eq!(
            b.annual_tax,
            b.residential_tax + b.commercial_tax + b.industrial_tax
        );
        assert_eq!(sim.pop.population, sim.pop.residential_population);
    }

    #[test]
    fn finance_sector_tax_changes_only_matching_revenue() {
        let mut map = Map::new(5, 5);
        map.set(0, 0, Tile::ResHigh);
        map.set(1, 0, Tile::CommHigh);
        map.set(2, 0, Tile::IndHeavy);

        let mut base_sim = SimState::default();
        run_finance(&mut map, &mut base_sim);

        let base = base_sim.economy.last_breakdown;

        let mut higher_res = SimState::default();
        higher_res.economy.tax_rates.residential = 15;
        run_finance(&mut map, &mut higher_res);

        let changed = higher_res.economy.last_breakdown;
        assert!(changed.residential_tax > base.residential_tax);
        assert_eq!(changed.commercial_tax, base.commercial_tax);
        assert_eq!(changed.industrial_tax, base.industrial_tax);
    }

    #[test]
    fn finance_treasury_receives_one_twelfth_tax_each_month() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::ResHigh);
        let mut sim = SimState::default();
        sim.month = 6;
        sim.economy.tax_rates.residential = 9;
        let annual_tax = annual_tax_from_base(200, 9);
        let before = sim.economy.treasury;
        run_finance(&mut map, &mut sim);
        assert_eq!(sim.economy.treasury - before, annual_tax / 12);
    }

    #[test]
    fn finance_treasury_receives_tax_in_month_1() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::ResHigh);
        let mut sim = SimState::default();
        sim.month = 1;
        sim.economy.tax_rates.residential = 9;
        let annual_tax = annual_tax_from_base(200, 9);
        let before = sim.economy.treasury;
        run_finance(&mut map, &mut sim);
        assert_eq!(sim.economy.treasury - before, annual_tax / 12);
    }

    #[test]
    fn finance_treasury_small_tax_rounds_to_zero_per_month() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::ResLow);
        let mut sim = SimState::default();
        sim.month = 3;
        sim.economy.tax_rates.residential = 1;
        let annual_tax = annual_tax_from_base(10, 1);
        let monthly = annual_tax / 12;
        let before = sim.economy.treasury;
        run_finance(&mut map, &mut sim);
        assert_eq!(sim.economy.treasury - before, monthly);
    }
}
