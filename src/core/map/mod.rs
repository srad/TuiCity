pub mod gen;
pub mod tile;

pub use tile::{
    TerrainTile, Tile, TileOverlay, TransportTile, TripFailure, TripMode, UndergroundTile,
    ZoneDensity, ZoneKind, ZoneSpec,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ViewLayer {
    #[default]
    Surface,
    Underground,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Map {
    // `tiles` is the rendered/legacy-facing view. The remaining vectors are the authoritative
    // layered state and are recomposed into `tiles` after edits and during save migration.
    pub tiles: Vec<Tile>,
    #[serde(default)]
    pub terrain: Vec<TerrainTile>,
    #[serde(default)]
    pub transport: Vec<Option<TransportTile>>,
    #[serde(default)]
    pub power_lines: Vec<bool>,
    #[serde(default)]
    pub underground: Vec<Option<UndergroundTile>>,
    #[serde(default)]
    pub occupants: Vec<Option<Tile>>,
    #[serde(default)]
    pub zones: Vec<Option<ZoneKind>>,
    #[serde(default)]
    pub zone_densities: Vec<Option<ZoneDensity>>,
    pub overlays: Vec<TileOverlay>,
    pub width: usize,
    pub height: usize,
}

impl Map {
    pub fn new(w: usize, h: usize) -> Self {
        let len = w * h;
        Self {
            tiles: vec![Tile::default(); len],
            terrain: vec![TerrainTile::default(); len],
            transport: vec![None; len],
            power_lines: vec![false; len],
            underground: vec![None; len],
            occupants: vec![None; len],
            zones: vec![None; len],
            zone_densities: vec![None; len],
            overlays: vec![TileOverlay::default(); len],
            width: w,
            height: h,
        }
    }

    pub fn normalize_layers(&mut self) {
        let len = self.width * self.height;
        if self.terrain.len() == len
            && self.transport.len() == len
            && self.power_lines.len() == len
            && self.underground.len() == len
            && self.occupants.len() == len
            && self.zones.len() == len
            && self.zone_densities.len() == len
        {
            for idx in 0..len {
                // Older layered saves may have zone kinds without explicit density. Recover the
                // intended density from the visible tile when possible and keep overlays aligned.
                self.zone_densities[idx] = self.zone_densities[idx].or_else(|| {
                    self.tiles
                        .get(idx)
                        .copied()
                        .and_then(Tile::inferred_zone_density)
                });
                if let Some(zone) = self.zones[idx] {
                    self.overlays[idx].zone = Some(zone);
                }
            }
            self.rebuild_all_tiles();
            return;
        }

        let legacy_tiles = self.tiles.clone();
        // Legacy saves only knew about a single visible tile. Rehydrate that flat format into
        // the layered model so newer systems can reason about overlapping infrastructure.
        self.terrain = vec![TerrainTile::default(); len];
        self.transport = vec![None; len];
        self.power_lines = vec![false; len];
        self.underground = vec![None; len];
        self.occupants = vec![None; len];
        self.zones = vec![None; len];
        self.zone_densities = vec![None; len];

        for (idx, tile) in legacy_tiles.into_iter().enumerate().take(len) {
            self.import_legacy_tile(idx, tile);
        }
        self.rebuild_all_tiles();
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn get(&self, x: usize, y: usize) -> Tile {
        self.tiles[self.idx(x, y)]
    }

    pub fn view_tile(&self, layer: ViewLayer, x: usize, y: usize) -> Tile {
        match layer {
            ViewLayer::Surface => self.get(x, y),
            ViewLayer::Underground => {
                let underground = self.underground_at(x, y);
                if underground.subway {
                    Tile::SubwayTunnel
                } else if underground.water_pipe {
                    Tile::WaterPipe
                } else if self.occupant_at(x, y) == Some(Tile::SubwayStation) {
                    Tile::SubwayStation
                } else {
                    Tile::Dirt
                }
            }
        }
    }

    pub fn set(&mut self, x: usize, y: usize, tile: Tile) {
        let idx = self.idx(x, y);
        let previous_zone = self.effective_zone_spec(x, y);
        // `set` behaves like importing a full visible tile. It resets the surface layers while
        // leaving underground infrastructure intact, then rebuilds the composite tile.
        self.clear_surface_layers_at(idx);
        self.import_legacy_tile(idx, tile);
        if let Some(previous_zone) = previous_zone {
            if previous_zone.density == ZoneDensity::Dense
                && ZoneKind::from_tile(tile) == Some(previous_zone.kind)
            {
                self.zones[idx] = Some(previous_zone.kind);
                self.zone_densities[idx] = Some(previous_zone.density);
                self.overlays[idx].zone = Some(previous_zone.kind);
            }
        }
        self.rebuild_cell(idx);
    }

    pub fn get_overlay(&self, x: usize, y: usize) -> TileOverlay {
        self.overlays[self.idx(x, y)]
    }

    #[allow(dead_code)]
    pub fn set_overlay(&mut self, x: usize, y: usize, overlay: TileOverlay) {
        let idx = self.idx(x, y);
        self.overlays[idx] = overlay;
    }

    pub(crate) fn reset_service_overlays(&mut self) {
        for overlay in &mut self.overlays {
            overlay.power_level = 0;
            overlay.water_service = 0;
            overlay.trip_success = false;
            overlay.trip_mode = None;
            overlay.trip_failure = None;
            overlay.trip_cost = 0;
        }
    }

    #[allow(dead_code)]
    pub fn terrain_at(&self, x: usize, y: usize) -> TerrainTile {
        self.terrain[self.idx(x, y)]
    }

    pub fn set_terrain(&mut self, x: usize, y: usize, terrain: TerrainTile) {
        let idx = self.idx(x, y);
        self.terrain[idx] = terrain;
        self.rebuild_cell(idx);
    }

    pub fn transport_at(&self, x: usize, y: usize) -> Option<TransportTile> {
        self.transport[self.idx(x, y)]
    }

    pub fn set_transport(&mut self, x: usize, y: usize, transport: Option<TransportTile>) {
        let idx = self.idx(x, y);
        self.transport[idx] = transport;
        self.rebuild_cell(idx);
    }

    pub fn has_power_line(&self, x: usize, y: usize) -> bool {
        self.power_lines[self.idx(x, y)]
    }

    pub fn set_power_line(&mut self, x: usize, y: usize, present: bool) {
        let idx = self.idx(x, y);
        self.power_lines[idx] = present;
        self.rebuild_cell(idx);
    }

    pub fn underground_at(&self, x: usize, y: usize) -> UndergroundTile {
        self.underground[self.idx(x, y)].unwrap_or_default()
    }

    pub fn set_underground(&mut self, x: usize, y: usize, underground: UndergroundTile) {
        let idx = self.idx(x, y);
        self.underground[idx] = (!underground.is_empty()).then_some(underground);
    }

    pub fn has_water_pipe(&self, x: usize, y: usize) -> bool {
        self.underground_at(x, y).water_pipe
    }

    pub fn set_water_pipe(&mut self, x: usize, y: usize, present: bool) {
        let mut underground = self.underground_at(x, y);
        underground.water_pipe = present;
        self.set_underground(x, y, underground);
    }

    pub fn has_subway_tunnel(&self, x: usize, y: usize) -> bool {
        self.underground_at(x, y).subway
    }

    pub fn set_subway_tunnel(&mut self, x: usize, y: usize, present: bool) {
        let mut underground = self.underground_at(x, y);
        underground.subway = present;
        self.set_underground(x, y, underground);
    }

    pub fn occupant_at(&self, x: usize, y: usize) -> Option<Tile> {
        self.occupants[self.idx(x, y)]
    }

    pub fn set_occupant(&mut self, x: usize, y: usize, occupant: Option<Tile>) {
        let idx = self.idx(x, y);
        self.occupants[idx] = occupant;
        if let Some(tile) = occupant {
            if let Some(zone) = ZoneKind::from_tile(tile) {
                self.zones[idx] = Some(zone);
                self.zone_densities[idx] =
                    self.zone_densities[idx].or_else(|| tile.inferred_zone_density());
                self.overlays[idx].zone = Some(zone);
            }
        }
        self.rebuild_cell(idx);
    }

    #[allow(dead_code)]
    pub fn zone_kind(&self, x: usize, y: usize) -> Option<ZoneKind> {
        self.zones[self.idx(x, y)]
    }

    pub fn zone_density(&self, x: usize, y: usize) -> Option<ZoneDensity> {
        self.zone_densities[self.idx(x, y)]
    }

    #[allow(dead_code)]
    pub fn zone_spec(&self, x: usize, y: usize) -> Option<ZoneSpec> {
        let idx = self.idx(x, y);
        Some(ZoneSpec {
            kind: self.zones[idx]?,
            density: self.zone_densities[idx].unwrap_or(ZoneDensity::Light),
        })
    }

    pub fn set_zone_spec(&mut self, x: usize, y: usize, zone: Option<ZoneSpec>) {
        let idx = self.idx(x, y);
        self.zones[idx] = zone.map(|zone| zone.kind);
        self.zone_densities[idx] = zone.map(|zone| zone.density);
        self.overlays[idx].zone = zone.map(|zone| zone.kind);
        self.rebuild_cell(idx);
    }

    #[allow(dead_code)]
    pub fn set_zone(&mut self, x: usize, y: usize, zone: Option<ZoneKind>) {
        let density = zone.map(|_| ZoneDensity::Light);
        self.set_zone_spec(
            x,
            y,
            zone.map(|kind| ZoneSpec {
                kind,
                density: density.expect("zone should have density"),
            }),
        );
    }

    pub fn clear_surface_preserve_zone(&mut self, x: usize, y: usize) {
        let idx = self.idx(x, y);
        // Used by bulldozing and growth transitions when the zone designation should survive
        // but the current surface occupant/infrastructure should not.
        self.transport[idx] = None;
        self.power_lines[idx] = false;
        self.occupants[idx] = None;
        self.rebuild_cell(idx);
    }

    pub fn effective_zone_kind(&self, x: usize, y: usize) -> Option<ZoneKind> {
        let idx = self.idx(x, y);
        self.zones[idx]
            .or(self.overlays[idx].zone)
            .or_else(|| ZoneKind::from_tile(self.tiles[idx]))
    }

    pub fn effective_zone_spec(&self, x: usize, y: usize) -> Option<ZoneSpec> {
        let idx = self.idx(x, y);
        // Zoning can be preserved underneath power lines, roads, and developed buildings, so
        // callers should use the effective zone instead of reading only the visible tile.
        let kind = self.effective_zone_kind(x, y)?;
        let density = self.zone_densities[idx]
            .or_else(|| self.tiles[idx].inferred_zone_density())
            .unwrap_or(ZoneDensity::Light);
        Some(ZoneSpec { kind, density })
    }

    pub fn surface_lot_tile(&self, x: usize, y: usize) -> Tile {
        match (
            self.occupant_at(x, y),
            self.effective_zone_spec(x, y),
            self.transport_at(x, y),
        ) {
            (Some(occupant), _, _) => occupant,
            // Empty zones stay developable while only a power line overlays them. Surface
            // transport still suppresses the lot because roads/rails occupy the tile.
            (None, Some(zone), None) => zone.empty_tile(),
            _ => self.get(x, y),
        }
    }

    pub fn rebuild_all_tiles(&mut self) {
        for idx in 0..self.tiles.len() {
            self.rebuild_cell(idx);
        }
    }

    pub fn neighbors4(&self, x: usize, y: usize) -> Vec<(usize, usize, Tile)> {
        let mut result = Vec::new();
        let ix = x as i32;
        let iy = y as i32;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = ix + dx;
            let ny = iy + dy;
            if self.in_bounds(nx, ny) {
                result.push((nx as usize, ny as usize, self.get(nx as usize, ny as usize)));
            }
        }
        result
    }

    #[allow(dead_code)]
    pub fn update_power_grid(&mut self) {}

    fn clear_surface_layers_at(&mut self, idx: usize) {
        self.terrain[idx] = TerrainTile::Grass;
        self.transport[idx] = None;
        self.power_lines[idx] = false;
        self.occupants[idx] = None;
        self.zones[idx] = None;
        self.zone_densities[idx] = None;
        self.overlays[idx].zone = None;
    }

    fn import_legacy_tile(&mut self, idx: usize, tile: Tile) {
        // This is the only place that translates a flat visible tile into the layered model.
        // Keep the mapping conservative so old saves remain stable as the simulation evolves.
        match tile {
            Tile::Grass => self.terrain[idx] = TerrainTile::Grass,
            Tile::Water => self.terrain[idx] = TerrainTile::Water,
            Tile::Trees => self.terrain[idx] = TerrainTile::Trees,
            Tile::Dirt => self.terrain[idx] = TerrainTile::Dirt,
            Tile::Road => self.transport[idx] = Some(TransportTile::Road),
            Tile::Rail => self.transport[idx] = Some(TransportTile::Rail),
            Tile::PowerLine => self.power_lines[idx] = true,
            Tile::RoadPowerLine => {
                self.transport[idx] = Some(TransportTile::Road);
                self.power_lines[idx] = true;
            }
            Tile::Highway => self.transport[idx] = Some(TransportTile::Highway),
            Tile::Onramp => self.transport[idx] = Some(TransportTile::Onramp),
            Tile::WaterPipe => {
                self.underground[idx] = Some(UndergroundTile {
                    water_pipe: true,
                    subway: false,
                })
            }
            Tile::SubwayTunnel => {
                self.underground[idx] = Some(UndergroundTile {
                    water_pipe: false,
                    subway: true,
                })
            }
            Tile::ZoneRes => {
                self.zones[idx] = Some(ZoneKind::Residential);
                self.zone_densities[idx] = Some(ZoneDensity::Light);
                self.overlays[idx].zone = Some(ZoneKind::Residential);
            }
            Tile::ZoneComm => {
                self.zones[idx] = Some(ZoneKind::Commercial);
                self.zone_densities[idx] = Some(ZoneDensity::Light);
                self.overlays[idx].zone = Some(ZoneKind::Commercial);
            }
            Tile::ZoneInd => {
                self.zones[idx] = Some(ZoneKind::Industrial);
                self.zone_densities[idx] = Some(ZoneDensity::Light);
                self.overlays[idx].zone = Some(ZoneKind::Industrial);
            }
            other => {
                self.occupants[idx] = Some(other);
                if let Some(zone) = ZoneKind::from_tile(other) {
                    self.zones[idx] = Some(zone);
                    self.zone_densities[idx] = other.inferred_zone_density();
                    self.overlays[idx].zone = Some(zone);
                }
            }
        }
    }

    fn rebuild_cell(&mut self, idx: usize) {
        // Surface occupants win, then transport/power overlays, then bare zoning, then terrain.
        // Underground state is rendered separately through `view_tile(ViewLayer::Underground, ...)`.
        self.tiles[idx] = if let Some(occupant) = self.occupants[idx] {
            occupant
        } else if let Some(transport) = self.transport[idx] {
            transport.to_tile(self.power_lines[idx])
        } else if self.power_lines[idx] {
            Tile::PowerLine
        } else if let Some(zone) = self.zones[idx] {
            zone.empty_tile()
        } else {
            self.terrain[idx].to_tile()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_spread_to_zones() {
        let mut map = Map::new(5, 5);
        map.set(0, 0, Tile::PowerPlantCoal);
        map.set(1, 0, Tile::PowerLine);
        map.set(2, 0, Tile::ZoneRes);

        use crate::core::sim::system::SimSystem;
        use crate::core::sim::systems::PowerSystem;
        use crate::core::sim::{PlantState, SimState};

        let mut sim = SimState::default();
        sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
                efficiency: 1.0,
            footprint: 4,
            },
        );

        PowerSystem.tick(&mut map, &mut sim);

        assert!(map.get_overlay(0, 0).is_powered());
        assert!(map.get_overlay(1, 0).is_powered());
        assert!(map.get_overlay(2, 0).is_powered());
    }

    #[test]
    fn underground_view_prefers_subway_then_pipe() {
        let mut map = Map::new(2, 1);
        map.set_water_pipe(0, 0, true);
        map.set_subway_tunnel(0, 0, true);
        map.set_water_pipe(1, 0, true);

        assert_eq!(
            map.view_tile(ViewLayer::Underground, 0, 0),
            Tile::SubwayTunnel
        );
        assert_eq!(map.view_tile(ViewLayer::Underground, 1, 0), Tile::WaterPipe);
    }

    #[test]
    fn set_zone_defaults_to_light_density() {
        let mut map = Map::new(1, 1);
        map.set_zone(0, 0, Some(ZoneKind::Residential));

        assert_eq!(map.zone_density(0, 0), Some(ZoneDensity::Light));
    }

    #[test]
    fn set_preserves_underground_layers() {
        let mut map = Map::new(1, 1);
        map.set_water_pipe(0, 0, true);
        map.set_subway_tunnel(0, 0, true);

        map.set(0, 0, Tile::ResLow);

        assert!(map.has_water_pipe(0, 0));
        assert!(map.has_subway_tunnel(0, 0));
        assert_eq!(map.get(0, 0), Tile::ResLow);
    }

    #[test]
    fn surface_lot_tile_treats_powerline_over_zone_as_zone() {
        let mut map = Map::new(1, 1);
        map.set_zone_spec(
            0,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
        );
        map.set_power_line(0, 0, true);

        assert_eq!(map.get(0, 0), Tile::PowerLine);
        assert_eq!(map.surface_lot_tile(0, 0), Tile::ZoneRes);
    }

    #[test]
    fn reset_service_overlays_clears_service_fields() {
        let mut map = Map::new(3, 3);
        let idx = map.idx(1, 1);
        map.overlays[idx].power_level = 200;
        map.overlays[idx].water_service = 150;
        map.overlays[idx].trip_success = true;
        map.overlays[idx].trip_cost = 50;
        map.overlays[idx].pollution = 100;
        map.overlays[idx].crime = 80;
        map.overlays[idx].fire_risk = 60;
        map.overlays[idx].neglected_months = 10;
        map.overlays[idx].on_fire = true;

        map.reset_service_overlays();

        assert_eq!(
            map.overlays[idx].power_level, 0,
            "power_level should be cleared"
        );
        assert_eq!(
            map.overlays[idx].water_service, 0,
            "water_service should be cleared"
        );
        assert!(
            !map.overlays[idx].trip_success,
            "trip_success should be cleared"
        );
        assert_eq!(
            map.overlays[idx].trip_mode, None,
            "trip_mode should be None"
        );
        assert_eq!(
            map.overlays[idx].trip_failure, None,
            "trip_failure should be None"
        );
        assert_eq!(
            map.overlays[idx].trip_cost, 0,
            "trip_cost should be cleared"
        );
    }

    #[test]
    fn reset_service_overlays_does_not_clear_pollution_crime_fire_risk() {
        let mut map = Map::new(3, 3);
        let idx = map.idx(1, 1);
        map.overlays[idx].pollution = 100;
        map.overlays[idx].crime = 80;
        map.overlays[idx].fire_risk = 60;
        map.overlays[idx].neglected_months = 10;
        map.overlays[idx].on_fire = true;

        map.reset_service_overlays();

        assert_eq!(
            map.overlays[idx].pollution, 100,
            "pollution must NOT be cleared"
        );
        assert_eq!(map.overlays[idx].crime, 80, "crime must NOT be cleared");
        assert_eq!(
            map.overlays[idx].fire_risk, 60,
            "fire_risk must NOT be cleared"
        );
        assert_eq!(
            map.overlays[idx].neglected_months, 10,
            "neglected_months must NOT be cleared"
        );
        assert!(map.overlays[idx].on_fire, "on_fire must NOT be cleared");
    }
}
