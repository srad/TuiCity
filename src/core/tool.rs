use crate::core::map::Tile;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tool {
    Inspect,
    ZoneRes,
    ZoneComm,
    ZoneInd,
    Road,
    Rail,
    PowerLine,
    PowerPlantCoal,
    PowerPlantGas,
    PowerPlantPicker,
    Park,
    Police,
    Fire,
    Bulldoze,
}

impl Tool {
    pub const ALL: [Tool; 14] = [
        Tool::Inspect,
        Tool::ZoneRes,
        Tool::ZoneComm,
        Tool::ZoneInd,
        Tool::Road,
        Tool::Rail,
        Tool::PowerLine,
        Tool::PowerPlantCoal,
        Tool::PowerPlantGas,
        Tool::PowerPlantPicker,
        Tool::Park,
        Tool::Police,
        Tool::Fire,
        Tool::Bulldoze,
    ];

    pub fn cost(&self) -> i64 {
        match self {
            Tool::Inspect => 0,
            Tool::ZoneRes => 100,
            Tool::ZoneComm => 100,
            Tool::ZoneInd => 100,
            Tool::Road => 10,
            Tool::Rail => 20,
            Tool::PowerLine => 5,
            Tool::PowerPlantCoal => 3_000,
            Tool::PowerPlantGas => 6_000,
            Tool::PowerPlantPicker => 0,
            Tool::Park => 80,
            Tool::Police => 500,
            Tool::Fire => 500,
            Tool::Bulldoze => 1,
        }
    }

    pub fn target_tile(&self) -> Option<Tile> {
        match self {
            Tool::Inspect => None,
            Tool::ZoneRes => Some(Tile::ZoneRes),
            Tool::ZoneComm => Some(Tile::ZoneComm),
            Tool::ZoneInd => Some(Tile::ZoneInd),
            Tool::Road => Some(Tile::Road),
            Tool::Rail => Some(Tile::Rail),
            Tool::PowerLine => Some(Tile::PowerLine),
            Tool::PowerPlantCoal => Some(Tile::PowerPlantCoal),
            Tool::PowerPlantGas => Some(Tile::PowerPlantGas),
            Tool::PowerPlantPicker => None,
            Tool::Park => Some(Tile::Park),
            Tool::Police => Some(Tile::Police),
            Tool::Fire => Some(Tile::Fire),
            Tool::Bulldoze => Some(Tile::Grass),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tool::Inspect => "Inspect",
            Tool::ZoneRes => "Residential",
            Tool::ZoneComm => "Commercial",
            Tool::ZoneInd => "Industrial",
            Tool::Road => "Road",
            Tool::Rail => "Rail",
            Tool::PowerLine => "Power Line",
            Tool::PowerPlantCoal => "Coal Plant",
            Tool::PowerPlantGas => "Gas Plant",
            Tool::PowerPlantPicker => "Power Plant...",
            Tool::Park => "Park",
            Tool::Police => "Police",
            Tool::Fire => "Fire Dept",
            Tool::Bulldoze => "Bulldoze",
        }
    }

    pub fn key_hint(&self) -> char {
        match self {
            Tool::Inspect => '?',
            Tool::ZoneRes => '1',
            Tool::ZoneComm => '2',
            Tool::ZoneInd => '3',
            Tool::Road => 'r',
            Tool::Rail => 'l',
            Tool::PowerLine => 'p',
            Tool::PowerPlantCoal => 'e',
            Tool::PowerPlantGas => 'g',
            Tool::PowerPlantPicker => 'E',
            Tool::Park => 'k',
            Tool::Police => 's',
            Tool::Fire => 'f',
            Tool::Bulldoze => 'b',
        }
    }

    pub fn can_place(&self, tile: Tile) -> bool {
        match self {
            Tool::Bulldoze => !matches!(tile, Tile::Water),
            Tool::Inspect | Tool::PowerPlantPicker => false,
            Tool::Road => matches!(
                tile,
                Tile::Grass | Tile::Trees | Tile::Dirt | Tile::PowerLine
            ),
            Tool::PowerLine => matches!(tile, Tile::Grass | Tile::Trees | Tile::Dirt | Tile::Road),
            Tool::ZoneRes | Tool::ZoneComm | Tool::ZoneInd => matches!(
                tile,
                Tile::Grass
                    | Tile::Trees
                    | Tile::Dirt
                    | Tile::ZoneRes
                    | Tile::ZoneComm
                    | Tile::ZoneInd
            ),
            _ => matches!(tile, Tile::Grass | Tile::Trees | Tile::Dirt | Tile::Rubble),
        }
    }

    pub fn is_traversable(self, tile: Tile) -> bool {
        match self {
            Tool::Road => matches!(tile, Tile::Road | Tile::RoadPowerLine),
            Tool::PowerLine => matches!(tile, Tile::PowerLine | Tile::RoadPowerLine),
            _ => self.target_tile() == Some(tile),
        }
    }

    /// Whether this tool uses the SimCity-style drag-to-draw line mechanic.
    pub fn uses_line_drag(tool: Tool) -> bool {
        matches!(tool, Tool::Road | Tool::Rail | Tool::PowerLine)
    }

    /// Whether this tool uses the SimCity-style drag-to-select rectangle mechanic.
    pub fn uses_rect_drag(tool: Tool) -> bool {
        matches!(tool, Tool::ZoneRes | Tool::ZoneComm | Tool::ZoneInd)
    }

    /// Footprint size (width, height) in tiles.  1×1 for single-tile tools.
    pub fn footprint(&self) -> (usize, usize) {
        match self {
            Tool::PowerPlantCoal => (4, 4),
            Tool::PowerPlantGas => (4, 4),
            Tool::Police | Tool::Fire => (3, 3),
            Tool::Park => (2, 2),
            _ => (1, 1),
        }
    }

    /// Whether this tool places a multi-tile building that shows a footprint preview on hover.
    pub fn uses_footprint_preview(tool: Tool) -> bool {
        let (w, h) = tool.footprint();
        w > 1 || h > 1
    }
}
