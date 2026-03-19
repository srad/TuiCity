use crate::core::engine::{EngineCommand, SimulationEngine};
use crate::core::map::{Map, Tile};
use crate::core::sim::SimState;
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
        let cost = Tool::ZoneRes.cost();

        let tiles = vec![(2, 2), (2, 3), (3, 2), (3, 3)];
        let cmd = EngineCommand::PlaceRect {
            tool: Tool::ZoneRes,
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
            x: 5,
            y: 5,
        };
        engine.execute_command(cmd1).unwrap();

        // Place a powerline over it
        let cmd2 = EngineCommand::PlaceTool {
            tool: Tool::PowerLine,
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
    fn test_power_plant_picker_not_placeable() {
        let mut engine = setup_engine();
        let cmd = EngineCommand::PlaceTool {
            tool: Tool::PowerPlantPicker,
            x: 5,
            y: 5,
        };
        let result = engine.execute_command(cmd);
        assert!(
            result.is_err(),
            "PowerPlantPicker is a UI trigger and should not be placeable on the map"
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
}
