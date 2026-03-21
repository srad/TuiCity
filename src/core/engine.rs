#![allow(dead_code)]
#[allow(unused_imports)]
use super::map::{Map, Tile, TransportTile, ViewLayer, ZoneSpec};
use super::sim::{SimState, TaxSector};
use super::tool::Tool;
use super::tool_placer::ToolPlacer;

#[derive(Debug, Clone)]
pub enum EngineCommand {
    PlaceTool {
        tool: Tool,
        layer: ViewLayer,
        x: usize,
        y: usize,
    },
    PlaceLine {
        tool: Tool,
        layer: ViewLayer,
        path: Vec<(usize, usize)>,
    },
    PlaceRect {
        tool: Tool,
        layer: ViewLayer,
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
                // Order matters: utilities feed transport, transport feeds growth, and growth
                // must settle before disasters and finance snapshot the month.
                Box::new(PowerSystem),
                Box::new(WaterSystem),
                Box::new(super::sim::transport::TransportSystem::default()),
                Box::new(PollutionSystem),
                Box::new(LandValueSystem),
                Box::new(PoliceSystem),
                Box::new(FireSystem),
                Box::new(GrowthSystem),
                Box::new(FireSpreadSystem),
                Box::new(FloodSystem),
                Box::new(TornadoSystem),
                Box::new(FinanceSystem),
                Box::new(HistorySystem),
            ],
        };
        ToolPlacer::new(&mut engine.map, &mut engine.sim).refresh_sector_stats();

        // Guard: catch accidental system reordering at development time.
        #[cfg(debug_assertions)]
        {
            const EXPECTED_ORDER: &[&str] = &[
                "Power", "Water", "Transport", "Pollution", "LandValue", "Police",
                "Fire", "Growth", "FireSpread", "Flood", "Tornado", "Finance", "History",
            ];
            let actual: Vec<&str> = engine.systems.iter().map(|s| s.name()).collect();
            assert_eq!(
                actual, EXPECTED_ORDER,
                "System execution order mismatch — update EXPECTED_ORDER or move the system"
            );
        }

        engine
    }

    pub fn execute_command(&mut self, cmd: EngineCommand) -> Result<(), String> {
        match cmd {
            EngineCommand::PlaceTool { tool, layer, x, y } => {
                ToolPlacer::new(&mut self.map, &mut self.sim).place_tool(tool, layer, x, y)
            }
            EngineCommand::PlaceLine { tool, layer, path } => {
                ToolPlacer::new(&mut self.map, &mut self.sim).place_line(tool, layer, path)
            }
            EngineCommand::PlaceRect { tool, layer, tiles } => {
                ToolPlacer::new(&mut self.map, &mut self.sim).place_rect(tool, layer, tiles)
            }
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
                self.sim.economy.tax_rates.set(sector, rate);
                Ok(())
            }
            EngineCommand::ReplaceState { map, sim } => {
                self.map = map;
                self.sim = sim;
                ToolPlacer::new(&mut self.map, &mut self.sim).refresh_sector_stats();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{ViewLayer, ZoneDensity, ZoneKind};

    #[test]
    fn set_tax_rate_updates_only_target_sector() {
        let mut engine = SimulationEngine::new(Map::new(8, 8), SimState::default());

        engine
            .execute_command(EngineCommand::SetTaxRate {
                sector: TaxSector::Commercial,
                rate: 14,
            })
            .unwrap();

        assert_eq!(engine.sim.economy.tax_rates.residential, 9);
        assert_eq!(engine.sim.economy.tax_rates.commercial, 14);
        assert_eq!(engine.sim.economy.tax_rates.industrial, 9);
    }

    #[test]
    fn underground_bulldoze_removes_pipe_without_touching_surface() {
        let mut engine = SimulationEngine::new(Map::new(4, 4), SimState::default());
        engine.map.set_zone_spec(
            1,
            1,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
        );
        engine.map.set_water_pipe(1, 1, true);

        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::Bulldoze,
                layer: ViewLayer::Underground,
                x: 1,
                y: 1,
            })
            .unwrap();

        assert!(!engine.map.has_water_pipe(1, 1));
        assert_eq!(engine.map.zone_kind(1, 1), Some(ZoneKind::Residential));
    }

    #[test]
    fn locked_tools_require_unlock_year() {
        let mut engine = SimulationEngine::new(Map::new(4, 4), SimState::default());
        engine.sim.year = 1900;

        let err = engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::Subway,
                layer: ViewLayer::Underground,
                x: 1,
                y: 1,
            })
            .unwrap_err();

        assert_eq!(err, "Subway unlocks in 1910");
    }

    #[test]
    fn bus_depot_must_touch_a_road() {
        let mut engine = SimulationEngine::new(Map::new(6, 6), SimState::default());
        engine.sim.economy.treasury = 100_000;
        engine.sim.year = 1920;

        let err = engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::BusDepot,
                layer: ViewLayer::Surface,
                x: 2,
                y: 2,
            })
            .unwrap_err();

        assert_eq!(err, "Bus depots must be adjacent to roads");

        engine.map.set_transport(0, 2, Some(TransportTile::Road));
        assert!(engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::BusDepot,
                layer: ViewLayer::Surface,
                x: 2,
                y: 2,
            })
            .is_ok());
    }

    #[test]
    fn rail_depot_must_touch_a_rail() {
        let mut engine = SimulationEngine::new(Map::new(6, 6), SimState::default());
        engine.sim.economy.treasury = 100_000;

        let err = engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::RailDepot,
                layer: ViewLayer::Surface,
                x: 2,
                y: 2,
            })
            .unwrap_err();

        assert_eq!(err, "Rail depots must be adjacent to rails");

        engine.map.set_transport(0, 2, Some(TransportTile::Rail));
        assert!(engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::RailDepot,
                layer: ViewLayer::Surface,
                x: 2,
                y: 2,
            })
            .is_ok());
    }
}
