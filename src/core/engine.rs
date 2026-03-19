#![allow(dead_code)]
use super::map::{Map, Tile, TransportTile, ViewLayer, ZoneSpec};
use super::sim::{SimState, TaxSector};
use super::tool::Tool;

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
        engine.refresh_sector_stats();
        engine
    }

    pub fn execute_command(&mut self, cmd: EngineCommand) -> Result<(), String> {
        match cmd {
            EngineCommand::PlaceTool { tool, layer, x, y } => self.place_tool(tool, layer, x, y),
            EngineCommand::PlaceLine { tool, layer, path } => self.place_line(tool, layer, path),
            EngineCommand::PlaceRect { tool, layer, tiles } => self.place_rect(tool, layer, tiles),
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

    fn prepare_lot_for_network(&mut self, tool: Tool, x: usize, y: usize) {
        match tool {
            // Surface transport occupies the tile in the SC2000 sense, so zoning underneath is
            // removed when the player runs a road, rail, highway, or onramp through the lot.
            Tool::Road | Tool::Highway | Tool::Onramp | Tool::Rail => {
                self.map.set_zone_spec(x, y, None);
            }
            // Power lines are the exception: they can coexist with a zone until a building
            // eventually grows in and consumes the line.
            Tool::PowerLine => {
                if let Some(zone) = self.map.effective_zone_spec(x, y) {
                    self.map.set_zone_spec(x, y, Some(zone));
                }
            }
            _ => {}
        }

        if matches!(self.map.get(x, y), Tile::Rubble) || self.map.get(x, y).is_building() {
            self.map.set_occupant(x, y, None);
        }
    }

    fn place_network_cell(&mut self, tool: Tool, layer: ViewLayer, x: usize, y: usize) {
        if layer == ViewLayer::Surface {
            self.prepare_lot_for_network(tool, x, y);
        }

        match tool {
            Tool::Road => self.map.set_transport(x, y, Some(TransportTile::Road)),
            Tool::Highway => self.map.set_transport(x, y, Some(TransportTile::Highway)),
            Tool::Onramp => self.map.set_transport(x, y, Some(TransportTile::Onramp)),
            Tool::Rail => self.map.set_transport(x, y, Some(TransportTile::Rail)),
            Tool::PowerLine => self.map.set_power_line(x, y, true),
            Tool::WaterPipe => self.map.set_water_pipe(x, y, true),
            Tool::Subway => self.map.set_subway_tunnel(x, y, true),
            _ => {}
        }
    }

    fn bulldoze_tile(&mut self, layer: ViewLayer, x: usize, y: usize) {
        if layer == ViewLayer::Underground {
            // Underground bulldozing is intentionally narrow: remove one hidden network at a
            // time without touching the visible lot above it.
            if self.map.has_subway_tunnel(x, y) {
                self.map.set_subway_tunnel(x, y, false);
            } else if self.map.has_water_pipe(x, y) {
                self.map.set_water_pipe(x, y, false);
            }
            return;
        }

        let visible = self.map.get(x, y);
        let zone = self.map.effective_zone_spec(x, y);

        if let Some((px, py)) = self.find_plant_at(x, y) {
            for dy in 0..4 {
                for dx in 0..4 {
                    let tx = px + dx;
                    let ty = py + dy;
                    if self.map.in_bounds(tx as i32, ty as i32) {
                        self.map.clear_surface_preserve_zone(tx, ty);
                    }
                }
            }
            self.sim.plants.remove(&(px, py));
            return;
        }

        let has_surface = self.map.transport_at(x, y).is_some()
            || self.map.has_power_line(x, y)
            || self.map.occupant_at(x, y).is_some();

        self.map.clear_surface_preserve_zone(x, y);

        // Bulldozing a bare zone clears zoning. Bulldozing a road/building/power line reveals
        // the underlying zone again if one still exists.
        if !has_surface && visible.is_zone() {
            self.map.set_zone_spec(x, y, None);
        } else if let Some(spec) = zone {
            self.map.set_zone_spec(x, y, Some(spec));
        }
    }

    fn find_plant_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.sim
            .plants
            .keys()
            .find(|&&(px, py)| x >= px && x < px + 4 && y >= py && y < py + 4)
            .copied()
    }

    fn ensure_tool_unlocked(&self, tool: Tool) -> Result<(), String> {
        if tool.is_unlocked(self.sim.year, self.sim.unlock_mode) {
            Ok(())
        } else {
            Err(format!(
                "{} unlocks in {}",
                tool.label(),
                tool.unlock_year()
            ))
        }
    }

    fn place_line(
        &mut self,
        tool: Tool,
        layer: ViewLayer,
        path: Vec<(usize, usize)>,
    ) -> Result<(), String> {
        self.ensure_tool_unlocked(tool)?;

        let cost_per = tool.cost();
        let mut total_cost = 0;
        let mut tiles_to_place = Vec::new();

        for (x, y) in path {
            if x >= self.map.width || y >= self.map.height {
                continue;
            }
            let existing = self.map.view_tile(layer, x, y);
            if !tool.can_place(existing) {
                continue;
            }
            if self.sim.treasury < total_cost + cost_per {
                return Err("Insufficient funds!".to_string());
            }

            tiles_to_place.push((x, y));
            total_cost += cost_per;
        }

        for (x, y) in tiles_to_place {
            self.place_network_cell(tool, layer, x, y);
            self.sim.plants.remove(&(x, y));
        }
        self.sim.treasury -= total_cost;
        self.refresh_sector_stats();

        Ok(())
    }

    fn place_rect(
        &mut self,
        tool: Tool,
        layer: ViewLayer,
        tiles: Vec<(usize, usize)>,
    ) -> Result<(), String> {
        self.ensure_tool_unlocked(tool)?;
        let zone_spec = match tool.zone_spec() {
            Some(kind) => kind,
            None => return Ok(()),
        };
        let cost_per = tool.cost();
        let mut total_cost = 0;
        let mut tiles_to_place = Vec::new();

        for (x, y) in tiles {
            if x >= self.map.width || y >= self.map.height {
                continue;
            }
            if !tool.can_place(self.map.view_tile(layer, x, y)) {
                continue;
            }
            if self.sim.treasury < total_cost + cost_per {
                return Err("Insufficient funds!".to_string());
            }
            tiles_to_place.push((x, y));
            total_cost += cost_per;
        }

        for (x, y) in tiles_to_place {
            self.map.set_zone_spec(x, y, Some(zone_spec));
            self.sim.plants.remove(&(x, y));
        }
        self.sim.treasury -= total_cost;
        self.refresh_sector_stats();

        Ok(())
    }

    fn can_place_footprint(
        &self,
        tool: Tool,
        layer: ViewLayer,
        ax: usize,
        ay: usize,
        fw: usize,
        fh: usize,
    ) -> bool {
        for dy in 0..fh {
            for dx in 0..fw {
                if !tool.can_place(self.map.view_tile(layer, ax + dx, ay + dy)) {
                    return false;
                }
            }
        }
        true
    }

    fn place_footprint_occupant(
        &mut self,
        tool: Tool,
        ax: usize,
        ay: usize,
        fw: usize,
        fh: usize,
    ) -> Result<(), String> {
        self.validate_footprint_requirements(tool, ax, ay, fw, fh)?;
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
                self.sim.plants.remove(&(ax + dx, ay + dy));
            }
        }

        match tool {
            Tool::PowerPlantCoal => {
                self.sim.plants.insert(
                    (ax, ay),
                    super::sim::PlantState {
                        age_months: 0,
                        max_life_months: 50 * 12,
                        capacity_mw: 500,
                    },
                );
            }
            Tool::PowerPlantGas => {
                self.sim.plants.insert(
                    (ax, ay),
                    super::sim::PlantState {
                        age_months: 0,
                        max_life_months: 60 * 12,
                        capacity_mw: 800,
                    },
                );
            }
            _ => {}
        }

        self.sim.treasury -= cost;
        self.refresh_sector_stats();
        Ok(())
    }

    fn validate_footprint_requirements(
        &self,
        tool: Tool,
        ax: usize,
        ay: usize,
        fw: usize,
        fh: usize,
    ) -> Result<(), String> {
        match tool {
            // Depots are only useful when they actually touch the network they are meant to
            // serve, so placement rejects disconnected stubs up front.
            Tool::BusDepot
                if !self.footprint_touches(ax, ay, fw, fh, |tile| tile.road_connects()) =>
            {
                Err("Bus depots must be adjacent to roads".to_string())
            }
            Tool::RailDepot
                if !self.footprint_touches(ax, ay, fw, fh, |tile| tile.rail_connects()) =>
            {
                Err("Rail depots must be adjacent to rails".to_string())
            }
            _ => Ok(()),
        }
    }

    fn footprint_touches(
        &self,
        ax: usize,
        ay: usize,
        fw: usize,
        fh: usize,
        pred: impl Fn(Tile) -> bool,
    ) -> bool {
        for fy in 0..fh {
            for fx in 0..fw {
                let x = ax + fx;
                let y = ay + fy;
                for (nx, ny, tile) in self.map.neighbors4(x, y) {
                    if nx >= ax && nx < ax + fw && ny >= ay && ny < ay + fh {
                        continue;
                    }
                    if pred(tile) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn place_tool(
        &mut self,
        tool: Tool,
        layer: ViewLayer,
        x: usize,
        y: usize,
    ) -> Result<(), String> {
        self.ensure_tool_unlocked(tool)?;

        let (fw, fh) = tool.footprint();

        if fw > 1 || fh > 1 {
            let ax = x
                .saturating_sub(fw / 2)
                .min(self.map.width.saturating_sub(fw));
            let ay = y
                .saturating_sub(fh / 2)
                .min(self.map.height.saturating_sub(fh));

            if !self.can_place_footprint(tool, layer, ax, ay, fw, fh) {
                return Err("Cannot place tool here".to_string());
            }

            return self.place_footprint_occupant(tool, ax, ay, fw, fh);
        }

        if x >= self.map.width || y >= self.map.height {
            return Err("Out of bounds".to_string());
        }

        let tile = self.map.view_tile(layer, x, y);
        if !tool.can_place(tile) && tool != Tool::Bulldoze {
            return Err("Cannot place tool here".to_string());
        }

        let cost = tool.cost();
        if self.sim.treasury < cost {
            return Err("Insufficient funds!".to_string());
        }

        if let Some(zone_spec) = tool.zone_spec() {
            self.map.set_zone_spec(x, y, Some(zone_spec));
        } else if matches!(
            tool,
            Tool::Road
                | Tool::Highway
                | Tool::Onramp
                | Tool::Rail
                | Tool::PowerLine
                | Tool::WaterPipe
                | Tool::Subway
        ) {
            self.place_network_cell(tool, layer, x, y);
        } else if tool == Tool::Bulldoze {
            self.bulldoze_tile(layer, x, y);
        } else {
            let new_tile = match tool.target_tile() {
                Some(t) => t,
                None => return Ok(()),
            };
            self.map.set(x, y, new_tile);
        }

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

        assert_eq!(engine.sim.tax_rates.residential, 9);
        assert_eq!(engine.sim.tax_rates.commercial, 14);
        assert_eq!(engine.sim.tax_rates.industrial, 9);
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
        engine.sim.treasury = 100_000;
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
        engine.sim.treasury = 100_000;

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
