use crate::core::{
    map::{Tile, ZoneDensity, ZoneKind, ZoneSpec},
    sim::UnlockMode,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tool {
    Inspect,
    ZoneResLight,
    ZoneResDense,
    ZoneCommLight,
    ZoneCommDense,
    ZoneIndLight,
    ZoneIndDense,
    Road,
    Highway,
    Onramp,
    Rail,
    PowerLine,
    WaterPipe,
    Subway,
    PowerPlantCoal,
    PowerPlantGas,
    BusDepot,
    RailDepot,
    SubwayStation,
    WaterPump,
    WaterTower,
    WaterTreatment,
    Desalination,
    Park,
    Police,
    Fire,
    Bulldoze,
}

impl Tool {
    #[allow(dead_code)]
    pub const ALL: [Tool; 27] = [
        Tool::Inspect,
        Tool::ZoneResLight,
        Tool::ZoneResDense,
        Tool::ZoneCommLight,
        Tool::ZoneCommDense,
        Tool::ZoneIndLight,
        Tool::ZoneIndDense,
        Tool::Road,
        Tool::Highway,
        Tool::Onramp,
        Tool::Rail,
        Tool::PowerLine,
        Tool::WaterPipe,
        Tool::Subway,
        Tool::PowerPlantCoal,
        Tool::PowerPlantGas,
        Tool::BusDepot,
        Tool::RailDepot,
        Tool::SubwayStation,
        Tool::WaterPump,
        Tool::WaterTower,
        Tool::WaterTreatment,
        Tool::Desalination,
        Tool::Park,
        Tool::Police,
        Tool::Fire,
        Tool::Bulldoze,
    ];

    pub fn cost(&self) -> i64 {
        match self {
            Tool::Inspect => 0,
            Tool::ZoneResLight | Tool::ZoneCommLight | Tool::ZoneIndLight => 5,
            Tool::ZoneResDense | Tool::ZoneCommDense | Tool::ZoneIndDense => 10,
            Tool::Road => 10,
            Tool::Highway => 40,
            Tool::Onramp => 25,
            Tool::Rail => 20,
            Tool::PowerLine => 5,
            Tool::WaterPipe => 5,
            Tool::Subway => 50,
            Tool::PowerPlantCoal => 3_000,
            Tool::PowerPlantGas => 6_000,
            Tool::BusDepot => 250,
            Tool::RailDepot => 500,
            Tool::SubwayStation => 500,
            Tool::WaterPump => 200,
            Tool::WaterTower => 350,
            Tool::WaterTreatment => 750,
            Tool::Desalination => 1_200,
            Tool::Park => 80,
            Tool::Police => 500,
            Tool::Fire => 500,
            Tool::Bulldoze => 1,
        }
    }

    pub fn target_tile(&self) -> Option<Tile> {
        match self {
            Tool::Inspect => None,
            Tool::ZoneResLight | Tool::ZoneResDense => Some(Tile::ZoneRes),
            Tool::ZoneCommLight | Tool::ZoneCommDense => Some(Tile::ZoneComm),
            Tool::ZoneIndLight | Tool::ZoneIndDense => Some(Tile::ZoneInd),
            Tool::Road => Some(Tile::Road),
            Tool::Highway => Some(Tile::Highway),
            Tool::Onramp => Some(Tile::Onramp),
            Tool::Rail => Some(Tile::Rail),
            Tool::PowerLine => Some(Tile::PowerLine),
            Tool::WaterPipe => Some(Tile::WaterPipe),
            Tool::Subway => Some(Tile::SubwayTunnel),
            Tool::PowerPlantCoal => Some(Tile::PowerPlantCoal),
            Tool::PowerPlantGas => Some(Tile::PowerPlantGas),
            Tool::BusDepot => Some(Tile::BusDepot),
            Tool::RailDepot => Some(Tile::RailDepot),
            Tool::SubwayStation => Some(Tile::SubwayStation),
            Tool::WaterPump => Some(Tile::WaterPump),
            Tool::WaterTower => Some(Tile::WaterTower),
            Tool::WaterTreatment => Some(Tile::WaterTreatment),
            Tool::Desalination => Some(Tile::Desalination),
            Tool::Park => Some(Tile::Park),
            Tool::Police => Some(Tile::Police),
            Tool::Fire => Some(Tile::Fire),
            Tool::Bulldoze => Some(Tile::Grass),
        }
    }

    pub fn zone_spec(self) -> Option<ZoneSpec> {
        match self {
            Tool::ZoneResLight => Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Light,
            }),
            Tool::ZoneResDense => Some(ZoneSpec {
                kind: ZoneKind::Residential,
                density: ZoneDensity::Dense,
            }),
            Tool::ZoneCommLight => Some(ZoneSpec {
                kind: ZoneKind::Commercial,
                density: ZoneDensity::Light,
            }),
            Tool::ZoneCommDense => Some(ZoneSpec {
                kind: ZoneKind::Commercial,
                density: ZoneDensity::Dense,
            }),
            Tool::ZoneIndLight => Some(ZoneSpec {
                kind: ZoneKind::Industrial,
                density: ZoneDensity::Light,
            }),
            Tool::ZoneIndDense => Some(ZoneSpec {
                kind: ZoneKind::Industrial,
                density: ZoneDensity::Dense,
            }),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tool::Inspect => "Inspect",
            Tool::ZoneResLight => "Res Light",
            Tool::ZoneResDense => "Res Dense",
            Tool::ZoneCommLight => "Comm Light",
            Tool::ZoneCommDense => "Comm Dense",
            Tool::ZoneIndLight => "Ind Light",
            Tool::ZoneIndDense => "Ind Dense",
            Tool::Road => "Road",
            Tool::Highway => "Highway",
            Tool::Onramp => "Onramp",
            Tool::Rail => "Rail",
            Tool::PowerLine => "Power Line",
            Tool::WaterPipe => "Water Pipe",
            Tool::Subway => "Subway",
            Tool::PowerPlantCoal => "Coal Plant",
            Tool::PowerPlantGas => "Gas Plant",
            Tool::BusDepot => "Bus Depot",
            Tool::RailDepot => "Rail Depot",
            Tool::SubwayStation => "Subway Sta",
            Tool::WaterPump => "Water Pump",
            Tool::WaterTower => "Water Tower",
            Tool::WaterTreatment => "Treatment",
            Tool::Desalination => "Desal",
            Tool::Park => "Park",
            Tool::Police => "Police",
            Tool::Fire => "Fire Dept",
            Tool::Bulldoze => "Bulldoze",
        }
    }

    pub fn key_hint(&self) -> char {
        match self {
            Tool::Inspect => '?',
            Tool::ZoneResLight => '1',
            Tool::ZoneResDense => '2',
            Tool::ZoneCommLight => '3',
            Tool::ZoneCommDense => '4',
            Tool::ZoneIndLight => '5',
            Tool::ZoneIndDense => '6',
            Tool::Road => 'r',
            Tool::Highway => 'h',
            Tool::Onramp => 'o',
            Tool::Rail => 'l',
            Tool::PowerLine => 'p',
            Tool::WaterPipe => 'w',
            Tool::Subway => 'm',
            Tool::PowerPlantCoal => 'e',
            Tool::PowerPlantGas => 'g',
            Tool::BusDepot => 'd',
            Tool::RailDepot => 't',
            Tool::SubwayStation => 'n',
            Tool::WaterPump => 'u',
            Tool::WaterTower => 'i',
            Tool::WaterTreatment => 'y',
            Tool::Desalination => 'a',
            Tool::Park => 'k',
            Tool::Police => 's',
            Tool::Fire => 'f',
            Tool::Bulldoze => 'b',
        }
    }

    pub fn unlock_year(self) -> i32 {
        match self {
            Tool::Subway | Tool::SubwayStation => 1910,
            Tool::BusDepot => 1920,
            Tool::Highway | Tool::Onramp => 1930,
            _ => 0,
        }
    }

    pub fn is_unlocked(self, year: i32, mode: UnlockMode) -> bool {
        mode == UnlockMode::Sandbox || year >= self.unlock_year()
    }

    pub fn uses_underground_layer(self) -> bool {
        matches!(self, Tool::WaterPipe | Tool::Subway)
    }

    pub fn can_place(&self, tile: Tile) -> bool {
        match self {
            Tool::Bulldoze => !matches!(tile, Tile::Water),
            Tool::Inspect => false,
            Tool::Road | Tool::Highway | Tool::Onramp => matches!(
                tile,
                Tile::Grass
                    | Tile::Trees
                    | Tile::Dirt
                    | Tile::PowerLine
                    | Tile::ZoneRes
                    | Tile::ZoneComm
                    | Tile::ZoneInd
                    | Tile::Rubble
                    | Tile::Road
                    | Tile::RoadPowerLine
                    | Tile::Highway
                    | Tile::Onramp
            ),
            Tool::Rail => matches!(
                tile,
                Tile::Grass
                    | Tile::Trees
                    | Tile::Dirt
                    | Tile::ZoneRes
                    | Tile::ZoneComm
                    | Tile::ZoneInd
                    | Tile::Rubble
                    | Tile::Rail
            ),
            Tool::PowerLine => matches!(
                tile,
                Tile::Grass
                    | Tile::Trees
                    | Tile::Dirt
                    | Tile::Road
                    | Tile::ZoneRes
                    | Tile::ZoneComm
                    | Tile::ZoneInd
                    | Tile::Rubble
                    | Tile::PowerLine
                    | Tile::RoadPowerLine
            ),
            Tool::WaterPipe | Tool::Subway => !matches!(tile, Tile::Water),
            Tool::ZoneResLight
            | Tool::ZoneResDense
            | Tool::ZoneCommLight
            | Tool::ZoneCommDense
            | Tool::ZoneIndLight
            | Tool::ZoneIndDense => matches!(
                tile,
                Tile::Grass
                    | Tile::Trees
                    | Tile::Dirt
                    | Tile::Road
                    | Tile::PowerLine
                    | Tile::RoadPowerLine
                    | Tile::Highway
                    | Tile::Onramp
                    | Tile::ZoneRes
                    | Tile::ZoneComm
                    | Tile::ZoneInd
                    | Tile::Rubble
            ),
            // Surface occupants can consume bare power lines, but transport tiles still need to
            // be cleared explicitly by the player.
            Tool::PowerPlantCoal
            | Tool::PowerPlantGas
            | Tool::BusDepot
            | Tool::RailDepot
            | Tool::SubwayStation
            | Tool::WaterPump
            | Tool::WaterTower
            | Tool::WaterTreatment
            | Tool::Desalination
            | Tool::Park
            | Tool::Police
            | Tool::Fire => matches!(
                tile,
                Tile::Grass | Tile::Trees | Tile::Dirt | Tile::Rubble | Tile::PowerLine
            ),
        }
    }

    pub fn is_traversable(self, tile: Tile) -> bool {
        match self {
            Tool::Road => matches!(tile, Tile::Road | Tile::RoadPowerLine | Tile::Onramp),
            Tool::Highway => matches!(tile, Tile::Highway | Tile::Onramp),
            Tool::Onramp => matches!(tile, Tile::Onramp | Tile::Road | Tile::RoadPowerLine),
            Tool::Rail => tile == Tile::Rail,
            Tool::PowerLine => matches!(tile, Tile::PowerLine | Tile::RoadPowerLine),
            Tool::WaterPipe => tile == Tile::WaterPipe,
            Tool::Subway => tile == Tile::SubwayTunnel || tile == Tile::SubwayStation,
            _ => self.target_tile() == Some(tile),
        }
    }

    pub fn uses_line_drag(tool: Tool) -> bool {
        matches!(
            tool,
            Tool::Road
                | Tool::Highway
                | Tool::Onramp
                | Tool::Rail
                | Tool::PowerLine
                | Tool::WaterPipe
                | Tool::Subway
        )
    }

    pub fn uses_rect_drag(tool: Tool) -> bool {
        matches!(
            tool,
            Tool::ZoneResLight
                | Tool::ZoneResDense
                | Tool::ZoneCommLight
                | Tool::ZoneCommDense
                | Tool::ZoneIndLight
                | Tool::ZoneIndDense
        )
    }

    pub fn footprint(&self) -> (usize, usize) {
        match self {
            Tool::PowerPlantCoal | Tool::PowerPlantGas => (4, 4),
            Tool::Police | Tool::Fire => (3, 3),
            Tool::WaterTreatment | Tool::Desalination => (3, 3),
            Tool::WaterTower | Tool::Park | Tool::BusDepot | Tool::RailDepot => (2, 2),
            _ => (1, 1),
        }
    }

    pub fn uses_footprint_preview(tool: Tool) -> bool {
        let (w, h) = tool.footprint();
        w > 1 || h > 1
    }
}
