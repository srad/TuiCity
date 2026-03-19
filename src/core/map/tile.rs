#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum TerrainTile {
    #[default]
    Grass,
    Water,
    Trees,
    Dirt,
}

impl TerrainTile {
    pub fn to_tile(self) -> Tile {
        match self {
            TerrainTile::Grass => Tile::Grass,
            TerrainTile::Water => Tile::Water,
            TerrainTile::Trees => Tile::Trees,
            TerrainTile::Dirt => Tile::Dirt,
        }
    }

    pub fn from_tile(tile: Tile) -> Option<Self> {
        match tile {
            Tile::Grass => Some(TerrainTile::Grass),
            Tile::Water => Some(TerrainTile::Water),
            Tile::Trees => Some(TerrainTile::Trees),
            Tile::Dirt => Some(TerrainTile::Dirt),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum TransportTile {
    Road,
    Rail,
    Highway,
    Onramp,
}

impl TransportTile {
    pub fn to_tile(self, has_power_line: bool) -> Tile {
        match self {
            TransportTile::Road if has_power_line => Tile::RoadPowerLine,
            TransportTile::Road => Tile::Road,
            TransportTile::Rail => Tile::Rail,
            TransportTile::Highway => Tile::Highway,
            TransportTile::Onramp => Tile::Onramp,
        }
    }

    pub fn is_surface_road(self) -> bool {
        matches!(self, TransportTile::Road | TransportTile::Onramp)
    }

    pub fn is_drive_network(self) -> bool {
        matches!(
            self,
            TransportTile::Road | TransportTile::Highway | TransportTile::Onramp
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct UndergroundTile {
    #[serde(default)]
    pub water_pipe: bool,
    #[serde(default)]
    pub subway: bool,
}

impl UndergroundTile {
    pub fn is_empty(self) -> bool {
        !self.water_pipe && !self.subway
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ZoneKind {
    Residential,
    Commercial,
    Industrial,
}

impl ZoneKind {
    pub fn label(self) -> &'static str {
        match self {
            ZoneKind::Residential => "Residential",
            ZoneKind::Commercial => "Commercial",
            ZoneKind::Industrial => "Industrial",
        }
    }

    pub fn empty_tile(self) -> Tile {
        match self {
            ZoneKind::Residential => Tile::ZoneRes,
            ZoneKind::Commercial => Tile::ZoneComm,
            ZoneKind::Industrial => Tile::ZoneInd,
        }
    }

    pub fn from_tile(tile: Tile) -> Option<Self> {
        match tile {
            Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh => {
                Some(ZoneKind::Residential)
            }
            Tile::ZoneComm | Tile::CommLow | Tile::CommHigh => Some(ZoneKind::Commercial),
            Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy => Some(ZoneKind::Industrial),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ZoneDensity {
    Light,
    Dense,
}

impl ZoneDensity {
    pub fn label(self) -> &'static str {
        match self {
            ZoneDensity::Light => "Light",
            ZoneDensity::Dense => "Dense",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ZoneSpec {
    pub kind: ZoneKind,
    pub density: ZoneDensity,
}

impl ZoneSpec {
    pub fn empty_tile(self) -> Tile {
        self.kind.empty_tile()
    }

    pub fn label(self) -> String {
        format!("{} {}", self.density.label(), self.kind.label())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum TripMode {
    Road,
    Bus,
    Rail,
    Subway,
}

impl TripMode {
    pub fn label(self) -> &'static str {
        match self {
            TripMode::Road => "Road",
            TripMode::Bus => "Bus",
            TripMode::Rail => "Rail",
            TripMode::Subway => "Subway",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum TripFailure {
    NoLocalAccess,
    NoDestination,
    NoRoute,
    TooLong,
}

impl TripFailure {
    pub fn label(self) -> &'static str {
        match self {
            TripFailure::NoLocalAccess => "No Local Access",
            TripFailure::NoDestination => "No Destination",
            TripFailure::NoRoute => "No Route",
            TripFailure::TooLong => "Too Long",
        }
    }
}

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
    Highway = 14,
    Onramp = 15,
    WaterPipe = 16,
    SubwayTunnel = 17,
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
    BusDepot = 66,
    RailDepot = 67,
    SubwayStation = 68,
    WaterPump = 69,
    WaterTower = 70,
    WaterTreatment = 71,
    Desalination = 72,
    Rubble = 80,
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
            14 => Tile::Highway,
            15 => Tile::Onramp,
            16 => Tile::WaterPipe,
            17 => Tile::SubwayTunnel,
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
            66 => Tile::BusDepot,
            67 => Tile::RailDepot,
            68 => Tile::SubwayStation,
            69 => Tile::WaterPump,
            70 => Tile::WaterTower,
            71 => Tile::WaterTreatment,
            72 => Tile::Desalination,
            80 => Tile::Rubble,
            _ => Tile::Grass,
        }
    }

    pub fn is_zone(self) -> bool {
        matches!(self, Tile::ZoneRes | Tile::ZoneComm | Tile::ZoneInd)
    }

    pub fn is_building(self) -> bool {
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

    pub fn is_service_building(self) -> bool {
        matches!(
            self,
            Tile::Police
                | Tile::Fire
                | Tile::Hospital
                | Tile::Park
                | Tile::BusDepot
                | Tile::RailDepot
                | Tile::SubwayStation
                | Tile::WaterPump
                | Tile::WaterTower
                | Tile::WaterTreatment
                | Tile::Desalination
        )
    }

    pub fn is_transport(self) -> bool {
        matches!(
            self,
            Tile::Road | Tile::Rail | Tile::RoadPowerLine | Tile::Highway | Tile::Onramp
        )
    }

    pub fn is_drive_network(self) -> bool {
        matches!(
            self,
            Tile::Road | Tile::RoadPowerLine | Tile::Highway | Tile::Onramp
        )
    }

    pub fn receives_power(self) -> bool {
        self.is_building()
            || self.is_zone()
            || matches!(
                self,
                Tile::Police
                    | Tile::Fire
                    | Tile::Hospital
                    | Tile::BusDepot
                    | Tile::RailDepot
                    | Tile::SubwayStation
                    | Tile::WaterPump
                    | Tile::WaterTower
                    | Tile::WaterTreatment
                    | Tile::Desalination
            )
    }

    pub fn is_conductive_structure(self) -> bool {
        self.is_building()
            || matches!(
                self,
                Tile::Police
                    | Tile::Fire
                    | Tile::Hospital
                    | Tile::BusDepot
                    | Tile::RailDepot
                    | Tile::SubwayStation
                    | Tile::WaterPump
                    | Tile::WaterTower
                    | Tile::WaterTreatment
                    | Tile::Desalination
            )
    }

    pub fn road_connects(self) -> bool {
        matches!(self, Tile::Road | Tile::RoadPowerLine | Tile::Onramp)
    }

    pub fn vehicle_connects(self) -> bool {
        matches!(
            self,
            Tile::Road | Tile::RoadPowerLine | Tile::Highway | Tile::Onramp
        )
    }

    pub fn rail_connects(self) -> bool {
        self == Tile::Rail
    }

    pub fn subway_connects(self) -> bool {
        self == Tile::SubwayTunnel
    }

    pub fn water_connects(self) -> bool {
        self == Tile::WaterPipe
    }

    pub fn power_connects(self) -> bool {
        matches!(self, Tile::PowerLine | Tile::RoadPowerLine)
    }

    pub fn name(self) -> &'static str {
        match self {
            Tile::Grass => "Grass",
            Tile::Water => "Water",
            Tile::Trees => "Trees",
            Tile::Dirt => "Dirt",
            Tile::Road => "Road",
            Tile::Rail => "Rail",
            Tile::PowerLine => "Power Line",
            Tile::RoadPowerLine => "Road + Power Line",
            Tile::Highway => "Highway",
            Tile::Onramp => "Onramp",
            Tile::WaterPipe => "Water Pipe",
            Tile::SubwayTunnel => "Subway Tunnel",
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
            Tile::BusDepot => "Bus Depot",
            Tile::RailDepot => "Rail Depot",
            Tile::SubwayStation => "Subway Station",
            Tile::WaterPump => "Water Pump",
            Tile::WaterTower => "Water Tower",
            Tile::WaterTreatment => "Water Treatment",
            Tile::Desalination => "Desalination Plant",
            Tile::Rubble => "Rubble",
        }
    }

    pub fn inferred_zone_density(self) -> Option<ZoneDensity> {
        match self {
            Tile::ZoneRes
            | Tile::ZoneComm
            | Tile::ZoneInd
            | Tile::ResLow
            | Tile::CommLow
            | Tile::IndLight => Some(ZoneDensity::Light),
            Tile::ResMed | Tile::ResHigh | Tile::CommHigh | Tile::IndHeavy => {
                Some(ZoneDensity::Dense)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TileOverlay {
    /// Power level delivered to this tile. 0 = none, 255 = maximum strength.
    pub power_level: u8,
    pub on_fire: bool,
    pub crime: u8,
    #[serde(default)]
    pub zone: Option<ZoneKind>,
    /// 0 = clean air, 255 = heavily polluted (computed each tick)
    #[serde(default)]
    pub pollution: u8,
    /// 0 = lowest value, 255 = prime real estate (computed each tick)
    #[serde(default)]
    pub land_value: u8,
    /// 0 = safe, 255 = extreme risk (computed each tick, reduced by fire stations)
    #[serde(default)]
    pub fire_risk: u8,
    /// 0 = empty roads, 255 = gridlock.
    #[serde(default)]
    pub traffic: u8,
    /// 0 = dry, 255 = fully served by the water network.
    #[serde(default)]
    pub water_service: u8,
    /// Whether the lot's last monthly transport simulation found a usable trip.
    #[serde(default)]
    pub trip_success: bool,
    /// Dominant successful mode from the last monthly simulation for this lot.
    #[serde(default)]
    pub trip_mode: Option<TripMode>,
    /// Best failure reason seen across the lot's required trips when no trip succeeded.
    #[serde(default)]
    pub trip_failure: Option<TripFailure>,
    /// Last successful trip cost, normalized to the simulation's cost scale.
    #[serde(default)]
    pub trip_cost: u8,
}

impl TileOverlay {
    pub fn is_powered(&self) -> bool {
        self.power_level > 0
    }

    pub fn has_water(&self) -> bool {
        self.water_service > 0
    }
}
