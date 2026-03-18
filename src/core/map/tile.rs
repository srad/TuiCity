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
    PowerPlantCoal = 60,
    PowerPlantGas = 61,
    Park = 62,
    Police = 63,
    Fire = 64,
    Hospital = 65,
    Rubble = 70,
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
            60 => Tile::PowerPlantCoal,
            61 => Tile::PowerPlantGas,
            62 => Tile::Park,
            63 => Tile::Police,
            64 => Tile::Fire,
            65 => Tile::Hospital,
            70 => Tile::Rubble,
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
        matches!(self, Tile::Road | Tile::Rail | Tile::RoadPowerLine)
    }

    pub fn receives_power(&self) -> bool {
        self.is_building()
            || self.is_zone()
            || matches!(
                self,
                Tile::Police | Tile::Fire | Tile::Hospital
            )
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
            Tile::PowerPlantCoal => "Coal Power Plant",
            Tile::PowerPlantGas => "Gas Power Plant",
            Tile::Park => "Park",
            Tile::Police => "Police Dept",
            Tile::Fire => "Fire Dept",
            Tile::Hospital => "Hospital",
            Tile::Rubble => "Rubble",
        }
    }
}

#[derive(Clone, Copy, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct TileOverlay {
    /// Power level delivered to this tile. 0 = none, 255 = maximum strength.
    pub power_level: u8,
    pub on_fire: bool,
    pub crime: u8,
    /// 0 = clean air, 255 = heavily polluted (computed each tick)
    #[serde(default)]
    pub pollution: u8,
    /// 0 = lowest value, 255 = prime real estate (computed each tick)
    #[serde(default)]
    pub land_value: u8,
    /// 0 = safe, 255 = extreme risk (computed each tick, reduced by fire stations)
    #[serde(default)]
    pub fire_risk: u8,
}

impl TileOverlay {
    pub fn is_powered(&self) -> bool {
        self.power_level > 0
    }
}
