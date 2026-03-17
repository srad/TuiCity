#![allow(dead_code)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Tile {
    #[default]
    Grass = 0,
    Water = 1,
    Trees = 2,
    Dirt = 3,
    Road = 10,
    Rail = 11,
    PowerLine = 12,
    RoadPowerLine = 13,
    ZoneRes = 20,
    ZoneComm = 21,
    ZoneInd = 22,
    ResLow = 30,
    ResMed = 31,
    ResHigh = 32,
    CommLow = 40,
    CommHigh = 41,
    IndLight = 50,
    IndHeavy = 51,
    PowerPlant = 60,
    Park = 61,
    Police = 62,
    Fire = 63,
    Hospital = 64,
}

impl Tile {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Tile::Grass,
            1 => Tile::Water,
            2 => Tile::Trees,
            3 => Tile::Dirt,
            10 => Tile::Road,
            11 => Tile::Rail,
            12 => Tile::PowerLine,
            13 => Tile::RoadPowerLine,
            20 => Tile::ZoneRes,
            21 => Tile::ZoneComm,
            22 => Tile::ZoneInd,
            30 => Tile::ResLow,
            31 => Tile::ResMed,
            32 => Tile::ResHigh,
            40 => Tile::CommLow,
            41 => Tile::CommHigh,
            50 => Tile::IndLight,
            51 => Tile::IndHeavy,
            60 => Tile::PowerPlant,
            61 => Tile::Park,
            62 => Tile::Police,
            63 => Tile::Fire,
            64 => Tile::Hospital,
            _ => Tile::Grass,
        }
    }

    pub fn is_zone(&self) -> bool {
        matches!(self, Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd)
    }

    pub fn is_building(&self) -> bool {
        matches!(
            self,
            Tile::ResLow
                | Tile::ResMed
                | Tile::ResHigh
                | Tile::CommLow
                | Tile::CommHigh
                | Tile::IndLight
                | Tile::IndHeavy
        )
    }

    pub fn is_road(&self) -> bool {
        matches!(self, Tile::Road | Tile::Rail)
    }

    pub fn road_connects(self) -> bool {
        matches!(self, Tile::Road | Tile::RoadPowerLine)
    }

    pub fn power_connects(self) -> bool {
        matches!(self, Tile::PowerLine | Tile::RoadPowerLine)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tile::Grass => "Grass",
            Tile::Water => "Water",
            Tile::Trees => "Trees",
            Tile::Dirt => "Dirt",
            Tile::Road => "Road",
            Tile::Rail => "Rail",
            Tile::PowerLine => "Power Line",
            Tile::RoadPowerLine => "Road + Power Line",
            Tile::ZoneRes => "Residential Zone",
            Tile::ZoneComm => "Commercial Zone",
            Tile::ZoneInd => "Industrial Zone",
            Tile::ResLow => "Res Low-Density",
            Tile::ResMed => "Res Mid-Density",
            Tile::ResHigh => "Res High-Density",
            Tile::CommLow => "Comm Low-Density",
            Tile::CommHigh => "Comm High-Density",
            Tile::IndLight => "Light Industry",
            Tile::IndHeavy => "Heavy Industry",
            Tile::PowerPlant => "Power Plant",
            Tile::Park => "Park",
            Tile::Police => "Police Dept",
            Tile::Fire => "Fire Dept",
            Tile::Hospital => "Hospital",
        }
    }
}

#[derive(Clone, Copy, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct TileOverlay {
    pub powered: bool,
    pub on_fire: bool,
    pub crime: u8,
}
