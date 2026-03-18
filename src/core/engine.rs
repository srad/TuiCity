#![allow(dead_code)]
use super::map::{Map, Tile};
use super::sim::{SimState, TaxSector};
use super::tool::Tool;

#[derive(Debug, Clone)]
pub enum EngineCommand {
    PlaceTool {
        tool: Tool,
        x: usize,
        y: usize,
    },
    PlaceLine {
        tool: Tool,
        path: Vec<(usize, usize)>,
    },
    PlaceRect {
        tool: Tool,
        tiles: Vec<(usize, usize)>,
    },
    AdvanceMonth,
    SetCityName(String),
    SetTaxRate {
        sector: TaxSector,
        rate: u8,
    },
    ReplaceState {
        map: Map,
        sim: SimState,
    },
    SetPaused(bool),
    SetDisasters(super::sim::DisasterConfig),
}

use super::sim::system::SimSystem;
use super::sim::systems::*;

pub struct SimulationEngine {
    pub map: Map,
    pub sim: SimState,
    pub is_paused: bool,
    pub systems: Vec<Box<dyn SimSystem>>,
}

impl SimulationEngine {
    pub fn new(map: Map, sim: SimState) -> Self {
        let mut engine = Self {
            map,
            sim,
            is_paused: false,
            systems: vec![
                Box::new(PowerSystem),        // 1. power grid
                Box::new(PollutionSystem),    // 2. pollution (from industry)
                Box::new(LandValueSystem),    // 3. land value (uses pollution)
                Box::new(PoliceSystem),       // 4. crime overlays
                Box::new(FireSystem),         // 5. fire-risk overlays
                Box::new(GrowthSystem),       // 6. zone → building (uses all overlays)
                Box::new(FireSpreadSystem),   // 7. active fire disaster
                Box::new(FloodSystem),        // 8. flood disaster
                Box::new(TornadoSystem),      // 9. tornado disaster
                Box::new(FinanceSystem),      // 10. taxes & maintenance
                Box::new(HistorySystem),      // 11. record history
            ],
        };
        engine.refresh_sector_stats();
        engine
    }

    pub fn execute_command(&mut self, cmd: EngineCommand) -> Result<(), String> {
        match cmd {
            EngineCommand::PlaceTool { tool, x, y } => self.place_tool(tool, x, y),
            EngineCommand::PlaceLine { tool, path } => self.place_line(tool, path),
            EngineCommand::PlaceRect { tool, tiles } => self.place_rect(tool, tiles),
            EngineCommand::AdvanceMonth => {
                if !self.is_paused {
                    self.sim.month += 1;
                    if self.sim.month > 12 {
                        self.sim.month = 1;
                        self.sim.year += 1;
                    }
                    
                    for system in &mut self.systems {
                        system.tick(&mut self.map, &mut self.sim);
                    }
                }
                Ok(())
            }
            EngineCommand::SetCityName(name) => {
                self.sim.city_name = name;
                Ok(())
            }
            EngineCommand::SetTaxRate { sector, rate } => {
                self.sim.tax_rates.set(sector, rate);
                Ok(())
            }
            EngineCommand::ReplaceState { map, sim } => {
                self.map = map;
                self.sim = sim;
                self.refresh_sector_stats();
                Ok(())
            }
            EngineCommand::SetPaused(paused) => {
                self.is_paused = paused;
                Ok(())
            }
            EngineCommand::SetDisasters(cfg) => {
                self.sim.disasters = cfg;
                Ok(())
            }
        }
    }

    fn place_line(&mut self, tool: Tool, path: Vec<(usize, usize)>) -> Result<(), String> {
        let cost_per = tool.cost();
        let mut total_cost = 0;
        let mut tiles_to_place = Vec::new();

        for (x, y) in path {
            if x >= self.map.width || y >= self.map.height {
                continue;
            }
            let existing = self.map.get(x, y);
            if !tool.can_place(existing) {
                continue;
            }
            if self.sim.treasury < total_cost + cost_per {
                return Err("Insufficient funds!".to_string());
            }

            let new_tile = match (tool, existing) {
                (Tool::Road, Tile::PowerLine) | (Tool::PowerLine, Tile::Road) => {
                    Tile::RoadPowerLine
                }
                _ => match tool.target_tile() {
                    Some(t) => t,
                    None => continue,
                },
            };

            tiles_to_place.push((x, y, new_tile));
            total_cost += cost_per;
        }

        for (x, y, tile) in tiles_to_place {
            self.map.set(x, y, tile);
            // If we're placing something over a plant tile (somehow), we should remove it.
            // But plants are 4x4, so this is unlikely via line drag.
            // However, we should be safe.
            self.sim.plants.remove(&(x, y));
        }
        self.sim.treasury -= total_cost;
        self.refresh_sector_stats();

        Ok(())
    }

    fn place_rect(&mut self, tool: Tool, tiles: Vec<(usize, usize)>) -> Result<(), String> {
        let target = match tool.target_tile() {
            Some(t) => t,
            None => return Ok(()),
        };
        let cost_per = tool.cost();
        let mut total_cost = 0;
        let mut tiles_to_place = Vec::new();

        for (x, y) in tiles {
            if x >= self.map.width || y >= self.map.height {
                continue;
            }
            if !tool.can_place(self.map.get(x, y)) {
                continue;
            }
            if self.sim.treasury < total_cost + cost_per {
                return Err("Insufficient funds!".to_string());
            }
            tiles_to_place.push((x, y, target));
            total_cost += cost_per;
        }

        for (x, y, tile) in tiles_to_place {
            self.map.set(x, y, tile);
            self.sim.plants.remove(&(x, y));
        }
        self.sim.treasury -= total_cost;
        self.refresh_sector_stats();

        Ok(())
    }

    fn place_tool(&mut self, tool: Tool, x: usize, y: usize) -> Result<(), String> {
        let (fw, fh) = tool.footprint();

        if fw > 1 || fh > 1 {
            let ax = x
                .saturating_sub(fw / 2)
                .min(self.map.width.saturating_sub(fw));
            let ay = y
                .saturating_sub(fh / 2)
                .min(self.map.height.saturating_sub(fh));

            for dy in 0..fh {
                for dx in 0..fw {
                    if !tool.can_place(self.map.get(ax + dx, ay + dy)) {
                        return Err("Cannot place tool here".to_string());
                    }
                }
            }

            let cost = tool.cost();
            if self.sim.treasury < cost {
                return Err("Insufficient funds!".to_string());
            }

            let new_tile = match tool.target_tile() {
                Some(t) => t,
                None => return Ok(()),
            };

            for dy in 0..fh {
                for dx in 0..fw {
                    self.map.set(ax + dx, ay + dy, new_tile);
                    // Remove any existing plant metadata if we overwrite it
                    self.sim.plants.remove(&(ax + dx, ay + dy));
                }
            }
            
            // Register new plant if applicable
            match tool {
                Tool::PowerPlantCoal => {
                    self.sim.plants.insert((ax, ay), super::sim::PlantState {
                        age_months: 0,
                        max_life_months: 50 * 12, // 50 years
                        capacity_mw: 500,
                    });
                }
                Tool::PowerPlantGas => {
                    self.sim.plants.insert((ax, ay), super::sim::PlantState {
                        age_months: 0,
                        max_life_months: 60 * 12, // 60 years
                        capacity_mw: 800,
                    });
                }
                _ => {}
            }

            self.sim.treasury -= cost;
            self.refresh_sector_stats();
            return Ok(());
        }

        // Bounds check for single-tile tools
        if x >= self.map.width || y >= self.map.height {
            return Err("Out of bounds".to_string());
        }

        let tile = self.map.get(x, y);
        if !tool.can_place(tile) {
            return Err("Cannot place tool here".to_string());
        }

        let cost = tool.cost();
        if self.sim.treasury < cost {
            return Err("Insufficient funds!".to_string());
        }

        let new_tile = match (tool, tile) {
            (Tool::Road, Tile::PowerLine) | (Tool::PowerLine, Tile::Road) => Tile::RoadPowerLine,
            _ => match tool.target_tile() {
                Some(t) => t,
                None => return Ok(()),
            },
        };

        // If bulldoze, we should check if we're hitting any part of a 4x4 plant area
        if tool == Tool::Bulldoze {
            // This is tricky because we only have the top-left in `sim.plants`.
            // Let's do a search.
            let mut plant_to_remove = None;
            for (&(px, py), _) in self.sim.plants.iter() {
                if x >= px && x < px + 4 && y >= py && y < py + 4 {
                    plant_to_remove = Some((px, py));
                    break;
                }
            }
            if let Some(pos) = plant_to_remove {
                self.sim.plants.remove(&pos);
            }
        }

        self.map.set(x, y, new_tile);
        self.sim.treasury -= cost;
        self.refresh_sector_stats();
        Ok(())
    }

    fn refresh_sector_stats(&mut self) {
        let stats = super::sim::economy::compute_sector_stats(&self.map);
        self.sim.residential_population = stats.residential_population;
        self.sim.commercial_jobs = stats.commercial_jobs;
        self.sim.industrial_jobs = stats.industrial_jobs;
        self.sim.population = stats.residential_population;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_tax_rate_updates_only_target_sector() {
        let mut engine = SimulationEngine::new(Map::new(8, 8), SimState::default());

        engine
            .execute_command(EngineCommand::SetTaxRate {
                sector: TaxSector::Commercial,
                rate: 14,
            })
            .unwrap();

        assert_eq!(engine.sim.tax_rates.residential, 9);
        assert_eq!(engine.sim.tax_rates.commercial, 14);
        assert_eq!(engine.sim.tax_rates.industrial, 9);
    }

    #[test]
    fn set_tax_rate_clamps_to_supported_range() {
        let mut engine = SimulationEngine::new(Map::new(8, 8), SimState::default());

        engine
            .execute_command(EngineCommand::SetTaxRate {
                sector: TaxSector::Industrial,
                rate: 99,
            })
            .unwrap();

        assert_eq!(engine.sim.tax_rates.industrial, 99);
    }

    #[test]
    fn set_tax_rate_clamps_at_one_hundred_percent() {
        let mut engine = SimulationEngine::new(Map::new(8, 8), SimState::default());

        engine
            .execute_command(EngineCommand::SetTaxRate {
                sector: TaxSector::Industrial,
                rate: u8::MAX,
            })
            .unwrap();

        assert_eq!(engine.sim.tax_rates.industrial, 100);
    }
}
