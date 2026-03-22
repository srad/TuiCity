use crate::core::map::{Map, Tile};
use crate::core::sim::SimState;

use super::types::CityContext;

impl CityContext {
    /// Build a compact city snapshot from the current simulation state and map.
    /// This is the only place that reads game internals for LLM purposes.
    pub fn from_state(sim: &SimState, map: &Map) -> Self {
        let mut avg_pollution: u32 = 0;
        let mut avg_crime: u32 = 0;
        let mut avg_land_value: u32 = 0;
        let mut avg_fire_risk: u32 = 0;
        let mut civic_count: u32 = 0;
        let mut active_fires: usize = 0;
        let mut num_schools: u32 = 0;
        let mut num_hospitals: u32 = 0;
        let mut num_police: u32 = 0;
        let mut num_fire_stations: u32 = 0;
        let mut num_parks: u32 = 0;

        for (i, (&tile, overlay)) in map.tiles.iter().zip(map.overlays.iter()).enumerate() {
            let _ = i;
            if overlay.on_fire {
                active_fires += 1;
            }
            match tile {
                Tile::School => num_schools += 1,
                Tile::Hospital => num_hospitals += 1,
                Tile::Police => num_police += 1,
                Tile::Fire => num_fire_stations += 1,
                Tile::Park => num_parks += 1,
                _ => {}
            }
            if tile.receives_power() {
                civic_count += 1;
                avg_pollution += overlay.pollution as u32;
                avg_crime += overlay.crime as u32;
                avg_land_value += overlay.land_value as u32;
                avg_fire_risk += overlay.fire_risk as u32;
            }
        }

        let avg = |sum: u32| -> u8 {
            if civic_count == 0 {
                0
            } else {
                (sum / civic_count).min(255) as u8
            }
        };

        let pop_delta = {
            let previous = if sim.history.population.len() >= 2 {
                sim.history.population[sim.history.population.len() - 2]
            } else if let Some(previous) = sim.history.population.back() {
                *previous
            } else {
                sim.pop.population
            };
            sim.pop.population as i64 - previous as i64
        };

        let trip_success_rate = if sim.trips.attempts == 0 {
            1.0
        } else {
            sim.trips.successes as f32 / sim.trips.attempts as f32
        };

        CityContext {
            city_name: sim.city_name.clone(),
            year: sim.year,
            month: sim.month,
            population: sim.pop.population,
            treasury: sim.economy.treasury,
            last_income: sim.economy.last_income,
            tax_res: sim.economy.tax_rates.residential,
            tax_comm: sim.economy.tax_rates.commercial,
            tax_ind: sim.economy.tax_rates.industrial,
            demand_res: sim.demand.res,
            demand_comm: sim.demand.comm,
            demand_ind: sim.demand.ind,
            power_produced_mw: sim.utilities.power_produced_mw,
            power_consumed_mw: sim.utilities.power_consumed_mw,
            water_produced: sim.utilities.water_produced_units,
            water_consumed: sim.utilities.water_consumed_units,
            avg_pollution: avg(avg_pollution),
            avg_crime: avg(avg_crime),
            avg_land_value: avg(avg_land_value),
            avg_fire_risk: avg(avg_fire_risk),
            active_fires,
            trip_success_rate,
            pop_delta,
            num_schools,
            num_hospitals,
            num_police,
            num_fire_stations,
            num_parks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::Map;
    use crate::core::sim::SimState;

    #[test]
    fn from_state_on_empty_map() {
        let sim = SimState::default();
        let map = Map::new(4, 4);
        let ctx = CityContext::from_state(&sim, &map);

        assert_eq!(ctx.city_name, "Unnamed City");
        assert_eq!(ctx.year, 1900);
        assert_eq!(ctx.population, 0);
        assert_eq!(ctx.active_fires, 0);
        assert_eq!(ctx.avg_pollution, 0);
    }

    #[test]
    fn counts_civic_buildings() {
        let sim = SimState::default();
        let mut map = Map::new(8, 8);
        map.set(0, 0, Tile::School);
        map.set(1, 0, Tile::Hospital);
        map.set(2, 0, Tile::Police);
        map.set(3, 0, Tile::Fire);
        map.set(4, 0, Tile::Park);
        map.set(5, 0, Tile::Park);

        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.num_schools, 1);
        assert_eq!(ctx.num_hospitals, 1);
        assert_eq!(ctx.num_police, 1);
        assert_eq!(ctx.num_fire_stations, 1);
        assert_eq!(ctx.num_parks, 2);
    }

    #[test]
    fn copies_sim_identity_fields() {
        let mut sim = SimState::default();
        sim.city_name = "Lakewood".to_string();
        sim.year = 1975;
        sim.month = 11;
        let map = Map::new(4, 4);

        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.city_name, "Lakewood");
        assert_eq!(ctx.year, 1975);
        assert_eq!(ctx.month, 11);
    }

    #[test]
    fn copies_economy_fields() {
        let mut sim = SimState::default();
        sim.economy.treasury = -5000;
        sim.economy.last_income = -300;
        sim.economy.tax_rates.residential = 12;
        sim.economy.tax_rates.commercial = 7;
        sim.economy.tax_rates.industrial = 5;
        let map = Map::new(4, 4);

        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.treasury, -5000);
        assert_eq!(ctx.last_income, -300);
        assert_eq!(ctx.tax_res, 12);
        assert_eq!(ctx.tax_comm, 7);
        assert_eq!(ctx.tax_ind, 5);
    }

    #[test]
    fn copies_demand_fields() {
        let mut sim = SimState::default();
        sim.demand.res = 0.75;
        sim.demand.comm = 0.25;
        sim.demand.ind = -0.1;
        let map = Map::new(4, 4);

        let ctx = CityContext::from_state(&sim, &map);
        assert!((ctx.demand_res - 0.75).abs() < f32::EPSILON);
        assert!((ctx.demand_comm - 0.25).abs() < f32::EPSILON);
        assert!((ctx.demand_ind - (-0.1)).abs() < f32::EPSILON);
    }

    #[test]
    fn copies_utility_fields() {
        let mut sim = SimState::default();
        sim.utilities.power_produced_mw = 800;
        sim.utilities.power_consumed_mw = 600;
        sim.utilities.water_produced_units = 300;
        sim.utilities.water_consumed_units = 250;
        let map = Map::new(4, 4);

        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.power_produced_mw, 800);
        assert_eq!(ctx.power_consumed_mw, 600);
        assert_eq!(ctx.water_produced, 300);
        assert_eq!(ctx.water_consumed, 250);
    }

    #[test]
    fn counts_active_fires() {
        let sim = SimState::default();
        let mut map = Map::new(4, 4);
        map.set(0, 0, Tile::ResLow);
        let mut ov = map.get_overlay(0, 0);
        ov.on_fire = true;
        map.set_overlay(0, 0, ov);
        map.set(1, 0, Tile::CommLow);
        let mut ov2 = map.get_overlay(1, 0);
        ov2.on_fire = true;
        map.set_overlay(1, 0, ov2);

        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.active_fires, 2);
    }

    #[test]
    fn computes_averages_over_civic_tiles() {
        let sim = SimState::default();
        let mut map = Map::new(4, 4);

        // Place two buildings with different overlays
        map.set(0, 0, Tile::ResLow);
        map.set_overlay(
            0,
            0,
            crate::core::map::TileOverlay {
                pollution: 100,
                crime: 200,
                land_value: 50,
                fire_risk: 80,
                power_level: 255,
                ..Default::default()
            },
        );
        map.set(1, 0, Tile::CommLow);
        map.set_overlay(
            1,
            0,
            crate::core::map::TileOverlay {
                pollution: 200,
                crime: 100,
                land_value: 150,
                fire_risk: 40,
                power_level: 255,
                ..Default::default()
            },
        );

        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.avg_pollution, 150); // (100+200)/2
        assert_eq!(ctx.avg_crime, 150); // (200+100)/2
        assert_eq!(ctx.avg_land_value, 100); // (50+150)/2
        assert_eq!(ctx.avg_fire_risk, 60); // (80+40)/2
    }

    #[test]
    fn trip_success_rate_default_is_one() {
        let sim = SimState::default();
        let map = Map::new(4, 4);
        let ctx = CityContext::from_state(&sim, &map);
        assert!((ctx.trip_success_rate - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn trip_success_rate_computed_correctly() {
        let mut sim = SimState::default();
        sim.trips.attempts = 100;
        sim.trips.successes = 75;
        let map = Map::new(4, 4);
        let ctx = CityContext::from_state(&sim, &map);
        assert!((ctx.trip_success_rate - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn pop_delta_with_no_history() {
        let sim = SimState::default();
        let map = Map::new(4, 4);
        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.pop_delta, 0);
    }

    #[test]
    fn pop_delta_with_growth() {
        let mut sim = SimState::default();
        sim.pop.population = 1500;
        sim.history.population.push_back(1000);
        sim.history.population.push_back(1500);
        let map = Map::new(4, 4);
        let ctx = CityContext::from_state(&sim, &map);
        assert_eq!(ctx.pop_delta, 500);
    }
}
