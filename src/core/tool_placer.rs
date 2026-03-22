use super::map::{Map, TerrainTile, Tile, TransportTile, ViewLayer};
use super::sim::SimState;
use super::tool::{Tool, ToolContext};

// ── ToolPlacer ────────────────────────────────────────────────────────────────
//
// Owns all tool-placement and bulldoze logic. Extracted from SimulationEngine to
// separate placement orchestration from the simulation tick loop.

pub struct ToolPlacer<'a> {
    pub map: &'a mut Map,
    pub sim: &'a mut SimState,
}

impl<'a> ToolPlacer<'a> {
    pub fn new(map: &'a mut Map, sim: &'a mut SimState) -> Self {
        Self { map, sim }
    }

    pub fn place_tool(
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
        if self.sim.economy.treasury < cost {
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
        } else if matches!(
            tool,
            Tool::TerrainWater | Tool::TerrainLand | Tool::TerrainTrees
        ) {
            let terrain = match tool {
                Tool::TerrainWater => TerrainTile::Water,
                Tool::TerrainLand => TerrainTile::Grass,
                Tool::TerrainTrees => TerrainTile::Trees,
                _ => unreachable!(),
            };
            if terrain == TerrainTile::Water {
                self.map.set_zone_spec(x, y, None);
            }
            self.map.set_terrain(x, y, terrain);
        } else {
            let new_tile = match tool.target_tile() {
                Some(t) => t,
                None => return Ok(()),
            };
            self.map.set(x, y, new_tile);
        }

        self.sim.economy.treasury -= cost;
        self.refresh_sector_stats();
        Ok(())
    }

    pub fn place_line(
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
            if self.sim.economy.treasury < total_cost + cost_per {
                return Err("Insufficient funds!".to_string());
            }

            tiles_to_place.push((x, y));
            total_cost += cost_per;
        }

        for (x, y) in tiles_to_place {
            self.place_network_cell(tool, layer, x, y);
            self.sim.plants.remove(&(x, y));
        }
        self.sim.economy.treasury -= total_cost;
        self.refresh_sector_stats();

        Ok(())
    }

    pub fn place_rect(
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
            if self.sim.economy.treasury < total_cost + cost_per {
                return Err("Insufficient funds!".to_string());
            }
            tiles_to_place.push((x, y));
            total_cost += cost_per;
        }

        for (x, y) in tiles_to_place {
            self.map.set_zone_spec(x, y, Some(zone_spec));
            self.sim.plants.remove(&(x, y));
        }
        self.sim.economy.treasury -= total_cost;
        self.refresh_sector_stats();

        Ok(())
    }

    pub fn refresh_sector_stats(&mut self) {
        let stats = super::sim::economy::compute_sector_stats(self.map);
        self.sim.pop.residential_population = stats.residential_population;
        self.sim.pop.commercial_jobs = stats.commercial_jobs;
        self.sim.pop.industrial_jobs = stats.industrial_jobs;
        self.sim.pop.population = stats.residential_population;
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn ensure_tool_unlocked(&self, tool: Tool) -> Result<(), String> {
        let ctx = ToolContext { year: self.sim.year, unlock_mode: self.sim.economy.unlock_mode };
        if tool.is_available(&ctx) {
            Ok(())
        } else {
            Err(tool
                .unavailable_reason(&ctx)
                .unwrap_or_else(|| format!("{} not available", tool.label())))
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
            let fp = self.sim.plants.get(&(px, py)).map(|s| s.footprint as usize).unwrap_or(4);
            for dy in 0..fp {
                for dx in 0..fp {
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

        if let Some(occupant) = self.map.occupant_at(x, y) {
            if matches!(
                occupant,
                Tile::BusDepot | Tile::RailDepot | Tile::SubwayStation
            ) {
                self.sim.depots.remove(&(x, y));
            }
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
            .iter()
            .find(|(&(px, py), state)| {
                let fp = state.footprint as usize;
                x >= px && x < px + fp && y >= py && y < py + fp
            })
            .map(|(&pos, _)| pos)
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
        if self.sim.economy.treasury < cost {
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
                use super::sim::constants::{COAL_PLANT_CAPACITY_MW, COAL_PLANT_LIFE_MONTHS};
                self.sim.plants.insert(
                    (ax, ay),
                    super::sim::PlantState {
                        age_months: 0,
                        max_life_months: COAL_PLANT_LIFE_MONTHS,
                        capacity_mw: COAL_PLANT_CAPACITY_MW,
                        efficiency: 1.0,
                        footprint: 4,
                    },
                );
            }
            Tool::PowerPlantGas => {
                use super::sim::constants::{GAS_PLANT_CAPACITY_MW, GAS_PLANT_LIFE_MONTHS};
                self.sim.plants.insert(
                    (ax, ay),
                    super::sim::PlantState {
                        age_months: 0,
                        max_life_months: GAS_PLANT_LIFE_MONTHS,
                        capacity_mw: GAS_PLANT_CAPACITY_MW,
                        efficiency: 1.0,
                        footprint: 4,
                    },
                );
            }
            Tool::PowerPlantNuclear => {
                use super::sim::constants::{
                    NUCLEAR_PLANT_CAPACITY_MW, NUCLEAR_PLANT_LIFE_MONTHS,
                };
                self.sim.plants.insert(
                    (ax, ay),
                    super::sim::PlantState {
                        age_months: 0,
                        max_life_months: NUCLEAR_PLANT_LIFE_MONTHS,
                        capacity_mw: NUCLEAR_PLANT_CAPACITY_MW,
                        efficiency: 1.0,
                        footprint: 4,
                    },
                );
            }
            Tool::PowerPlantWind => {
                use super::sim::constants::{WIND_FARM_CAPACITY_MW, WIND_FARM_LIFE_MONTHS};
                self.sim.plants.insert(
                    (ax, ay),
                    super::sim::PlantState {
                        age_months: 0,
                        max_life_months: WIND_FARM_LIFE_MONTHS,
                        capacity_mw: WIND_FARM_CAPACITY_MW,
                        efficiency: 1.0,
                        footprint: 1,
                    },
                );
            }
            Tool::PowerPlantSolar => {
                use super::sim::constants::{SOLAR_PLANT_CAPACITY_MW, SOLAR_PLANT_LIFE_MONTHS};
                self.sim.plants.insert(
                    (ax, ay),
                    super::sim::PlantState {
                        age_months: 0,
                        max_life_months: SOLAR_PLANT_LIFE_MONTHS,
                        capacity_mw: SOLAR_PLANT_CAPACITY_MW,
                        efficiency: 1.0,
                        footprint: 2,
                    },
                );
            }
            Tool::BusDepot => {
                self.sim
                    .depots
                    .insert((ax, ay), super::sim::DepotState { trips_used: 0 });
            }
            _ => {}
        }

        self.sim.economy.treasury -= cost;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::map::{ZoneDensity, ZoneKind, ZoneSpec};

    fn placer<'a>(map: &'a mut Map, sim: &'a mut SimState) -> ToolPlacer<'a> {
        ToolPlacer::new(map, sim)
    }

    #[test]
    fn place_road_deducts_cost_and_sets_transport() {
        let mut map = Map::new(3, 1);
        let mut sim = SimState::default();
        sim.economy.treasury = 10_000;
        let cost = Tool::Road.cost();

        placer(&mut map, &mut sim)
            .place_tool(Tool::Road, ViewLayer::Surface, 1, 0)
            .unwrap();

        assert_eq!(map.transport_at(1, 0), Some(TransportTile::Road));
        assert_eq!(sim.economy.treasury, 10_000 - cost);
    }

    #[test]
    fn place_tool_fails_when_out_of_bounds() {
        let mut map = Map::new(3, 3);
        let mut sim = SimState::default();
        let err = placer(&mut map, &mut sim)
            .place_tool(Tool::Road, ViewLayer::Surface, 99, 99)
            .unwrap_err();
        assert_eq!(err, "Out of bounds");
    }

    #[test]
    fn place_tool_fails_on_insufficient_funds() {
        let mut map = Map::new(3, 3);
        let mut sim = SimState::default();
        sim.economy.treasury = 0;
        let err = placer(&mut map, &mut sim)
            .place_tool(Tool::Road, ViewLayer::Surface, 1, 1)
            .unwrap_err();
        assert_eq!(err, "Insufficient funds!");
    }

    #[test]
    fn bulldoze_clears_road_and_restores_zone() {
        let mut map = Map::new(3, 3);
        let mut sim = SimState::default();
        map.set_zone_spec(
            1,
            1,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        map.set_transport(1, 1, Some(TransportTile::Road));
        sim.economy.treasury = 10_000;

        placer(&mut map, &mut sim)
            .place_tool(Tool::Bulldoze, ViewLayer::Surface, 1, 1)
            .unwrap();

        assert!(map.transport_at(1, 1).is_none());
        assert_eq!(map.zone_kind(1, 1), Some(ZoneKind::Residential));
    }

    #[test]
    fn bulldoze_bare_zone_removes_zoning() {
        let mut map = Map::new(3, 3);
        let mut sim = SimState::default();
        map.set_zone_spec(
            1,
            1,
            Some(ZoneSpec {
                kind: ZoneKind::Commercial,
                density: ZoneDensity::Light,
            }),
        );
        sim.economy.treasury = 10_000;

        placer(&mut map, &mut sim)
            .place_tool(Tool::Bulldoze, ViewLayer::Surface, 1, 1)
            .unwrap();

        assert_eq!(map.zone_kind(1, 1), None);
    }

    #[test]
    fn place_line_skips_tiles_where_tool_cannot_be_placed() {
        let mut map = Map::new(5, 1);
        let mut sim = SimState::default();
        sim.economy.treasury = 100_000;
        // Place a building on tile 2 to block it.
        map.set(2, 0, Tile::ResLow);

        let path = vec![(0, 0), (1, 0), (2, 0), (3, 0)];
        placer(&mut map, &mut sim)
            .place_line(Tool::Road, ViewLayer::Surface, path)
            .unwrap();

        assert_eq!(map.transport_at(0, 0), Some(TransportTile::Road));
        assert_eq!(map.transport_at(1, 0), Some(TransportTile::Road));
        // Tile 2 was a building — road cannot be placed; tile 3 can.
        assert!(map.transport_at(2, 0).is_none());
        assert_eq!(map.transport_at(3, 0), Some(TransportTile::Road));
    }

    #[test]
    fn place_rect_zones_all_eligible_tiles() {
        let mut map = Map::new(5, 5);
        let mut sim = SimState::default();
        sim.economy.treasury = 100_000;

        let tiles = vec![(1, 1), (2, 1), (3, 1)];
        placer(&mut map, &mut sim)
            .place_rect(Tool::ZoneResLight, ViewLayer::Surface, tiles)
            .unwrap();

        assert_eq!(map.zone_kind(1, 1), Some(ZoneKind::Residential));
        assert_eq!(map.zone_kind(2, 1), Some(ZoneKind::Residential));
        assert_eq!(map.zone_kind(3, 1), Some(ZoneKind::Residential));
    }

    #[test]
    fn refresh_sector_stats_updates_sim_pop() {
        let mut map = Map::new(2, 1);
        let mut sim = SimState::default();
        map.set(0, 0, Tile::ResHigh);
        map.set(1, 0, Tile::CommHigh);

        placer(&mut map, &mut sim).refresh_sector_stats();

        assert!(sim.pop.residential_population > 0);
        assert!(sim.pop.commercial_jobs > 0);
        assert_eq!(sim.pop.population, sim.pop.residential_population);
    }
}
