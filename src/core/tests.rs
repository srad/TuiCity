use crate::core::engine::{EngineCommand, SimulationEngine};
use crate::core::map::{Map, Tile, ViewLayer, ZoneDensity, ZoneKind, ZoneSpec};
use crate::core::sim::{PlantState, SimState};
use crate::core::tool::Tool;

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_engine() -> SimulationEngine {
        let map = Map::new(10, 10);
        let mut sim = SimState::default();
        sim.treasury = 100000; // Give enough money for tests
        SimulationEngine::new(map, sim)
    }

    #[test]
    fn test_place_single_tool() {
        let mut engine = setup_engine();
        let initial_funds = engine.sim.treasury;
        let cost = Tool::Road.cost();

        // Place a road
        let cmd = EngineCommand::PlaceTool {
            tool: Tool::Road,
            layer: ViewLayer::Surface,
            x: 5,
            y: 5,
        };
        let result = engine.execute_command(cmd);

        assert!(result.is_ok(), "Tool placement should succeed");
        assert_eq!(
            engine.map.get(5, 5),
            Tile::Road,
            "Tile should be updated to Road"
        );
        assert_eq!(
            engine.sim.treasury,
            initial_funds - cost,
            "Treasury should be deducted by tool cost"
        );
    }

    #[test]
    fn test_place_tool_out_of_bounds() {
        let mut engine = setup_engine();
        let initial_funds = engine.sim.treasury;

        // Try to place a road outside the map (map is 10x10)
        let cmd = EngineCommand::PlaceTool {
            tool: Tool::Road,
            layer: ViewLayer::Surface,
            x: 10,
            y: 10,
        };
        let result = engine.execute_command(cmd);

        assert!(result.is_err(), "Tool placement out of bounds should fail");
        assert_eq!(
            engine.sim.treasury, initial_funds,
            "Treasury should not change on failure"
        );
    }

    #[test]
    fn test_insufficient_funds() {
        let mut engine = setup_engine();
        // Set treasury lower than a coal plant's cost
        engine.sim.treasury = 10;
        let cost = Tool::PowerPlantCoal.cost();
        assert!(cost > 10, "PowerPlant cost must be > 10 for this test");

        let cmd = EngineCommand::PlaceTool {
            tool: Tool::PowerPlantCoal,
            layer: ViewLayer::Surface,
            x: 5,
            y: 5,
        };
        let result = engine.execute_command(cmd);

        assert!(
            result.is_err(),
            "Tool placement should fail due to insufficient funds"
        );
        assert_eq!(result.unwrap_err(), "Insufficient funds!");
        assert_eq!(
            engine.sim.treasury, 10,
            "Treasury should not change on failure"
        );
    }

    #[test]
    fn test_place_line() {
        let mut engine = setup_engine();
        let initial_funds = engine.sim.treasury;
        let cost = Tool::Road.cost();

        // Path of 3 tiles
        let path = vec![(1, 1), (1, 2), (1, 3)];
        let cmd = EngineCommand::PlaceLine {
            tool: Tool::Road,
            layer: ViewLayer::Surface,
            path: path.clone(),
        };
        let result = engine.execute_command(cmd);

        assert!(result.is_ok(), "Line placement should succeed");
        assert_eq!(engine.map.get(1, 1), Tile::Road);
        assert_eq!(engine.map.get(1, 2), Tile::Road);
        assert_eq!(engine.map.get(1, 3), Tile::Road);
        assert_eq!(
            engine.sim.treasury,
            initial_funds - (cost * 3),
            "Treasury should be deducted for 3 tiles"
        );
    }

    #[test]
    fn test_place_line_with_obstacle() {
        let mut engine = setup_engine();

        // Place water which blocks roads by default
        engine.map.set(1, 2, Tile::Water);

        let path = vec![(1, 1), (1, 2), (1, 3)];
        let cmd = EngineCommand::PlaceLine {
            tool: Tool::Road,
            layer: ViewLayer::Surface,
            path: path.clone(),
        };

        // The implementation skips invalid tiles. Let's see what it does:
        // By design in this project, line drag filters out invalid tiles *before* committing,
        // but `place_line` iterates and skips `!tool.can_place(existing)`.
        let result = engine.execute_command(cmd);

        assert!(
            result.is_ok(),
            "Line placement should succeed (skipping invalid tiles)"
        );
        assert_eq!(
            engine.map.get(1, 1),
            Tile::Road,
            "Valid tile should be placed"
        );
        assert_eq!(
            engine.map.get(1, 2),
            Tile::Water,
            "Water should remain blocking"
        );
        assert_eq!(
            engine.map.get(1, 3),
            Tile::Road,
            "Valid tile should be placed"
        );
    }

    #[test]
    fn test_place_rect() {
        let mut engine = setup_engine();
        let initial_funds = engine.sim.treasury;
        let cost = Tool::ZoneResLight.cost();

        let tiles = vec![(2, 2), (2, 3), (3, 2), (3, 3)];
        let cmd = EngineCommand::PlaceRect {
            tool: Tool::ZoneResLight,
            layer: ViewLayer::Surface,
            tiles,
        };
        let result = engine.execute_command(cmd);

        assert!(result.is_ok(), "Rect placement should succeed");
        assert_eq!(engine.map.get(2, 2), Tile::ZoneRes);
        assert_eq!(engine.map.get(3, 3), Tile::ZoneRes);
        assert_eq!(
            engine.sim.treasury,
            initial_funds - (cost * 4),
            "Treasury should be deducted for 4 tiles"
        );
    }

    #[test]
    fn test_road_powerline_intersection() {
        let mut engine = setup_engine();

        // Place a road
        let cmd1 = EngineCommand::PlaceTool {
            tool: Tool::Road,
            layer: ViewLayer::Surface,
            x: 5,
            y: 5,
        };
        engine.execute_command(cmd1).unwrap();

        // Place a powerline over it
        let cmd2 = EngineCommand::PlaceTool {
            tool: Tool::PowerLine,
            layer: ViewLayer::Surface,
            x: 5,
            y: 5,
        };
        engine.execute_command(cmd2).unwrap();

        assert_eq!(
            engine.map.get(5, 5),
            Tile::RoadPowerLine,
            "Intersection should become RoadPowerLine"
        );
    }

    #[test]
    fn test_set_city_name() {
        let mut engine = setup_engine();
        let cmd = EngineCommand::SetCityName("Testville".to_string());
        let result = engine.execute_command(cmd);

        assert!(result.is_ok());
        assert_eq!(engine.sim.city_name, "Testville");
    }

    #[test]
    fn test_zoning_over_road_preserves_road_surface() {
        let mut engine = setup_engine();

        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::Road,
                layer: ViewLayer::Surface,
                x: 4,
                y: 4,
            })
            .unwrap();

        engine
            .execute_command(EngineCommand::PlaceRect {
                tool: Tool::ZoneResLight,
                layer: ViewLayer::Surface,
                tiles: vec![(4, 4)],
            })
            .unwrap();

        assert_eq!(engine.map.get(4, 4), Tile::Road);
        assert_eq!(engine.map.zone_kind(4, 4), Some(ZoneKind::Residential));
    }

    #[test]
    fn test_road_through_zone_dezones_land_when_bulldozed() {
        let mut engine = setup_engine();

        engine
            .execute_command(EngineCommand::PlaceRect {
                tool: Tool::ZoneResLight,
                layer: ViewLayer::Surface,
                tiles: vec![(3, 3)],
            })
            .unwrap();
        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::Road,
                layer: ViewLayer::Surface,
                x: 3,
                y: 3,
            })
            .unwrap();

        assert_eq!(engine.map.get(3, 3), Tile::Road);
        assert_eq!(engine.map.zone_kind(3, 3), None);

        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::Bulldoze,
                layer: ViewLayer::Surface,
                x: 3,
                y: 3,
            })
            .unwrap();

        assert_eq!(engine.map.get(3, 3), Tile::Grass);
        assert_eq!(engine.map.zone_kind(3, 3), None);
    }

    #[test]
    fn test_single_tile_building_replaces_power_line() {
        let mut engine = setup_engine();

        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::PowerLine,
                layer: ViewLayer::Surface,
                x: 2,
                y: 2,
            })
            .unwrap();
        let treasury_after_line = engine.sim.treasury;

        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::WaterPump,
                layer: ViewLayer::Surface,
                x: 2,
                y: 2,
            })
            .unwrap();

        assert_eq!(engine.map.get(2, 2), Tile::WaterPump);
        assert!(!engine.map.has_power_line(2, 2));
        assert_eq!(
            engine.sim.treasury,
            treasury_after_line - Tool::WaterPump.cost()
        );
    }

    #[test]
    fn test_footprint_building_replaces_power_lines() {
        let mut engine = setup_engine();

        for (x, y) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
            engine
                .execute_command(EngineCommand::PlaceTool {
                    tool: Tool::PowerLine,
                    layer: ViewLayer::Surface,
                    x,
                    y,
                })
                .unwrap();
        }
        let treasury_after_lines = engine.sim.treasury;

        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::Park,
                layer: ViewLayer::Surface,
                x: 2,
                y: 2,
            })
            .unwrap();

        for (x, y) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
            assert_eq!(engine.map.get(x, y), Tile::Park);
            assert!(!engine.map.has_power_line(x, y));
        }
        assert_eq!(
            engine.sim.treasury,
            treasury_after_lines - Tool::Park.cost()
        );
    }

    #[test]
    fn test_building_cannot_replace_road_powerline() {
        let mut engine = setup_engine();

        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::Road,
                layer: ViewLayer::Surface,
                x: 4,
                y: 4,
            })
            .unwrap();
        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::PowerLine,
                layer: ViewLayer::Surface,
                x: 4,
                y: 4,
            })
            .unwrap();

        let err = engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::WaterPump,
                layer: ViewLayer::Surface,
                x: 4,
                y: 4,
            })
            .unwrap_err();

        assert_eq!(err, "Cannot place tool here");
        assert_eq!(engine.map.get(4, 4), Tile::RoadPowerLine);
        assert!(engine.map.has_power_line(4, 4));
    }

    #[test]
    fn test_powerline_through_zone_preserves_underlying_zone() {
        let mut engine = setup_engine();

        engine
            .execute_command(EngineCommand::PlaceRect {
                tool: Tool::ZoneResDense,
                layer: ViewLayer::Surface,
                tiles: vec![(3, 3)],
            })
            .unwrap();
        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::PowerLine,
                layer: ViewLayer::Surface,
                x: 3,
                y: 3,
            })
            .unwrap();

        assert_eq!(engine.map.get(3, 3), Tile::PowerLine);
        assert_eq!(engine.map.zone_kind(3, 3), Some(ZoneKind::Residential));
    }

    #[test]
    fn test_building_over_powerline_clears_underlying_zone() {
        let mut engine = setup_engine();

        engine
            .execute_command(EngineCommand::PlaceRect {
                tool: Tool::ZoneResDense,
                layer: ViewLayer::Surface,
                tiles: vec![(3, 3)],
            })
            .unwrap();
        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::PowerLine,
                layer: ViewLayer::Surface,
                x: 3,
                y: 3,
            })
            .unwrap();
        engine
            .execute_command(EngineCommand::PlaceTool {
                tool: Tool::WaterPump,
                layer: ViewLayer::Surface,
                x: 3,
                y: 3,
            })
            .unwrap();

        assert_eq!(engine.map.get(3, 3), Tile::WaterPump);
        assert!(!engine.map.has_power_line(3, 3));
        assert_eq!(engine.map.zone_kind(3, 3), None);
    }

    #[test]
    fn test_advance_month_builds_house_over_powerline_on_zone() {
        let mut engine = setup_engine();

        engine.map.set(0, 0, Tile::PowerPlantCoal);
        engine.sim.plants.insert(
            (0, 0),
            PlantState {
                age_months: 0,
                max_life_months: 600,
                capacity_mw: 500,
            },
        );
        engine.map.set(1, 0, Tile::PowerLine);
        engine.map.set_zone_spec(
            2,
            0,
            Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
        );
        engine.map.set_power_line(2, 0, true);
        engine.map.set(3, 0, Tile::Road);
        engine.sim.demand_res = 1.0;

        for _ in 0..6 {
            engine.execute_command(EngineCommand::AdvanceMonth).unwrap();
            if engine.map.get(2, 0) == Tile::ResLow {
                break;
            }
        }

        assert_eq!(engine.map.get(2, 0), Tile::ResLow);
        assert!(!engine.map.has_power_line(2, 0));
    }
}
